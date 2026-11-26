//! Pipeline states. Both descriptions start as on PC: the root signature,
//! then the shader stages (five for graphics, one for compute); streams
//! carry the rest of the graphics state as subobjects of the standard
//! types.
//!
//! Host pipelines are made on a pipeline's first draw, not at creation:
//! The title creates some 60,000 pipelines in its first 20
//! seconds and draws with a few dozen (decision 0007, step 3). Shader
//! containers are shared by content hash and translated once, on first
//! use (decision 0003).
//!
//! shortcut: only vertex (with or without a geometry stage) or mesh plus
//! pixel pipelines from streams and compute pipelines get host pipelines;
//! tessellation stages and the plain graphics description keep their
//! containers only. A console geometry stage is the second half of a merged
//! ES-GS program: its vertex stage's container carries the export half and
//! a vertex variant, which the host runs (`victory_shader::merged`). The 32-byte
//! packet after the header stays zero: titles only copy it inline, which
//! our null command-list limits keep from running.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::HashMap;
use std::mem::offset_of;
use std::sync::{Arc, Mutex, OnceLock};

use crate::object::{self, E_INVALIDARG, HRESULT, Iid, Object, PIPELINE, Table, This, guid};
use crate::resource::PAGEABLE_IIDS;

const PIPELINE_IIDS: [Iid; 4] = [
    guid(
        0x765a30f3,
        0xf624,
        0x4c6f,
        [0xa8, 0x28, 0xac, 0xe9, 0x48, 0x62, 0x24, 0x45],
    ),
    PAGEABLE_IIDS[0],
    PAGEABLE_IIDS[1],
    PAGEABLE_IIDS[2],
];

#[repr(C)]
pub(crate) struct Bytecode {
    code: *const u8,
    size: usize,
}

#[repr(C)]
pub(crate) struct GraphicsHead {
    root: This,
    /// Vertex, pixel, domain, hull, geometry.
    stages: [Bytecode; 5],
}

#[repr(C)]
pub(crate) struct ComputeHead {
    root: This,
    stage: Bytecode,
}

/// A shader container a title passed, shared by every pipeline with the
/// same bytes.
pub(crate) struct Shader {
    pub(crate) stage: &'static str,
    /// The first 8 bytes of its BLAKE3, as logged.
    pub(crate) hash: String,
    container: Vec<u8>,
    #[cfg(windows)]
    translated: OnceLock<Option<Translated>>,
    /// A mesh program's other translation, as compute plus a vertex shader
    /// ([`Shader::mesh_variant`]).
    #[cfg(windows)]
    mesh_variant: OnceLock<Option<MeshVariant>>,
    /// A pixel shader's translation that reads its barycentrics from
    /// inputs ([`Shader::bary_variant`]).
    #[cfg(windows)]
    bary_variant: OnceLock<Option<Translated>>,
}

impl std::fmt::Debug for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.stage, self.hash)
    }
}

/// Every container seen, by full content hash.
static SHADERS: Mutex<Option<HashMap<[u8; 32], Arc<Shader>>>> = Mutex::new(None);

/// For the memory lines: containers kept and their bytes, translations
/// kept and the bytes of their host code. None of them is ever let go.
static KEPT: [std::sync::atomic::AtomicU64; 4] = [const { std::sync::atomic::AtomicU64::new(0) }; 4];

/// What the shader table and the pipeline library keep in memory:
/// containers and their bytes, translations and their bytes, the bytes of
/// the library's file (kept whole while the library lives) and what its
/// last save wrote.
#[cfg(windows)]
#[cfg_attr(not(feature = "diagnostics"), allow(dead_code))]
pub(crate) fn kept() -> [u64; 7] {
    use std::sync::atomic::Ordering::Relaxed;
    [
        KEPT[0].load(Relaxed),
        KEPT[1].load(Relaxed),
        KEPT[2].load(Relaxed),
        KEPT[3].load(Relaxed),
        library::BLOB.load(Relaxed),
        library::SAVED.load(Relaxed),
        library::TRIMMED_BYTES.load(Relaxed),
    ]
}

