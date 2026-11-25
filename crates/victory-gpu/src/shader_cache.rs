//! Translated shaders kept on disk (decision 0003): under the configured
//! cache folder, keyed by the container's content and by the VIctory build
//! that translated it (the BLAKE3 of our executable), so a rebuilt
//! translator never serves an old translation. Entries go through
//! `victory-pipeline`'s content-addressed cache, which writes them
//! atomically; an entry that does not read back is translated again.
//!
//! The entry is our own encoding of [`HostShader`]: a field added to it
//! must be added to `encode` and `decode` too (entries of older builds
//! are ignored anyway).
//!
//! Each build's translations are in a folder of their own
//! (`builds/<build>`), so that what no build reads any more can be removed:
//! the folders of this build and of the build used last before it stay (a
//! normal and a diagnostics build, used in turn, each keep theirs), the
//! others go ([`prune`]). One build's translations of a whole title are
//! 1.3 GB (25,466 shaders, measured); kept in one folder for every build,
//! as before, nothing told an old build's entries from the current one's
//! and each new build added its own for good.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use victory_pipeline::{Cache, Converter};
use victory_shader::emit::{HostShader, StaticSampler, UserData};
use victory_shader::resource::RootIndexedView;
use victory_shader::{HwStage, RootType};

static DIR: OnceLock<PathBuf> = OnceLock::new();
static CACHE: OnceLock<Option<Cache>> = OnceLock::new();
static HITS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);

/// Builds whose translations stay: this one and the one used last before.
const BUILDS_KEPT: usize = 2;

/// Keeps translated shaders under `dir` from now on.
pub fn use_shader_cache(dir: PathBuf) {
    let _ = DIR.set(dir);
}

/// This build's cache, opened at the first translation (the build's
/// fingerprint needs the host's view limit), with a thread started that
/// removes what no build reads any more.
fn cache() -> Option<&'static Cache> {
    CACHE
        .get_or_init(|| {
            let dir = DIR.get()?;
            let name = build_name(build());
            match open_for(dir, &name) {
                Ok(cache) => {
                    let dir = dir.clone();
                    let spawned = std::thread::Builder::new().name("victory-shader-cache".into()).spawn(move || {
                        let removed = prune(&dir, &name);
                        if removed > 0 {
                            tracing::info!(removed, "shader cache: folders of translations no build reads any more removed");
                        }
                    });
                    if let Err(e) = spawned {
                        tracing::warn!(error = %e, "old shader cache entries not removed");
                    }
                    Some(cache)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "no shader cache");
                    None
                }
            }
        })
        .as_ref()
}