/// The shared container for `bytes`.
fn shader(stage: &'static str, bytes: &[u8]) -> Arc<Shader> {
    let full = blake3::hash(bytes);
    let mut shaders = SHADERS.lock().unwrap_or_else(|e| e.into_inner());
    shaders
        .get_or_insert_with(HashMap::new)
        .entry(*full.as_bytes())
        .or_insert_with(|| {
            KEPT[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            KEPT[1].fetch_add(bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
            Arc::new(Shader {
                stage,
                hash: full.to_hex()[..16].to_owned(),
                container: bytes.to_vec(),
                #[cfg(windows)]
                translated: OnceLock::new(),
                #[cfg(windows)]
                mesh_variant: OnceLock::new(),
                #[cfg(windows)]
                bary_variant: OnceLock::new(),
            })
        })
        .clone()
}

#[cfg(windows)]
pub(crate) use host::depth_bounds_test;

/// A translated shader and the console resources its code reads.
#[cfg(windows)]
pub(crate) struct Translated {
    pub(crate) host: victory_shader::emit::HostShader,
    pub(crate) used: Vec<victory_shader::UsedResource>,
    /// Whether the shader's code can store to buffer memory ([`stores`]);
    /// true when its code is not known.
    pub(crate) stores: bool,
    /// Whether the code can write anything through unordered access:
    /// [`stores`], or an image store or atomic (a texture shaders write).
    /// True when its code is not known. What `VICTORY_BARRIER_MERGE` asks
    /// of an op before it counts it as unordered writes to wait for.
    pub(crate) writes: bool,
}

#[cfg(windows)]
impl Shader {
    /// The translation, made on first use; a shader that does not
    /// translate is logged once.
    pub(crate) fn translated(&self) -> Option<&Translated> {
        self.translated
            .get_or_init(|| {
                let done = translate(&self.container);
                // `VICTORY_SHADER_SAVE=<dir>`: each shader as the title gave it (`.cdxil`) and as
                // translated (`.dxil`), by the hash crash reports name it with
                if let Some(dir) = std::env::var_os("VICTORY_SHADER_SAVE") {
                    let dir = std::path::PathBuf::from(dir);
                    let _ = std::fs::create_dir_all(&dir);
                    let stem = format!("{}.{}", self.hash, self.stage);
                    let _ = std::fs::write(dir.join(format!("{stem}.cdxil")), &self.container);
                    if let Ok(t) = &done {
                        let _ = std::fs::write(dir.join(format!("{stem}.dxil")), &t.host.dxil);
                    }
                }
                if let Ok(t) = &done {
                    KEPT[2].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    KEPT[3].fetch_add(t.host.dxil.len() as u64, std::sync::atomic::Ordering::Relaxed);
                }
                match &done {
                    Ok(t) => tracing::debug!(
                        stage = self.stage,
                        hash = %self.hash,
                        user_data = t.host.user_data.len(),
                        samplers = t.host.samplers.len(),
                        "shader translated"
                    ),
                    Err(e) => {
                        static FAILED: std::sync::atomic::AtomicU64 =
                            std::sync::atomic::AtomicU64::new(0);
                        if victory_core::logging::sample(&FAILED) {
                            tracing::warn!(
                                stage = self.stage,
                                hash = %self.hash,
                                error = %e,
                                "shader does not translate; its pipelines' draws are dropped"
                            );
                        }
                    }
                }
                done.ok()
            })
            .as_ref()
    }
}

/// A mesh program for a host without mesh shaders
/// (`victory_shader::emit::mesh_compute`): the compute shader that runs it
#[cfg(windows)]
impl Shader {
    /// (PROJECT ONE) This pixel shader translated to read its barycentric
    /// weights and its inputs' vertex values from inputs a geometry stage
    /// writes (`victory_shader::emit::bary`), for a host without
    /// barycentrics; made on first use (after [`Shader::translated`]), its
    /// bindings the same as the title translation's.
    /// shortcut: translated again each run, not kept in the shader cache.
    pub(crate) fn bary_variant(&self) -> Option<&Translated> {
        self.bary_variant
            .get_or_init(|| {
                let title = self.translated()?;
                let done = victory_shader::emit::for_program(&self.hash, || {
                    victory_shader::emit::bary::translate_with_barycentric_inputs(&self.container)
                });
                match done {
                    Ok(host) => {
                        tracing::info!(hash = %self.hash, "pixel shader translated to read its barycentrics from inputs");
                        Some(Translated {
                            host,
                            used: title.used.clone(),
                            stores: title.stores,
                            writes: title.writes,
                        })
                    }
                    Err(error) => {
                        tracing::warn!(hash = %self.hash, %error, "pixel shader does not translate with barycentric inputs");
                        None
                    }
                }
            })
            .as_ref()
    }
}

/// and writes each group's vertices and triangles to a record buffer, and
/// the vertex shader that draws the records with the title's pixel shader.
#[cfg(windows)]
pub(crate) struct MeshVariant {
    /// Reads and stores what the mesh program does (its `used` and
    /// `stores` are the program's), and writes the records besides.
    pub(crate) compute: Translated,
    /// Reads the records only.
    pub(crate) vertex: Translated,
    pub(crate) layout: victory_shader::emit::mesh_compute::RecordLayout,
}

#[cfg(windows)]
impl Shader {
    /// The mesh program as compute plus a vertex shader, made on first use
    /// (after [`Shader::translated`]); a program that does not translate so
    /// is logged once.
    /// shortcut: translated again each run, not kept in the shader cache.
    pub(crate) fn mesh_variant(&self) -> Option<&MeshVariant> {
        self.mesh_variant
            .get_or_init(|| {
                let program = self.translated()?;
                let done = victory_shader::emit::mesh_compute::translate_mesh_as_compute(&self.container)
                    .and_then(|m| Ok((m.vertex_shader()?, m)))
                    .map_err(|e| e.to_string());
                let (vertex, m) = match done {
                    Ok(done) => done,
                    Err(error) => {
                        tracing::warn!(
                            hash = %self.hash,
                            %error,
                            "mesh program does not translate as compute; its draws are dropped"
                        );
                        return None;
                    }
                };
                if let Some(dir) = std::env::var_os("VICTORY_SHADER_SAVE") {
                    let dir = std::path::PathBuf::from(dir);
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(dir.join(format!("{}.mesh-compute.dxil", self.hash)), &m.cs.dxil);
                    let _ = std::fs::write(dir.join(format!("{}.mesh-vertex.dxil", self.hash)), &vertex.dxil);
                }
                KEPT[2].fetch_add(2, std::sync::atomic::Ordering::Relaxed);
                KEPT[3].fetch_add(
                    (m.cs.dxil.len() + vertex.dxil.len()) as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                tracing::info!(
                    hash = %self.hash,
                    vertices = m.layout.max_vertices,
                    primitives = m.layout.max_primitives,
                    outputs = m.layout.vertex_slots,
                    group_bytes = m.layout.group_bytes(),
                    "mesh program translated as compute plus a vertex shader"
                );
                Some(MeshVariant {
                    compute: Translated {
                        host: m.cs,
                        used: program.used.clone(),
                        stores: program.stores,
                        writes: program.writes,
                    },
                    vertex: Translated {
                        host: vertex,
                        used: Vec::new(),
                        stores: false,
                        writes: false,
                    },
                    layout: m.layout,
                })
            })
            .as_ref()
    }
}

/// A mesh shader linked to the pixel shader it draws for
/// (`victory_shader::emit::link`), when its own translation's outputs pass
/// D3D12's 32 KiB: the mesh code with only the outputs the pixel shader
/// reads, the pixel code with only those inputs; without a pixel shader
/// (depth only), the mesh code with its system values alone. None when the mesh shader
/// fits alone (or a link cannot be made: the pipeline is then made from the
/// two translations, as before). Mesh variants are kept by (mesh, inputs the
/// pixel shader reads), pixel variants by pixel shader.
#[cfg(windows)]
pub(crate) fn linked(ms: &Shader, ps: Option<&Shader>) -> Option<(Arc<Vec<u8>>, Option<Arc<Vec<u8>>>)> {
    use victory_shader::emit::{self, link};
    type Made = Option<Arc<emit::HostShader>>;
    static MESH: Mutex<Option<HashMap<(String, Vec<link::Semantic>), Made>>> = Mutex::new(None);
    static PIXEL: Mutex<Option<HashMap<String, Made>>> = Mutex::new(None);
    let mt = &ms.translated()?.host;
    if !link::mesh_outputs_over(mt) {
        return None;
    }
    let pt = match ps {
        Some(ps) => Some(&ps.translated()?.host),
        None => None,
    };
    let reads = match pt {
        Some(pt) => link::pixel_reads(pt).ok()?,
        None => Vec::new(),
    };
    let made = |own: &emit::HostShader, s: &Shader| -> Made {
        let r = emit::for_program(&s.hash, || link::translate_keeping(&s.container, &reads));
        match r {
            Ok(t) if link::same_bindings(own, &t) => Some(Arc::new(t)),
            Ok(_) => {
                tracing::warn!(stage = s.stage, hash = %s.hash, "linked translation binds differently; left unlinked");
                None
            }
            Err(e) => {
                tracing::warn!(stage = s.stage, hash = %s.hash, error = %e, "linked translation failed");
                None
            }
        }
    };
    let m = MESH
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .entry((ms.hash.clone(), reads.clone()))
        .or_insert_with(|| made(mt, ms))
        .clone()?;
    let p = match (ps, pt) {
        (Some(ps), Some(pt)) => Some(
            PIXEL
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get_or_insert_with(HashMap::new)
                .entry(ps.hash.clone())
                .or_insert_with(|| made(pt, ps))
                .clone()?,
        ),
        _ => None,
    };
    let pixel = ps.map_or("-", |ps| ps.hash.as_str());
    if link::mesh_outputs_over(&m) {
        tracing::warn!(mesh = %ms.hash, pixel, "mesh outputs the pixel shader reads pass 32 KiB");
        return None;
    }
    tracing::info!(mesh = %ms.hash, pixel, reads = reads.len(), "mesh shader linked to its pixel shader");
    Some((Arc::new(m.dxil.clone()), p.map(|p| Arc::new(p.dxil.clone()))))
}

#[cfg(windows)]
fn translate(container: &[u8]) -> Result<Translated, String> {
    use victory_shader::{Container, FourCC, HwStage};
    // The translation depends on it (a view index past a capped heap).
    crate::descriptor::host::probe_view_limit();
    let c = Container::parse(container).map_err(|e| e.to_string())?;
    let programs = match c.part(FourCC::DXIL) {
        Some(part) => victory_shader::native_programs(part.data).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    let host = crate::shader_cache::host_shader(container)?;
    let stores = programs.is_empty() || programs.iter().any(stores);
    let writes = stores || programs.iter().any(image_stores);
    let used = programs
        .into_iter()
        .find(|p| {
            matches!(
                p.stage,
                HwStage::Vs | HwStage::Ps | HwStage::Ms | HwStage::Cs | HwStage::Gs
            )
        })
        .map(|p| p.used)
        .unwrap_or_default();
    Ok(Translated { host, used, stores, writes })
}

/// Whether a console program has an image store or atomic (it writes a
/// texture through unordered access), or code that does not decode.
#[cfg(windows)]
fn image_stores(program: &victory_shader::NativeProgram) -> bool {
    use victory_shader::gfx10::{Encoding, decode};
    let Ok(code) = decode(&program.code) else {
        return true;
    };
    code.iter().any(|i| {
        matches!(i.encoding, Encoding::Mimg) && i.name().is_none_or(|n| n.contains("store") || n.contains("atomic"))
    })
}

/// The compute pipelines of the console's ray-tracing structure builder
/// (`XboxBvhBuilder.dll`; its programs are the ones with a spilled
/// user-data table) run **all or none**. Its stages feed each other
/// through memory: a stage that runs on what a dropped stage never wrote
/// computes its store offsets from garbage. The structures lie in plain
/// title memory, between the title's own objects, and a root view has no
/// size, so those stores land on the neighbours (run 98, with 14 of 35
/// stages translated: the title died within a second of the first builder
/// submission, on pointers that had become bounding-box bit patterns).
///
/// So a builder dispatch runs only while every builder pipeline the title
/// has created has a host pipeline; the first builder dispatch makes them
/// all on the spot instead of over the next frames (a stage "still being
/// made" would be a dropped stage). One that cannot be made turns the
/// builder off for the rest of the run.
///
/// shortcut: and only with `VICTORY_BVH_BUILDER=1`, until a run has shown
/// the whole builder and the host side of title memory by address
/// (`memory.rs`) working on a card; nothing reads the structures before
/// the programs that walk them translate.
#[cfg(windows)]
struct Builder {
    pipelines: Vec<Arc<make::Pending>>,
    /// Every one of `pipelines` has a host pipeline.
    whole: bool,
    /// One could not be made.
    off: bool,
}

#[cfg(windows)]
static BUILDER: Mutex<Builder> = Mutex::new(Builder {
    pipelines: Vec::new(),
    whole: false,
    off: false,
});

/// Whether `shaders` are a pipeline of the structure builder.
#[cfg(windows)]
fn of_builder(shaders: &[Arc<Shader>]) -> bool {
    use victory_shader::{Container, FourCC};
    let [shader] = shaders else { return false };
    if shader.stage != "compute" {
        return false;
    }
    let Ok(c) = Container::parse(&shader.container) else {
        return false;
    };
    c.part(FourCC::DXIL)
        .and_then(|part| victory_shader::native_programs(part.data).ok())
        .is_some_and(|programs| programs.iter().any(victory_shader::spill::is_spilled))
}

/// `VICTORY_BVH_QUARANTINE=1`: what the structure builder's stages store
/// stays in the host's copies and is not copied back into title memory. A
/// check for a first run of the whole builder: a stage that stores where
/// it should not can then not reach the title's own objects (run 98), and
/// the programs that walk the structures read the host's copies anyway.
/// shortcut: not a way to run: the title also reads results back (sizes,
/// counters) and gets none, and a copy's pages are overwritten from title
/// memory when the title writes them.
#[cfg(windows)]
pub(crate) fn builder_quarantined(this: This) -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    is_builder(this) && *ON.get_or_init(|| std::env::var_os("VICTORY_BVH_QUARANTINE").is_some_and(|v| v == "1"))
}

/// The compute shaders whose dispatches are left out, by hash as logs
/// print it ([`leave_out`]).
static LEFT_OUT: OnceLock<std::collections::HashSet<String>> = OnceLock::new();

/// Dispatches left out since the start, and the compute pipelines the
/// title made with a shader of the list.
pub(crate) static LEFT_OUT_COUNT: [std::sync::atomic::AtomicU64; 2] = [const { std::sync::atomic::AtomicU64::new(0) }; 2];

/// Leaves out every dispatch of the compute shaders named by `hashes` (the
/// 16 digits logs print): the effect they compute is not computed, and
/// what reads its result reads what was there. Compute pipelines only: a
/// hash of another stage's shader leaves out no draw, so that no list can
/// take geometry away. Once, before the title makes its pipelines; false if
/// a list was given already.
pub fn leave_out(hashes: impl IntoIterator<Item = String>) -> bool {
    let list: std::collections::HashSet<String> = hashes.into_iter().map(|h| h.to_ascii_lowercase()).collect();
    tracing::info!(
        shaders = list.len(),
        "the dispatches of compute pipelines made of these shaders are left out for this run; no draw is"
    );
    LEFT_OUT.set(list).is_ok()
}

/// (PROJECT ONE) The shaders whose draws are left out, by hash
/// ([`leave_out_draws`]).
static LEFT_OUT_DRAWS: OnceLock<std::collections::HashSet<String>> = OnceLock::new();

/// (PROJECT ONE) Draws left out since the start ([`leave_out_draws`]).
pub(crate) static DRAWS_LEFT_OUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// (PROJECT ONE) Leaves out every draw (plain, indexed, mesh, indirect) of
/// the graphics and mesh pipelines made with a shader named by `hashes`:
/// what they draw is not drawn, and what reads their targets reads what a
/// clear or an earlier frame left there. Unlike [`leave_out`] this takes
/// geometry away: it is for passes a picture can do without (shadow maps,
/// whose 1,300 indirect draws a frame cost a quarter of the card's time on
/// an RX 5600 XT, run of 25/11), asked for by a list of its own
/// (`VICTORY_SKIP_DRAW_SHADERS`), never by the lists of effects. Once,
/// before the title makes its pipelines; false if a list was given already.
pub fn leave_out_draws(hashes: impl IntoIterator<Item = String>) -> bool {
    let list: std::collections::HashSet<String> = hashes.into_iter().map(|h| h.to_ascii_lowercase()).collect();
    tracing::warn!(
        shaders = list.len(),
        "the DRAWS of graphics and mesh pipelines made of these shaders are left out for this run (VICTORY_SKIP_DRAW_SHADERS)"
    );
    LEFT_OUT_DRAWS.set(list).is_ok()
}

/// Whether a pipeline of `shaders` is left out: a compute pipeline
/// (`graphics` false) with a shader of the effects' list, or a graphics or
/// mesh pipeline with a shader of the draws' list.
#[cfg(windows)]
fn left_out(shaders: &[Arc<Shader>], graphics: bool) -> bool {
    let list = if graphics { &LEFT_OUT_DRAWS } else { &LEFT_OUT };
    list.get()
        .is_some_and(|list| !list.is_empty() && shaders.iter().any(|s| list.contains(&s.hash)))
}

/// Whether the dispatches (a compute pipeline) or the draws (a graphics or
/// mesh one) of the title's pipeline `this` are left out.
#[cfg(windows)]
pub(crate) fn is_left_out(this: This) -> bool {
    !this.is_null() && pipeline(this).left_out
}

/// (PROJECT ONE) Whether a draw of the title's pipeline `this` is left out
/// ([`leave_out_draws`]); counts it if so.
#[cfg(windows)]
pub(crate) fn draw_left_out(this: This) -> bool {
    let out = is_left_out(this);
    if out {
        DRAWS_LEFT_OUT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    out
}

/// Whether the title's pipeline `this` is one of the structure builder's.
#[cfg(windows)]
pub(crate) fn is_builder(this: This) -> bool {
    !this.is_null() && pipeline(this).builder
}

/// A stage of the builder was dropped after all: none runs from here on.
#[cfg(windows)]
pub(crate) fn builder_off(why: &'static str) {
    let mut b = BUILDER.lock().unwrap_or_else(|e| e.into_inner());
    if !b.off {
        b.off = true;
        tracing::warn!(why, "the ray-tracing structure builder is turned off for the rest of the run: its stages run all or none");
    }
}

/// At a dispatch of a builder pipeline: whether the builder runs (see
/// [`Builder`]).
#[cfg(windows)]
pub(crate) fn builder_runs() -> bool {
    static ALLOWED: OnceLock<bool> = OnceLock::new();
    if !*ALLOWED.get_or_init(|| std::env::var_os("VICTORY_BVH_BUILDER").is_some_and(|v| v == "1")) {
        return false;
    }
    let mut b = BUILDER.lock().unwrap_or_else(|e| e.into_inner());
    if b.off {
        return false;
    }
    if b.whole {
        return true;
    }
    let missing_pipelines: Vec<Arc<make::Pending>> = b
        .pipelines
        .iter()
        .filter(|p| !make::now(p))
        .cloned()
        .collect();
    let missing = missing_pipelines.len();
    if missing > 0 {
        b.off = true;
        for p in &missing_pipelines {
            if let Some(shader) = p.shaders.first() {
                tracing::warn!(
                    hash = %shader.hash,
                    stage = %shader.stage,
                    "the ray-tracing structure builder is missing this program's host pipeline"
                );
            }
        }
        tracing::warn!(
            missing,
            of = b.pipelines.len(),
            "the ray-tracing structure builder stays off: not all of its programs have a host pipeline, and its stages run all or none"
        );
        return false;
    }
    tracing::info!(
        pipelines = b.pipelines.len(),
        "the ray-tracing structure builder runs: all of its programs have host pipelines"
    );
    b.whole = true;
    true
}

/// Whether a console program can store to buffer memory: it has a buffer,
/// typed-buffer, scalar-memory, flat or global store or atomic, or code
/// that is not all known (an instruction of those kinds outside our table,
/// an indirect jump, code that does not decode). Stores to textures
/// (image instructions) and to the program's own scratch memory do not
/// count: neither reaches a buffer's host copy. After upstream #413,
/// which leaves scalar-memory stores out and counts image stores.
#[cfg(windows)]
fn stores(program: &victory_shader::NativeProgram) -> bool {
    use victory_shader::gfx10::{Encoding, decode};
    let Ok(code) = decode(&program.code) else {
        return true;
    };
    code.iter().any(|i| {
        let name = i.name();
        match i.encoding {
            Encoding::Mubuf
            | Encoding::Mtbuf
            | Encoding::Smem
            | Encoding::Flat
            | Encoding::Global => name.is_none_or(|n| {
                (n.contains("store") || n.contains("atomic")) && !n.contains("scratch")
            }),
            Encoding::Sop1 => name.is_none_or(|n| matches!(n, "s_setpc_b64" | "s_swappc_b64")),
            _ => false,
        }
    })
}

#[repr(C)]
struct Pipeline {
    object: Object,
    packet: [u32; 8],
    /// Each used stage's container.
    shaders: Vec<Arc<Shader>>,
    /// What a precompiled pipeline (device methods 0x1c8 and 0x218) was
    /// made from; its shaders are the archive's containers its stages name
    /// (none with `VICTORY_PRECOMPILED_PSO=stub`).
    precompiled: Option<Precompiled>,
    /// Whether a list has bound it yet (its shaders are logged then).
    bound: std::sync::atomic::AtomicBool,
    /// What its host pipeline is made from, and the host pipeline once made
    /// (by a worker: the title may release the pipeline meanwhile).
    #[cfg(windows)]
    host: Arc<make::Pending>,
    /// One of the ray-tracing structure builder's ([`Builder`]).
    #[cfg(windows)]
    builder: bool,
    /// A compute pipeline whose dispatches are left out ([`leave_out`]).
    #[cfg(windows)]
    left_out: bool,
}
const _: () = assert!(offset_of!(Pipeline, packet) == 0x10);

/// Stage names, in description order.
const GRAPHICS_STAGES: [&str; 5] = ["vertex", "pixel", "domain", "hull", "geometry"];

fn pipeline<'a>(this: This) -> &'a Pipeline {
    // SAFETY: the title passes one of our pipelines.
    unsafe { &*this.cast::<Pipeline>() }
}

/// List method: logs each pipeline's shaders the first time a list binds
/// it, so a run lists the shaders its frames need first.
pub(crate) fn bound(this: This) {
    if this.is_null() {
        return;
    }
    let p = pipeline(this);
    if p.bound.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let frame = crate::frame::current();
    for shader in &p.shaders {
        tracing::debug!(pipeline = ?this, frame, stage = shader.stage, size = shader.container.len(), hash = %shader.hash, "first bind");
    }
}

/// The host pipeline of a title pipeline; `None` if one of its shaders does
/// not translate, the host refuses it, or it is still being made (its first
/// use starts that, see [`make`]).
#[cfg(windows)]
pub(crate) fn host_pipeline<'a>(this: This) -> Option<&'a host::HostPipeline> {
    if this.is_null() {
        return None;
    }
    make::ready(&pipeline(this).host)
}

/// Host pipelines queued and not made yet.
#[cfg(windows)]
pub(crate) fn making() -> u64 {
    make::waiting()
}

/// (PROJECT ONE) For the frame log: microseconds frames waited for
/// pipelines, pipelines made and microseconds spent making them (since the
/// start), and the age in milliseconds of the oldest pipeline a draw waits
/// for (now).
#[cfg(windows)]
pub(crate) fn make_stats() -> [u64; 4] {
    use std::sync::atomic::Ordering::Relaxed;
    [
        make::FRAME_WAIT_US.load(Relaxed),
        make::MADE_COUNT.load(Relaxed),
        make::MAKE_US.load(Relaxed),
        make::oldest_requested_ms(),
    ]
}

/// Draws and dispatches dropped since the last [`take_drops`], by reason
/// (diagnostics).
#[cfg(all(windows, feature = "diagnostics"))]
static DROPS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

/// Shader hashes as printed in the pipeline diagnostics.
#[cfg(all(windows, feature = "diagnostics"))]
fn parse_skip_hashes(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|hash| !hash.is_empty())
        .filter_map(|hash| {
            if hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                Some(hash.to_ascii_lowercase())
            } else {
                tracing::warn!(
                    hash,
                    "VICTORY_SKIP_PIPELINES expects 16-digit shader hashes"
                );
                None
            }
        })
        .collect()
}

/// The skipped shader hashes: `VICTORY_SKIP_PIPELINES`, plus the file
/// `VICTORY_SKIP_FILE` names (hashes separated by commas or lines, `#`
/// comments), read again when it changes (checked every 2 s), so passes
/// can be switched off and on while the title runs.
#[cfg(all(windows, feature = "diagnostics"))]
fn skip_hashes() -> std::sync::Arc<std::collections::HashSet<String>> {
    use std::collections::HashSet;
    use std::sync::Arc;
    struct Skips {
        env: Vec<String>,
        file: Option<std::path::PathBuf>,
        modified: Option<std::time::SystemTime>,
        checked: std::time::Instant,
        set: Arc<HashSet<String>>,
    }
    static SKIPS: Mutex<Option<Skips>> = Mutex::new(None);
    let mut guard = SKIPS.lock().unwrap_or_else(|e| e.into_inner());
    let s = guard.get_or_insert_with(|| {
        let env = parse_skip_hashes(&std::env::var("VICTORY_SKIP_PIPELINES").unwrap_or_default());
        Skips {
            set: Arc::new(env.iter().cloned().collect()),
            env,
            file: std::env::var_os("VICTORY_SKIP_FILE").map(Into::into),
            modified: None,
            checked: std::time::Instant::now() - std::time::Duration::from_secs(10),
        }
    });
    if let Some(file) = &s.file
        && s.checked.elapsed() >= std::time::Duration::from_secs(2)
    {
        s.checked = std::time::Instant::now();
        let modified = std::fs::metadata(file).and_then(|m| m.modified()).ok();
        if modified != s.modified {
            s.modified = modified;
            let text = std::fs::read_to_string(file).unwrap_or_default();
            let listed: String = text
                .lines()
                .map(|l| l.split('#').next().unwrap_or(""))
                .collect::<Vec<_>>()
                .join(",");
            let mut set: HashSet<String> = s.env.iter().cloned().collect();
            set.extend(parse_skip_hashes(&listed));
            tracing::warn!(count = set.len(), "VICTORY_SKIP_FILE: skipped shader hashes now");
            s.set = Arc::new(set);
        }
    }
    s.set.clone()
}

#[cfg(all(windows, feature = "diagnostics"))]
fn skipped_shader(p: &Pipeline) -> Option<&str> {
    let hashes = skip_hashes();
    if hashes.is_empty() {
        return None;
    }
    p.shaders
        .iter()
        .find(|shader| hashes.contains(&shader.hash))
        .map(|shader| shader.hash.as_str())
}

#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn is_skipped(this: This) -> bool {
    !this.is_null() && skipped_shader(pipeline(this)).is_some()
}

/// Counts a draw or dispatch of `this` intentionally skipped or dropped
/// for want of a host pipeline (diagnostics).
#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn note_drop(this: This) {
    let why = if this.is_null() {
        Some("no pipeline set".to_owned())
    } else {
        let p = pipeline(this);
        skipped_shader(p)
            .map(|hash| format!("skipped shader {hash}"))
            .or_else(|| {
                (p.precompiled.is_some() && p.shaders.is_empty())
                    .then(|| "precompiled pipeline without a host pipeline (VICTORY_PRECOMPILED_PSO=stub)".to_owned())
            })
            .or_else(|| make::missing(&p.host))
    };
    if let Some(why) = why {
        note_why(why);
    }
}

/// Counts a draw or dispatch dropped for `why` (diagnostics).
#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn note_why(why: String) {
    crate::capture::op(0, || format!("DROPPED {why}"));
    let mut drops = DROPS.lock().unwrap_or_else(|e| e.into_inner());
    *drops.get_or_insert_with(HashMap::new).entry(why).or_default() += 1;
}

/// The drops counted since the last call, most first (diagnostics).
#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn take_drops() -> Vec<(String, u64)> {
    let drops = DROPS.lock().unwrap_or_else(|e| e.into_inner()).take();
    let mut drops: Vec<(String, u64)> = drops.into_iter().flatten().collect();
    // Frame reports only show the first 40 reasons. Keep ties ordered so
    // a HashMap iteration order change cannot change which reasons appear.
    drops.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    drops
}

/// The stage's container, if it has one.
fn copy(stage: &'static str, code: &Bytecode) -> Option<Arc<Shader>> {
    if code.code.is_null() || code.size == 0 {
        return None;
    }
    // SAFETY: the title passes `size` bytes of shader code.
    Some(shader(stage, unsafe {
        std::slice::from_raw_parts(code.code, code.size)
    }))
}

fn hand_out(
    shaders: Vec<Arc<Shader>>,
    #[cfg(windows)] graphics: Option<Box<host::GraphicsState>>,
    precompiled: Option<Precompiled>,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    tracing::trace!(?shaders, "pipeline state");
    // (A graphics pipeline made from a plain description has no state
    // kept: told from a compute one by its shaders.)
    #[cfg(windows)]
    let draws = graphics.is_some() || !matches!(shaders.as_slice(), [only] if only.stage == "compute");
    #[cfg(windows)]
    let left_out = left_out(&shaders, draws);
    #[cfg(windows)]
    if left_out && LEFT_OUT_COUNT[1].fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 64 {
        tracing::info!(?shaders, draws, "a pipeline whose dispatches or draws are left out, as the list given asks");
    }
    #[cfg(windows)]
    let host = Arc::new(make::Pending::new(shaders.clone(), graphics));
    #[cfg(windows)]
    make::early(&host);
    #[cfg(windows)]
    let builder = of_builder(&shaders);
    #[cfg(windows)]
    if builder {
        // Another stage: the group is looked at again at its next dispatch.
        let mut b = BUILDER.lock().unwrap_or_else(|e| e.into_inner());
        b.pipelines.push(host.clone());
        b.whole = false;
    }
    let pipeline = Box::new(Pipeline {
        object: Object::new(&FUNCTIONS),
        packet: [0; 8],
        #[cfg(windows)]
        host,
        #[cfg(windows)]
        builder,
        #[cfg(windows)]
        left_out,
        shaders,
        precompiled,
        bound: std::sync::atomic::AtomicBool::new(false),
    });
    object::hand_out(PIPELINE, pipeline, iid, &PIPELINE_IIDS, out)
}