/// The folder name of a build's translations.
fn build_name(build: &[u8; 32]) -> String {
    build[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// Opens the folder of the build `name` under `dir` and marks it used now.
fn open_for(dir: &Path, name: &str) -> anyhow::Result<Cache> {
    let own = dir.join("builds").join(name);
    let cache = Cache::open(&own)?;
    // Written anew at each use: its time says when the build last ran.
    std::fs::write(own.join("used"), b"")?;
    Ok(cache)
}

/// Removes, under the shader cache folder `dir`, what no build reads any
/// more: the entries kept in `dir` itself before builds had folders (the
/// folders named by two hexadecimal digits), and the folders of builds
/// other than `own` except the [`BUILDS_KEPT`]` - 1` used last. Returns how
/// many folders went.
fn prune(dir: &Path, own: &str) -> usize {
    let folders = |of: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(of)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default()
    };
    let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut removed = 0;
    for folder in folders(dir) {
        let n = name(&folder);
        if n.len() == 2 && n.bytes().all(|b| b.is_ascii_hexdigit()) && std::fs::remove_dir_all(&folder).is_ok() {
            removed += 1;
        }
    }
    let used = |p: &Path| {
        std::fs::metadata(p.join("used"))
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let mut others: Vec<(std::time::SystemTime, PathBuf)> = folders(&dir.join("builds"))
        .into_iter()
        .filter(|p| name(p) != own)
        .map(|p| (used(&p), p))
        .collect();
    others.sort_by_key(|other| std::cmp::Reverse(other.0));
    for (_, folder) in others.into_iter().skip(BUILDS_KEPT - 1) {
        if std::fs::remove_dir_all(&folder).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Shaders found in the cache and translated since the start.
pub(crate) fn counts() -> (u64, u64) {
    (HITS.load(Ordering::Relaxed), MISSES.load(Ordering::Relaxed))
}

/// What identifies this build: the BLAKE3 of our executable file and of
/// the translation settings it runs with (the loop limit, which a
/// diagnostics build can change at run time).
fn build() -> &'static [u8; 32] {
    static BUILD: OnceLock<[u8; 32]> = OnceLock::new();
    BUILD.get_or_init(|| {
        let exe = std::env::current_exe().and_then(std::fs::read);
        let limit = victory_shader::emit::loop_limit();
        if limit != victory_shader::emit::DEFAULT_LOOP_LIMIT {
            tracing::warn!(
                limit,
                "VICTORY_SHADER_LOOP_LIMIT: translated loops stop sooner"
            );
        }
        match exe {
            Ok(mut bytes) => {
                bytes.extend_from_slice(&limit.to_le_bytes());
                // So is the view heap's null slot (it differs by host GPU):
                // a translation cached without the clamp must not be served.
                crate::descriptor::host::probe_view_limit();
                bytes.extend_from_slice(&victory_shader::emit::view_heap_limit().to_le_bytes());
                // Which group barriers wait is part of the translation too.
                bytes.push(victory_shader::emit::group_sync());
                // So are the switches that pick the form of a wave64 program:
                // without them here a run with `VICTORY_WAVE_WIDE=0` was
                // served the translations an earlier run of the same
                // executable had cached with the switch unset.
                for name in [
                    "VICTORY_WAVE_WIDE",
                    "VICTORY_WAVE_DEEP_SKIPS",
                    "VICTORY_WAVE_OWN_BITS",
                    "VICTORY_F16_NEAREST",
                ] {
                    bytes.extend_from_slice(name.as_bytes());
                    bytes.push(b'=');
                    if let Some(value) = std::env::var_os(name) {
                        bytes.extend_from_slice(value.as_encoded_bytes());
                    }
                    bytes.push(0);
                }
                // And which clean-up passes ran over the translated code.
                bytes.extend_from_slice(&victory_shader::emit::shader_opt().to_le_bytes());
                *blake3::hash(&bytes).as_bytes()
            }
            Err(e) => {
                // Without it no entry can be trusted: a fresh key per run.
                tracing::warn!(error = %e, "cannot read our executable; shader cache unused");
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                *blake3::hash(&now.as_nanos().to_le_bytes()).as_bytes()
            }
        }
    })
}

/// The translation of a console shader container, from the cache when
/// there is one.
pub(crate) fn host_shader(container: &[u8]) -> Result<HostShader, String> {
    let translate = |c: &[u8]| victory_shader::emit::translate(c).map_err(|e| e.to_string());
    match cache() {
        Some(cache) => cached(cache, build(), container, translate),
        None => translate(container),
    }
}

/// The cache entry's converter: the build and the container in, the
/// encoded translation out. `ran` tells a miss from a hit.
struct Translate<F> {
    translate: F,
    ran: AtomicBool,
}

impl<F: Fn(&[u8]) -> Result<HostShader, String> + Send + Sync> Converter for Translate<F> {
    fn id(&self) -> &'static str {
        "shader.dxil"
    }
    fn version(&self) -> u32 {
        // The build fingerprint in the input does the versioning.
        1
    }
    fn convert(&self, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.ran.store(true, Ordering::Relaxed);
        let shader = (self.translate)(&input[32..]).map_err(anyhow::Error::msg)?;
        Ok(encode(&shader))
    }
}

fn cached(
    cache: &Cache,
    build: &[u8; 32],
    container: &[u8],
    translate: impl Fn(&[u8]) -> Result<HostShader, String> + Send + Sync,
) -> Result<HostShader, String> {
    let input = [&build[..], container].concat();
    let converter = Translate {
        translate,
        ran: AtomicBool::new(false),
    };
    for _ in 0..2 {
        let path = cache
            .get_or_convert(&converter, &input)
            .map_err(|e| format!("{e:#}"))?;
        if let Some(shader) = std::fs::read(&path).ok().and_then(|b| decode(&b)) {
            let (counter, what) = if converter.ran.load(Ordering::Relaxed) {
                (&MISSES, "translated and cached")
            } else {
                (&HITS, "from the cache")
            };
            counter.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(path = %path.display(), "shader {what}");
            return Ok(shader);
        }
        tracing::debug!(path = %path.display(), "unreadable shader cache entry; translating again");
        let _ = std::fs::remove_file(&path);
    }
    MISSES.fetch_add(1, Ordering::Relaxed);
    (converter.translate)(container)
}

const MAGIC: &[u8; 4] = b"VSH1";
const STAGES: [HwStage; 8] = [
    HwStage::Ls,
    HwStage::Hs,
    HwStage::Es,
    HwStage::Gs,
    HwStage::Vs,
    HwStage::Ps,
    HwStage::Ms,
    HwStage::Cs,
];

fn root_code(r: RootType) -> u32 {
    match r {
        RootType::Constants => 0,
        RootType::Descriptor => 1,
        RootType::Table => 2,
        RootType::ImmediateSampler => 3,
        RootType::ResourceHeap => 4,
        RootType::SamplerHeap => 5,
        RootType::Other(x) => 6 + x,
    }
}

fn root_type(code: u32) -> RootType {
    match code {
        0 => RootType::Constants,
        1 => RootType::Descriptor,
        2 => RootType::Table,
        3 => RootType::ImmediateSampler,
        4 => RootType::ResourceHeap,
        5 => RootType::SamplerHeap,
        x => RootType::Other(x - 6),
    }
}

/// Little-endian words and length-prefixed lists.
fn encode(s: &HostShader) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    let word = |out: &mut Vec<u8>, v: u32| out.extend(v.to_le_bytes());
    let stage = STAGES.iter().position(|&x| x == s.stage).unwrap_or(0) as u32;
    word(&mut out, stage);
    word(&mut out, s.dxil.len() as u32);
    out.extend(&s.dxil);
    word(&mut out, s.user_data.len() as u32);
    for u in &s.user_data {
        for v in [u.index, u32::from(u.slot), root_code(u.root), u.dword] {
            word(&mut out, v);
        }
    }
    word(&mut out, s.samplers.len() as u32);
    for sampler in &s.samplers {
        for &v in sampler {
            word(&mut out, v);
        }
    }
    word(&mut out, s.static_samplers.len() as u32);
    for t in &s.static_samplers {
        let [u, v, w] = t.address;
        let words = [
            t.filter,
            u,
            v,
            w,
            t.mip_lod_bias.to_bits(),
            t.max_anisotropy,
            t.comparison,
            t.border,
            t.min_lod.to_bits(),
            t.max_lod.to_bits(),
        ];
        for v in words {
            word(&mut out, v);
        }
    }
    for slots in [&s.root_cbvs, &s.root_srvs, &s.root_uavs] {
        word(&mut out, slots.len() as u32);
        out.extend(slots);
    }
    word(&mut out, s.root_indexed_views.len() as u32);
    for view in &s.root_indexed_views {
        for value in [
            u32::from(view.table_slot),
            u32::from(view.constant_slot),
            view.constant_dword,
            view.byte_offset,
            u32::from(view.shift),
        ] {
            word(&mut out, value);
        }
    }
    out
}

fn decode(bytes: &[u8]) -> Option<HostShader> {
    struct Reader<'a>(&'a [u8]);
    impl<'a> Reader<'a> {
        fn take(&mut self, n: usize) -> Option<&'a [u8]> {
            let (head, rest) = self.0.split_at_checked(n)?;
            self.0 = rest;
            Some(head)
        }
        fn word(&mut self) -> Option<u32> {
            Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
        }
        fn list(&mut self) -> Option<Vec<u8>> {
            let n = self.word()? as usize;
            Some(self.take(n)?.to_vec())
        }
    }
    let mut r = Reader(bytes);
    if r.take(4)? != MAGIC {
        return None;
    }
    let stage = *STAGES.get(r.word()? as usize)?;
    let dxil = r.list()?;
    let n = r.word()? as usize;
    let mut user_data = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        user_data.push(UserData {
            index: r.word()?,
            slot: u8::try_from(r.word()?).ok()?,
            root: root_type(r.word()?),
            dword: r.word()?,
        });
    }
    let n = r.word()? as usize;
    let mut samplers = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        samplers.push([r.word()?, r.word()?, r.word()?, r.word()?]);
    }
    let n = r.word()? as usize;
    let mut static_samplers = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        static_samplers.push(StaticSampler {
            filter: r.word()?,
            address: [r.word()?, r.word()?, r.word()?],
            mip_lod_bias: f32::from_bits(r.word()?),
            max_anisotropy: r.word()?,
            comparison: r.word()?,
            border: r.word()?,
            min_lod: f32::from_bits(r.word()?),
            max_lod: f32::from_bits(r.word()?),
        });
    }
    let root_cbvs = r.list()?;
    let root_srvs = r.list()?;
    let root_uavs = r.list()?;
    let n = r.word()? as usize;
    let mut root_indexed_views = Vec::with_capacity(n.min(256));
    for _ in 0..n {
        root_indexed_views.push(RootIndexedView {
            table_slot: u8::try_from(r.word()?).ok()?,
            constant_slot: u8::try_from(r.word()?).ok()?,
            constant_dword: r.word()?,
            byte_offset: r.word()?,
            shift: u8::try_from(r.word()?).ok().filter(|&s| s < 32)?,
        });
    }
    r.0.is_empty().then_some(HostShader {
        stage,
        dxil,
        user_data,
        samplers,
        static_samplers,
        root_cbvs,
        root_srvs,
        root_uavs,
        root_indexed_views,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shader() -> HostShader {
        HostShader {
            stage: HwStage::Ps,
            dxil: b"DXBC and the rest".to_vec(),
            user_data: vec![UserData {
                index: 8,
                slot: 1,
                root: RootType::Table,
                dword: 0,
            }],
            samplers: vec![[0x400, 0xc0_0000, 0x18f0_0000, 0]],
            static_samplers: vec![StaticSampler::from_words([
                0x400,
                0xc0_0000,
                0x18f0_0000,
                0,
            ])],
            root_cbvs: vec![2],
            root_srvs: vec![0, 3],
            root_uavs: vec![1],
            root_indexed_views: vec![RootIndexedView {
                table_slot: 1,
                constant_slot: 2,
                constant_dword: 3,
                byte_offset: 32,
                shift: 5,
            }],
        }
    }

    /// A build's translations are in its own folder; the entries of the
    /// layout before, and the folders of all but the build used last
    /// before this one, are removed.
    #[test]
    fn old_builds_translations_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Entries as they were kept before builds had folders.
        for old in ["0a", "ff"] {
            std::fs::create_dir_all(root.join(old)).unwrap();
            std::fs::write(root.join(old).join("entry"), b"old").unwrap();
        }
        // Something else in the folder stays.
        std::fs::create_dir_all(root.join("notes")).unwrap();
        // Three earlier builds, used one, two and three hours ago.
        let hour = std::time::Duration::from_secs(3600);
        for (name, hours) in [("build-one", 1u32), ("build-two", 2), ("build-three", 3)] {
            let folder = root.join("builds").join(name);
            std::fs::create_dir_all(&folder).unwrap();
            let used = std::fs::File::create(folder.join("used")).unwrap();
            used.set_modified(std::time::SystemTime::now() - hour * hours).unwrap();
        }
        let own = build_name(&[7; 32]);
        assert_eq!(own, "0707070707070707");
        let cache = open_for(root, &own).unwrap();
        assert!(root.join("builds").join(&own).join("used").is_file());
        assert_eq!(prune(root, &own), 4);
        let left = |p: &str| root.join(p).exists();
        assert!(!left("0a") && !left("ff"), "entries of the old layout go");
        assert!(left("notes"));
        assert!(left("builds/build-one"), "the build used last before this one stays");
        assert!(!left("builds/build-two") && !left("builds/build-three"));
        // This build's cache works, and nothing more goes at the next look.
        let got = cached(&cache, &[7; 32], b"a console container", |_| Ok(shader())).unwrap();
        assert_eq!(got, shader());
        assert_eq!(prune(root, &own), 0);
    }

    #[test]
    fn translations_come_back_from_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(dir.path()).unwrap();
        let build = [7u8; 32];
        let container = b"a console container".as_slice();
        let runs = AtomicU64::new(0);
        let translate = |_: &[u8]| {
            runs.fetch_add(1, Ordering::Relaxed);
            Ok(shader())
        };
        assert_eq!(cached(&cache, &build, container, translate), Ok(shader()));
        assert_eq!(cached(&cache, &build, container, translate), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 1, "the second came from disk");

        // Another build translates again.
        assert_eq!(cached(&cache, &[8; 32], container, translate), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 2);

        // A damaged entry is a miss, translated and stored again.
        for e in walk(dir.path()) {
            std::fs::write(e, b"VSH1 broken").unwrap();
        }
        assert_eq!(cached(&cache, &build, container, translate), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 3);
        assert_eq!(cached(&cache, &build, container, translate), Ok(shader()));
        assert_eq!(runs.load(Ordering::Relaxed), 3);

        // Failures are not stored.
        let fail = |_: &[u8]| Err::<HostShader, _>("unsupported".to_owned());
        assert!(cached(&cache, &build, b"other", fail).is_err());
    }

    fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if e.path().is_dir() {
                files.extend(walk(&e.path()));
            } else {
                files.push(e.path());
            }
        }
        files
    }

    #[test]
    fn entries_round_trip_and_reject_damage() {
        let bytes = encode(&shader());
        assert_eq!(decode(&bytes), Some(shader()));
        assert_eq!(decode(&bytes[..bytes.len() - 1]), None);
        assert_eq!(decode(&[bytes.as_slice(), &[0]].concat()), None);
        assert_eq!(decode(b"VSH0"), None);
    }
}