/// Device method.
pub(crate) extern "system" fn create_graphics(
    _: This,
    desc: *const GraphicsHead,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes a description (null is tolerated).
    let Some(desc) = (unsafe { desc.as_ref() }) else {
        return E_INVALIDARG;
    };
    let stages = GRAPHICS_STAGES.iter().zip(&desc.stages);
    let shaders = stages.filter_map(|(s, c)| copy(s, c)).collect();
    hand_out(
        shaders,
        #[cfg(windows)]
        None,
        None,
        iid,
        out,
    )
}

/// Device method.
pub(crate) extern "system" fn create_compute(
    _: This,
    desc: *const ComputeHead,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes a description (null is tolerated).
    let Some(desc) = (unsafe { desc.as_ref() }) else {
        return E_INVALIDARG;
    };
    hand_out(
        copy("compute", &desc.stage).into_iter().collect(),
        #[cfg(windows)]
        None,
        None,
        iid,
        out,
    )
}

/// Device method 0x1f0: the plain compute description plus console
/// extensions `(desc, extension count, extensions, iid, out)`.
/// shortcut: the extensions (console shader settings) are ignored, as is
/// the rest of the description.
pub(crate) extern "system" fn create_compute_x(
    this: This,
    desc: *const ComputeHead,
    _extensions: u32,
    _extended: *const u8,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    create_compute(this, desc, iid, out)
}

/// A pipeline described as a stream: its size and the subobjects, each a
/// type then its data, padded to 8 bytes.
#[repr(C)]
pub(crate) struct StreamDesc {
    size: usize,
    stream: *const u8,
}

/// A subobject type's size (type included), and the stage it is if it is a
/// shader. The console numbers amplification and mesh shaders 23 and 24
/// (PC: 24 and 25); its own types from 25 on are not sized yet.
fn subobject(kind: u32) -> Option<(usize, Option<&'static str>)> {
    let stage = |name| Some((24, Some(name)));
    match kind {
        1 => stage("vertex"),
        2 => stage("pixel"),
        3 => stage("domain"),
        4 => stage("hull"),
        5 => stage("geometry"),
        6 => stage("compute"),
        23 => stage("amplification"),
        24 => stage("mesh"),
        0 => Some((16, None)),
        12 | 19 => Some((24, None)),
        7 | 15 => Some((40, None)),
        8 => Some((336, None)),
        10 => Some((48, None)),
        11 => Some((56, None)),
        17 => Some((16, None)),
        21 => Some((64, None)),
        22 => Some((32, None)),
        9 | 13 | 14 | 16 | 18 | 20 => Some((8, None)),
        _ => None,
    }
}

/// Device method: a pipeline from a stream; the shaders are kept, and the
/// graphics state for the host pipeline.
pub(crate) extern "system" fn create_stream(
    _: This,
    desc: *const StreamDesc,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes a description (null is tolerated).
    let Some(desc) = (unsafe { desc.as_ref() }) else {
        return E_INVALIDARG;
    };
    if desc.stream.is_null() {
        return E_INVALIDARG;
    }
    let mut shaders = Vec::new();
    #[cfg(windows)]
    let mut graphics = host::GraphicsState::default();
    let mut at = 0;
    while at + 8 <= desc.size {
        // SAFETY: inside the stream the title passes.
        let kind = unsafe { desc.stream.add(at).cast::<u32>().read_unaligned() };
        let Some((size, stage)) = subobject(kind) else {
            tracing::warn!(
                kind,
                at,
                "pipeline subobject of unknown size; the rest is ignored"
            );
            break;
        };
        if let Some(stage) = stage {
            // SAFETY: a shader subobject has its bytecode at +8.
            let code = unsafe { &*desc.stream.add(at + 8).cast::<Bytecode>() };
            shaders.extend(copy(stage, code));
        }
        #[cfg(windows)]
        // SAFETY: a whole subobject of this kind lies at `at`.
        unsafe {
            graphics.read(kind, desc.stream.add(at))
        };
        at += size;
    }
    hand_out(
        shaders,
        #[cfg(windows)]
        Some(Box::new(graphics)),
        None,
        iid,
        out,
    )
}

extern "system" fn query(this: This, iid: *const Iid, out: *mut This) -> HRESULT {
    let found = object::is(iid, &PIPELINE_IIDS).then_some(this);
    object::answer(PIPELINE, iid, found, out)
}
extern "system" fn release(this: This) -> u32 {
    object::release_boxed::<Pipeline>(this)
}

/// IUnknown 3, object 4, device child 1, pipeline 3.
static FUNCTIONS: Table<24> = {
    let mut f = object::d3d12_object(object::missing_table::<PIPELINE, 24>());
    f[0] = query as *const ();
    f[2] = release as *const ();
    f[7] = object::parent_device as *const (); // 0x38
    Table(f)
};

mod pack;
mod precompiled;
pub use pack::use_shader_pack;
pub use precompiled::PrecompileReport;
#[cfg(windows)]
pub use precompiled::precompile_graphics;
pub(crate) use precompiled::{Precompiled, create_precompiled};

#[cfg(windows)]
pub(crate) mod host;
#[cfg(windows)]
mod library;
#[cfg(windows)]
mod make;
#[cfg(windows)]
pub(crate) use make::worker_count;

/// Lets the pipeline library's mapped file go from memory when no pipeline
/// has loaded for a while (`library::trim`); called by the host's sweep.
#[cfg(windows)]
pub(crate) fn trim_library() {
    library::trim();
}

/// The pipeline library's part of the report when the host device is
/// removed (`library::removal_note`).
#[cfg(windows)]
pub(crate) fn library_removal_note() -> String {
    library::removal_note()
}

/// Keeps compiled host pipelines, and the list of title pipelines to start
/// early, in `dir` (a folder of the cache) across runs.
#[cfg(windows)]
pub fn use_pipeline_cache(dir: std::path::PathBuf) {
    library::use_pipeline_cache(dir);
}

/// What [`prepare`] did with the containers it was given.
#[cfg(windows)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Prepared {
    /// Containers of a stage a run translates (vertex, pixel, mesh,
    /// compute); the others are left alone.
    pub shaders: usize,
    /// Of them, those that translate, and those that do not.
    pub translated: usize,
    pub untranslated: usize,
    /// Translations the shader cache held already.
    pub cached: usize,
    /// Host pipelines in the library now (made, or loaded from it: made by
    /// an earlier run), and those the host refused.
    pub pipelines: usize,
    pub loaded: usize,
    pub refused: usize,
}

/// The stage of the program a run takes from a container, as pipelines
/// name stages.
#[cfg(windows)]
fn stage_of(container: &[u8]) -> Option<&'static str> {
    use victory_shader::{Container, FourCC, HwStage};
    let c = Container::parse(container).ok()?;
    let programs = victory_shader::native_programs(c.part(FourCC::DXIL)?.data).ok()?;
    // A container with a geometry or tessellation program is one of those
    // stages', which a run does not translate.
    if programs.iter().any(|p| !matches!(p.stage, HwStage::Vs | HwStage::Ps | HwStage::Ms | HwStage::Cs)) {
        return None;
    }
    programs.iter().find_map(|p| match p.stage {
        HwStage::Vs => Some("vertex"),
        HwStage::Ps => Some("pixel"),
        HwStage::Ms => Some("mesh"),
        HwStage::Cs => Some("compute"),
        _ => None,
    })
}

/// Before a run, apart from any title: translates shader `containers` into
/// the shader cache ([`crate::shader_cache`]), so that a run with this
/// build reads the translations instead of making them, and makes the host
/// pipelines that depend on one shader alone, which the pipeline library
/// then holds: with `compute`, the compute pipelines; with `mesh`, on a
/// host where mesh pipelines run as compute, the compute half of each mesh
/// program. Vertex and pixel pipelines need the title's graphics state,
/// known only in a run.
///
/// `jobs` threads work through the containers; `progress` is told now and
/// then how many are done. The caches must be set
/// ([`crate::shader_cache::use_shader_cache`], [`use_pipeline_cache`]).
#[cfg(windows)]
pub fn prepare(
    containers: &[&[u8]],
    compute: bool,
    mesh: bool,
    jobs: usize,
    progress: &(dyn Fn(usize) + Sync),
) -> Result<Prepared, String> {
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    if crate::host::host().is_none() {
        return Err("no host GPU: nothing is prepared".to_owned());
    }
    // A translation depends on it (a view index past a capped heap).
    crate::descriptor::host::probe_view_limit();
    let mesh = mesh && host::mesh_as_compute();
    let (hits, _) = crate::shader_cache::counts();
    let loaded = library::LOADED.load(Relaxed);
    let next = AtomicUsize::new(0);
    // Shaders, translated, untranslated, pipelines, refused.
    let counts = [const { AtomicUsize::new(0) }; 5];
    let count = |i: usize| {
        counts[i].fetch_add(1, Relaxed);
    };
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Relaxed);
                    let Some(&container) = containers.get(i) else {
                        break;
                    };
                    if i.is_multiple_of(64) {
                        progress(i);
                    }
                    let Some(stage) = stage_of(container) else {
                        continue;
                    };
                    count(0);
                    // The translation into the cache first, not kept in
                    // memory. A failure of the system's (the cache's file
                    // under this many threads: "insufficient system
                    // resources", once in 1,850 shaders) is tried again.
                    let translated = match crate::shader_cache::host_shader(container) {
                        Ok(_) => true,
                        Err(e) if e.contains("os error") => {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            crate::shader_cache::host_shader(container).is_ok()
                        }
                        Err(_) => false,
                    };
                    if !translated {
                        count(2);
                        continue;
                    }
                    count(1);
                    if !(stage == "compute" && compute || stage == "mesh" && mesh) {
                        continue;
                    }
                    // From the cache now.
                    let shader = shader(stage, container);
                    if shader.translated().is_none() {
                        count(4);
                        continue;
                    }
                    let shaders = [shader.clone()];
                    let made = if stage == "mesh" {
                        host::HostPipeline::mesh_compute_half(&shaders, &shader).is_some()
                    } else {
                        // Not the ones a run leaves out for what they cost
                        // the driver (`make::left_out`): no run asks for them.
                        match make::warm(shaders.to_vec()) {
                            Some(made) => made,
                            None => continue,
                        }
                    };
                    count(if made { 3 } else { 4 });
                }
            });
        }
    });
    library::save(true);
    let [shaders, translated, untranslated, pipelines, refused] = counts.map(|c| c.load(Relaxed));
    Ok(Prepared {
        shaders,
        translated,
        untranslated,
        // A shader a pipeline was made from was read a second time, from
        // the cache whatever it held before.
        cached: ((crate::shader_cache::counts().0 - hits) as usize).saturating_sub(pipelines + refused),
        pipelines,
        loaded: (library::LOADED.load(Relaxed) - loaded) as usize,
        refused,
    })
}

/// Saves the pipeline cache now if anything was added (at the end of a run).
#[cfg(windows)]
pub fn save_pipeline_cache() {
    library::save(true);
}

#[cfg(test)]
mod tests {
    /// Bytes that are no shader container are left alone by a
    /// preparation, and nothing is counted for them.
    #[cfg(windows)]
    #[test]
    fn a_preparation_leaves_what_is_no_shader_alone() {
        assert_eq!(super::stage_of(&[]), None);
        assert_eq!(super::stage_of(b"DXBC, but no container at all"), None);
        // (Without a host card there is nothing to prepare with.)
        if let Ok(done) = super::prepare(&[b"not a container", &[0u8; 64]], true, true, 2, &|_| {}) {
            assert_eq!(done, super::Prepared::default());
        }
    }

    /// Only compute pipelines made of a listed shader are left out: a
    /// listed shader in a pipeline that draws leaves out no draw.
    #[cfg(windows)]
    #[test]
    fn only_compute_pipelines_of_listed_shaders_are_left_out() {
        let listed = super::shader("compute", b"a compute shader the list names");
        let other = super::shader("compute", b"a compute shader it does not name");
        assert!(!super::left_out(std::slice::from_ref(&listed), false), "no list yet");
        assert!(super::leave_out([listed.hash.to_ascii_uppercase()]));
        assert!(super::left_out(std::slice::from_ref(&listed), false));
        assert!(!super::left_out(std::slice::from_ref(&other), false));
        assert!(!super::left_out(&[other, listed], true), "never a draw");
        assert!(!super::leave_out([String::new()]), "one list a run");
    }

    /// (PROJECT ONE) Draws are left out by their own list alone: a shader of
    /// it in a graphics or mesh pipeline, never a compute pipeline, and the
    /// effects' list still leaves out no draw.
    #[cfg(windows)]
    #[test]
    fn draws_are_left_out_by_their_own_list_only() {
        let shadow = super::shader("mesh", b"a mesh shader of a shadow pass the draws' list names");
        let pixel = super::shader("pixel", b"its pixel shader, which the list does not name");
        let world = super::shader("mesh", b"a mesh shader of the world, not named");
        let compute = super::shader("compute", b"a compute shader, which a draws' list cannot take");
        assert!(!super::left_out(&[shadow.clone(), pixel.clone()], true), "no list yet");
        assert!(super::leave_out_draws([shadow.hash.to_ascii_uppercase(), compute.hash.clone()]));
        assert!(super::left_out(&[shadow.clone(), pixel.clone()], true), "one named shader is enough");
        assert!(!super::left_out(&[world, pixel], true));
        assert!(!super::left_out(std::slice::from_ref(&compute), false), "a dispatch is not a draw");
        assert!(!super::leave_out_draws([String::new()]), "one list a run");
    }

    #[cfg(all(windows, feature = "diagnostics"))]
    #[test]
    fn skip_hashes_accepts_full_hashes_only() {
        assert!(super::parse_skip_hashes(" , ").is_empty());
        assert_eq!(
            super::parse_skip_hashes(" 0123456789ABCDEF, 0123456789abcde, xyz, 0011223344556677,"),
            ["0123456789abcdef", "0011223344556677"]
        );
    }

    use super::*;
    use crate::object::{S_OK, slot};

    fn shaders(p: This) -> Vec<(&'static str, Vec<u8>)> {
        pipeline(p)
            .shaders
            .iter()
            .map(|s| (s.stage, s.container.clone()))
            .collect()
    }

    #[test]
    fn pipelines_keep_their_shaders() {
        let (vs, ps) = ([1u8, 2, 3], [4u8; 5]);
        let none = || Bytecode {
            code: std::ptr::null(),
            size: 0,
        };
        let desc = GraphicsHead {
            root: std::ptr::null(),
            stages: [
                Bytecode {
                    code: vs.as_ptr(),
                    size: vs.len(),
                },
                Bytecode {
                    code: ps.as_ptr(),
                    size: ps.len(),
                },
                none(),
                none(),
                none(),
            ],
        };
        let mut p = std::ptr::null();
        assert_eq!(
            create_graphics(std::ptr::null(), &desc, &PIPELINE_IIDS[0], &mut p),
            S_OK
        );
        assert_eq!(
            shaders(p),
            [("vertex", vs.to_vec()), ("pixel", ps.to_vec())]
        );
        // A second pipeline with the same pixel shader shares its container.
        let mut q = std::ptr::null();
        create_graphics(std::ptr::null(), &desc, &PIPELINE_IIDS[0], &mut q);
        assert!(Arc::ptr_eq(
            &pipeline(p).shaders[1],
            &pipeline(q).shaders[1]
        ));
        assert_eq!(slot::<extern "system" fn(This) -> u32>(p, 0x10)(p), 0);
    }

    /// The stream layout the title builds (title+0x86f090): root
    /// signature, five shader slots (the first a mesh shader), blend, sample
    /// mask, rasterizer, depth-stencil 1, sample description, input layout,
    /// topology, render target and depth formats: 0x2b0 bytes.
    #[test]
    fn streams_yield_their_shaders() {
        let (ms, ps) = ([9u8; 7], [4u8; 5]);
        let mut stream = vec![0u8; 0x2b0];
        let mut put = |at: usize, v: u64| stream[at..at + 8].copy_from_slice(&v.to_le_bytes());
        for (at, kind) in [
            (0x10, 24),
            (0x28, 2),
            (0x40, 3),
            (0x58, 4),
            (0x70, 5),
            (0x88, 8),
            (0x1d8, 9),
            (0x1e0, 10),
            (0x210, 21),
            (0x250, 17),
            (0x260, 12),
            (0x278, 14),
            (0x280, 15),
            (0x2a8, 16),
        ] {
            put(at, kind);
        }
        put(0x18, ms.as_ptr() as u64);
        put(0x20, ms.len() as u64);
        put(0x30, ps.as_ptr() as u64);
        put(0x38, ps.len() as u64);
        // Topology type 3 (triangles), one render target of format 28.
        put(0x278, 14 | 3 << 32);
        put(0x280, 15 | 28 << 32);
        put(0x2a0, 1 << 32);
        let desc = StreamDesc {
            size: stream.len(),
            stream: stream.as_ptr(),
        };
        let mut p = std::ptr::null();
        let hr = create_stream(std::ptr::null(), &desc, &PIPELINE_IIDS[0], &mut p);
        assert_eq!(hr, S_OK);
        assert_eq!(shaders(p), [("mesh", ms.to_vec()), ("pixel", ps.to_vec())]);
        #[cfg(windows)]
        {
            let g = pipeline(p).host.graphics.as_deref().unwrap();
            assert_eq!(g.topology.0, 3);
            assert_eq!(
                (g.targets.NumRenderTargets, g.targets.RTFormats[0].0),
                (1, 28)
            );
        }
    }
}
