//! (PROJECT ONE) The title's graphics preset, chosen and lowered before the
//! title applies it. Off unless `VICTORY_PRESET` names a profile.
//!
//! What the 1.0.2.1 executable does (read in the binary, 26/11):
//!
//! - The caller (RVA 0x5b56ae2..0x5b56b3b) fills a settings structure (in
//!   `r15`), calls the selector through a thunk (0x5b56b25 -> 0x5b56b90 ->
//!   0x5b56bc0) with `rcx` = the settings manager and `rdx` = the structure,
//!   then calls 0x1d70c40 (manager, structure, ...), which applies the
//!   settings and runs their callbacks.
//! - The selector (0x5b56bc0) asks the platform (0x501ad20: 2 on a Series X,
//!   1 when the Series S flag of 0x75140 is set), picks the key 0xBEEFBABE
//!   (Anaconda) or 0xDEADBEEF (Lockhart), stores it at structure + 0x6f0, and
//!   tail-jumps (0x5b56bf3, `jmp rel32`) to the lookup 0x1d704f0 (manager,
//!   key, structure).
//! - The lookup finds the key in the manager's table (+0x2470 count, +0x2478
//!   groups) and deserializes the preset's data into the structure. It
//!   returns 0 if the key is not there (the structure untouched), else what
//!   the deserializer returns.
//! - The key names are in the table at 0x6556c50 (name hash, key): Lockhart
//!   0xDEADBEEF, Prospero 0xCAFED00D, Anaconda 0xBEEFBABE, Trinity
//!   0xCAFEBABE, SafeMode 0x7C3F7B63, and a sixth (0xC352A563) unnamed here.
//!
//! The hook replaces the selector's tail jump with a jump to [`preset_hook`]
//! (through a thunk within 2 GiB of the image). It looks up the chosen base
//! preset (falling back to the selector's own key if that one is not in
//! the table), then lowers the fields of the profile in the structure, and
//! returns: the caller then applies the structure, callbacks and all, as it
//! always does. Nothing else of the title is changed.
//!
//! The field offsets come from the user's extraction of the settings schema
//! (data file, 26/11). The binary agrees on their order (the members' name
//! hashes are in the same order in its metadata); their exact offsets were
//! not re-derived here. So each field is checked before it is written: an
//! enum must hold 0..=4, a bool 0 or 1, a float a finite value in (0, 16];
//! a field that does not is left as it is, and said so in the log.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// The environment variable naming the profile.
pub(crate) const PRESET_ENV: &str = "VICTORY_PRESET";
/// The environment variable naming the base preset (`safemode` by default
/// for every profile; `original` keeps the selector's choice).
pub(crate) const PRESET_BASE_ENV: &str = "VICTORY_PRESET_BASE";

/// RVAs of the 1.0.2.1 executable.
const SELECTOR_TAIL_JUMP: usize = 0x5b56bf3;
/// `jmp 0x1d704f0` at [`SELECTOR_TAIL_JUMP`].
const SELECTOR_TAIL_BYTES: [u8; 5] = [0xE9, 0xF8, 0x98, 0x21, 0xFC];
const LOOKUP: usize = 0x1d704f0;
/// Where the selector stores the key in the structure.
const KEY_AT: usize = 0x6f0;
/// The settings structure's size (the furthest field the selector and the
/// caller write is at 0x6f0).
pub(crate) const SETTINGS_BYTES: usize = 0x700;

/// Preset keys of the title's table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Base {
    /// The selector's own choice (Anaconda, or Lockhart with the Series S
    /// flag).
    Original,
    SafeMode,
    Lockhart,
    Anaconda,
}

impl Base {
    pub(crate) fn key(self) -> Option<u32> {
        match self {
            Base::Original => None,
            Base::SafeMode => Some(0x7C3F_7B63),
            Base::Lockhart => Some(0xDEAD_BEEF),
            Base::Anaconda => Some(0xBEEF_BABE),
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Base::Original => "the title's own",
            Base::SafeMode => "SafeMode",
            Base::Lockhart => "Lockhart",
            Base::Anaconda => "Anaconda",
        }
    }

    fn of_key(key: u32) -> &'static str {
        match key {
            0x7C3F_7B63 => "SafeMode",
            0xDEAD_BEEF => "Lockhart",
            0xBEEF_BABE => "Anaconda",
            0xCAFE_D00D => "Prospero",
            0xCAFE_BABE => "Trinity",
            _ => "unknown",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Base> {
        match text.trim().to_ascii_lowercase().as_str() {
            "original" | "jeu" => Some(Base::Original),
            "safemode" | "safe-mode" => Some(Base::SafeMode),
            "lockhart" => Some(Base::Lockhart),
            "anaconda" => Some(Base::Anaconda),
            _ => None,
        }
    }
}

/// A field of the settings structure and how it is stored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    /// u32, Low 0 .. Ultra 3, Custom 4.
    Enum,
    /// f32.
    Float,
    /// One byte, 0 or 1 (bools are packed: never written wider).
    Bool,
}

impl Kind {
    pub(crate) fn size(self) -> usize {
        match self {
            Kind::Enum | Kind::Float => 4,
            Kind::Bool => 1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Field {
    pub(crate) name: &'static str,
    pub(crate) at: usize,
    pub(crate) kind: Kind,
}

/// The fields a profile may lower (offsets: see the module's text).
pub(crate) const FIELDS: [Field; 9] = [
    Field {
        name: "textureQuality",
        at: 0x4d4,
        kind: Kind::Enum,
    },
    Field {
        name: "volumetricsQuality",
        at: 0x4d8,
        kind: Kind::Enum,
    },
    Field {
        name: "bvhQuality",
        at: 0x4f8,
        kind: Kind::Enum,
    },
    Field {
        name: "grassLod",
        at: 0x518,
        kind: Kind::Float,
    },
    Field {
        name: "lodScale",
        at: 0x524,
        kind: Kind::Float,
    },
    Field {
        name: "motionBlur",
        at: 0x56c,
        kind: Kind::Bool,
    },
    Field {
        name: "shadowSoftShadows",
        at: 0x5ac,
        kind: Kind::Enum,
    },
    Field {
        name: "pedDensity",
        at: 0x6d0,
        kind: Kind::Enum,
    },
    Field {
        name: "vehicleDensity",
        at: 0x6d4,
        kind: Kind::Enum,
    },
];

/// A value a profile asks for. Floats are a ceiling: a field already lower
/// stays as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Value {
    Enum(u32),
    AtMost(f32),
    Bool(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Profile {
    /// The base preset alone.
    BaseOnly,
    /// A: every listed quality at Low, no motion blur, LOD 0.6 and grass
    /// 0.5 (the values proposed with the schema).
    SafeLow,
    /// B: A, with lower LOD and grass, and fewer pedestrians and vehicles.
    ExtremeLow,
    /// C: the least the profile can ask, to measure; not meant to play.
    Diagnostic,
}

impl Profile {
    pub(crate) fn parse(text: &str) -> Option<Profile> {
        match text.trim().to_ascii_lowercase().as_str() {
            "safemode" | "base" => Some(Profile::BaseOnly),
            "safe-low" | "bas" | "a" => Some(Profile::SafeLow),
            "extreme-low" | "extreme" | "b" => Some(Profile::ExtremeLow),
            "diagnostic" | "diag" | "c" => Some(Profile::Diagnostic),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Profile::BaseOnly => "base preset only",
            Profile::SafeLow => "Safe Low",
            Profile::ExtremeLow => "Extreme Low",
            Profile::Diagnostic => "Performance Diagnostic",
        }
    }

    /// What the profile asks of each field it changes.
    pub(crate) fn values(self) -> Vec<(&'static str, Value)> {
        let low = |n| (n, Value::Enum(0));
        let mut v = match self {
            Profile::BaseOnly => return Vec::new(),
            _ => vec![
                low("textureQuality"),
                low("volumetricsQuality"),
                low("bvhQuality"),
                low("shadowSoftShadows"),
                ("motionBlur", Value::Bool(false)),
            ],
        };
        let (lod, grass, fewer) = match self {
            Profile::SafeLow => (0.6, 0.5, false),
            Profile::ExtremeLow => (0.5, 0.25, true),
            Profile::Diagnostic => (0.3, 0.1, true),
            Profile::BaseOnly => return v,
        };
        v.push(("lodScale", Value::AtMost(lod)));
        v.push(("grassLod", Value::AtMost(grass)));
        if fewer {
            v.push(low("pedDensity"));
            v.push(low("vehicleDensity"));
        }
        v
    }
}

/// What [`apply`] did to one field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Outcome {
    Written,
    /// Already at (or below) what was asked.
    Kept,
    /// Not of the expected form: left alone (wrong offset or type?).
    Unexpected,
    /// The profile names a field the table does not have.
    Unknown,
}

/// Lowers `settings` (the structure, [`SETTINGS_BYTES`] long) as `profile`
/// asks: each field read and checked first, then written at its own size.
pub(crate) fn apply(settings: &mut [u8], profile: Profile) -> Vec<(&'static str, Outcome)> {
    let mut out = Vec::new();
    for (name, value) in profile.values() {
        let Some(f) = FIELDS.iter().find(|f| f.name == name) else {
            out.push((name, Outcome::Unknown));
            continue;
        };
        if f.at + f.kind.size() > settings.len() {
            out.push((name, Outcome::Unexpected));
            continue;
        }
        let at = f.at;
        let outcome = match (f.kind, value) {
            (Kind::Enum, Value::Enum(want)) => {
                let now = u32::from_le_bytes(settings[at..at + 4].try_into().unwrap_or([0xff; 4]));
                if now > 4 {
                    Outcome::Unexpected
                } else if now == want {
                    Outcome::Kept
                } else {
                    settings[at..at + 4].copy_from_slice(&want.to_le_bytes());
                    Outcome::Written
                }
            }
            (Kind::Float, Value::AtMost(most)) => {
                let now = f32::from_le_bytes(settings[at..at + 4].try_into().unwrap_or([0xff; 4]));
                if !(0.0..=16.0).contains(&now) {
                    Outcome::Unexpected
                } else if now <= most {
                    Outcome::Kept
                } else {
                    settings[at..at + 4].copy_from_slice(&most.to_le_bytes());
                    Outcome::Written
                }
            }
            (Kind::Bool, Value::Bool(want)) => {
                let now = settings[at];
                if now > 1 {
                    Outcome::Unexpected
                } else if (now == 1) == want {
                    Outcome::Kept
                } else {
                    settings[at] = u8::from(want);
                    Outcome::Written
                }
            }
            _ => Outcome::Unexpected,
        };
        out.push((name, outcome));
    }
    out
}

/// What the hook was set up with.
struct Setup {
    profile: Profile,
    base: Base,
}

static SETUP: OnceLock<Setup> = OnceLock::new();
/// The lookup's address in the mapped image.
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Times the hook ran (the title chooses its preset at start and when the
/// settings change).
static RUNS: AtomicU32 = AtomicU32::new(0);

type Lookup = unsafe extern "win64" fn(*mut u8, u32, *mut u8) -> u64;

/// Takes the selector's place after it chose its key: see the module's text.
///
/// # Safety
/// Reached only from the title's selector, with its arguments: the settings
/// manager, the key it chose, the structure of [`SETTINGS_BYTES`] bytes.
unsafe extern "win64" fn preset_hook(manager: *mut u8, key: u32, settings: *mut u8) -> u64 {
    let original = ORIGINAL.load(Ordering::Acquire);
    // SAFETY: set to the lookup's address before the jump to here was written.
    let lookup: Lookup = unsafe { std::mem::transmute::<usize, Lookup>(original) };
    let Some(setup) = SETUP.get() else {
        // SAFETY: the title's own call, unchanged.
        return unsafe { lookup(manager, key, settings) };
    };
    let run = RUNS.fetch_add(1, Ordering::Relaxed) + 1;
    // SAFETY: the selector's own arguments, passed on.
    let (found, report) = unsafe { choose(lookup, setup, manager, key, settings) };
    report.log(run, setup);
    found
}

/// What one run of the hook did, for the log.
#[derive(Debug, PartialEq)]
struct Report {
    /// The selector's key.
    chose: u32,
    /// The key looked up in the end.
    used: u32,
    /// The base asked for was not in the table.
    missing: bool,
    /// The lookup failed: nothing was written.
    failed: bool,
    done: Vec<(&'static str, Outcome)>,
}

impl Report {
    fn log(&self, run: u32, setup: &Setup) {
        if self.missing {
            tracing::warn!(
                "[PROJECT ONE] base preset {} not in the title's table: its own preset ({}) is kept",
                setup.base.name(),
                Base::of_key(self.chose)
            );
        }
        tracing::warn!(
            run,
            "[PROJECT ONE] Selected base preset: {} (the selector chose {})",
            Base::of_key(self.used),
            Base::of_key(self.chose)
        );
        if self.failed {
            tracing::warn!("[PROJECT ONE] the preset lookup failed: no setting overridden");
            return;
        }
        for (name, outcome) in &self.done {
            if matches!(outcome, Outcome::Unexpected | Outcome::Unknown) {
                tracing::warn!(
                    field = name,
                    ?outcome,
                    "[PROJECT ONE] setting left alone: not of the expected form"
                );
            }
        }
        let written = self
            .done
            .iter()
            .filter(|(_, o)| *o == Outcome::Written)
            .count();
        let kept = self
            .done
            .iter()
            .filter(|(_, o)| *o == Outcome::Kept)
            .count();
        let left = self.done.len() - written - kept;
        if written == 0 {
            tracing::warn!(
                "[PROJECT ONE] Low-end override: no setting changed ({kept} already as low, {left} left alone); Effective profile: {}",
                setup.profile.name()
            );
        } else {
            tracing::warn!(
                "[PROJECT ONE] Low-end override applied; Overridden settings: {written} ({kept} already as low, {left} left alone); Effective profile: {}; {:?}",
                setup.profile.name(),
                self.done
            );
        }
    }
}

/// The hook's work, with the lookup passed in: the base looked up (the
/// selector's key if the base is not in the table), then the profile's
/// fields lowered if the lookup succeeded.
///
/// # Safety
/// As [`preset_hook`]; `lookup` behaves as the title's.
unsafe fn choose(
    lookup: Lookup,
    setup: &Setup,
    manager: *mut u8,
    key: u32,
    settings: *mut u8,
) -> (u64, Report) {
    let mut report = Report {
        chose: key,
        used: key,
        missing: false,
        failed: false,
        done: Vec::new(),
    };
    let mut found = None;
    if let Some(base) = setup.base.key().filter(|&k| k != key) {
        // SAFETY: the structure is the caller's, KEY_AT inside it; the lookup
        // with another key of the same table.
        let got = unsafe {
            settings.add(KEY_AT).cast::<u32>().write_unaligned(base);
            lookup(manager, base, settings)
        };
        if got as u32 == 0 {
            report.missing = true;
            // SAFETY: as above, the selector's own key back.
            unsafe { settings.add(KEY_AT).cast::<u32>().write_unaligned(key) };
        } else {
            report.used = base;
            found = Some(got);
        }
    }
    // SAFETY: the title's own call.
    let found = found.unwrap_or_else(|| unsafe { lookup(manager, key, settings) });
    if found as u32 == 0 {
        report.failed = true;
        return (found, report);
    }
    // SAFETY: the lookup filled the caller's structure of SETTINGS_BYTES.
    let bytes = unsafe { std::slice::from_raw_parts_mut(settings, SETTINGS_BYTES) };
    report.done = apply(bytes, setup.profile);
    (found, report)
}

/// `mov rax, imm64; jmp rax`, to `target`.
pub(crate) fn thunk_bytes(target: u64) -> [u8; 12] {
    let mut t = [0u8; 12];
    t[0] = 0x48;
    t[1] = 0xB8;
    t[2..10].copy_from_slice(&target.to_le_bytes());
    t[10] = 0xFF;
    t[11] = 0xE0;
    t
}

/// `jmp rel32` at `from` to `to`, if it reaches.
pub(crate) fn jump_bytes(from: usize, to: usize) -> Option<[u8; 5]> {
    let rel = (to as i64).checked_sub(from as i64 + 5)?;
    let rel = i32::try_from(rel).ok()?;
    let mut j = [0u8; 5];
    j[0] = 0xE9;
    j[1..].copy_from_slice(&rel.to_le_bytes());
    Some(j)
}

/// Executable memory holding `code`, within 2 GiB below or above `near`.
pub(crate) fn code_near(near: usize, code: &[u8]) -> Option<usize> {
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READ, VirtualProtect};
    let got = room_near(near, code.len())?;
    let mut old = 0;
    // SAFETY: our own fresh pages.
    unsafe {
        std::ptr::copy_nonoverlapping(code.as_ptr(), got as *mut u8, code.len());
        VirtualProtect(got as *const _, code.len(), PAGE_EXECUTE_READ, &mut old);
    }
    Some(got)
}

/// Fresh read-write memory of `len` bytes within 2 GiB below or above `near`
/// (made executable by the caller once its code is in).
pub(crate) fn room_near(near: usize, len: usize) -> Option<usize> {
    use windows_sys::Win32::System::Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc};
    const STEP: usize = 0x10000;
    const REACH: usize = 0x7000_0000;
    let start = near & !(STEP - 1);
    for i in 1..REACH / STEP {
        for at in [start.checked_sub(i * STEP), start.checked_add(i * STEP)]
            .into_iter()
            .flatten()
        {
            // SAFETY: asks for fresh pages at a free address; null if taken.
            let got = unsafe { VirtualAlloc(at as _, len.max(1), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };
            if !got.is_null() {
                return Some(got as usize);
            }
        }
    }
    None
}

/// Installs the hook if [`PRESET_ENV`] names a profile and the image is the
/// 1.0.2.1 executable whose selector holds the expected jump. Whether it did.
///
/// # Safety
/// `base` is the mapped image, and no title code has run yet.
pub(crate) unsafe fn install(base: usize, is_1_0_2_1: bool) -> bool {
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let Some(text) = std::env::var(PRESET_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty() && v.trim() != "0")
    else {
        return false;
    };
    let Some(profile) = Profile::parse(&text) else {
        tracing::warn!(value = %text, "{PRESET_ENV}: unknown profile (safemode, safe-low, extreme-low, diagnostic): no preset hook");
        return false;
    };
    let base_preset = match std::env::var(PRESET_BASE_ENV).ok() {
        None => Base::SafeMode,
        Some(v) => match Base::parse(&v) {
            Some(b) => b,
            None => {
                tracing::warn!(value = %v, "{PRESET_BASE_ENV}: unknown base (original, safemode, lockhart, anaconda): SafeMode");
                Base::SafeMode
            }
        },
    };
    if !is_1_0_2_1 {
        tracing::warn!("{PRESET_ENV}: not the 1.0.2.1 executable: no preset hook");
        return false;
    }
    let at = base + SELECTOR_TAIL_JUMP;
    // SAFETY: inside the mapped .text of this exact build.
    let now = unsafe { std::slice::from_raw_parts(at as *const u8, 5) };
    if now != SELECTOR_TAIL_BYTES {
        tracing::warn!(
            "{PRESET_ENV}: the selector's bytes are not the expected ones: no preset hook"
        );
        return false;
    }
    let hook: unsafe extern "win64" fn(*mut u8, u32, *mut u8) -> u64 = preset_hook;
    let Some(thunk) = code_near(at, &thunk_bytes(hook as usize as u64)) else {
        tracing::warn!("{PRESET_ENV}: no memory within reach of the title for the hook");
        return false;
    };
    let Some(jump) = jump_bytes(at, thunk) else {
        return false;
    };
    ORIGINAL.store(base + LOOKUP, Ordering::Release);
    let _ = SETUP.set(Setup {
        profile,
        base: base_preset,
    });
    let mut old = 0;
    // SAFETY: the five bytes checked above, writable for the write only.
    unsafe {
        if VirtualProtect(at as _, 5, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return false;
        }
        std::ptr::copy_nonoverlapping(jump.as_ptr(), at as *mut u8, 5);
        VirtualProtect(at as _, 5, old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), at as _, 5);
    }
    tracing::warn!(
        profile = profile.name(),
        base = base_preset.name(),
        "[PROJECT ONE] preset hook installed: the title's preset is lowered before it is applied ({PRESET_ENV})"
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fields lie inside the structure, on their own size's alignment, and
    /// none overlaps another or the key the selector stores.
    #[test]
    fn fields_fit_the_structure_without_overlapping() {
        let mut spans: Vec<(usize, usize)> = FIELDS
            .iter()
            .map(|f| (f.at, f.at + f.kind.size()))
            .collect();
        spans.push((KEY_AT, KEY_AT + 4));
        spans.sort();
        for w in spans.windows(2) {
            assert!(w[0].1 <= w[1].0, "{:?} overlaps {:?}", w[0], w[1]);
        }
        for f in FIELDS {
            assert!(f.at + f.kind.size() <= SETTINGS_BYTES, "{}", f.name);
            assert_eq!(f.at % f.kind.size(), 0, "{} aligned", f.name);
        }
        assert_eq!(
            FIELDS
                .iter()
                .find(|f| f.name == "motionBlur")
                .map(|f| f.kind.size()),
            Some(1)
        );
    }

    /// Every profile names fields of the table, of their kind.
    #[test]
    fn profiles_name_known_fields_of_their_kind() {
        for p in [Profile::SafeLow, Profile::ExtremeLow, Profile::Diagnostic] {
            for (name, value) in p.values() {
                let f = FIELDS
                    .iter()
                    .find(|f| f.name == name)
                    .unwrap_or_else(|| panic!("{name}"));
                let fits = matches!(
                    (f.kind, value),
                    (Kind::Enum, Value::Enum(0..=4))
                        | (Kind::Float, Value::AtMost(_))
                        | (Kind::Bool, Value::Bool(_))
                );
                assert!(fits, "{name} {value:?}");
            }
        }
        assert!(Profile::BaseOnly.values().is_empty());
    }

    fn structure() -> Vec<u8> {
        let mut s = vec![0xAAu8; SETTINGS_BYTES];
        for f in FIELDS {
            match f.kind {
                Kind::Enum => s[f.at..f.at + 4].copy_from_slice(&2u32.to_le_bytes()),
                Kind::Float => s[f.at..f.at + 4].copy_from_slice(&1.0f32.to_le_bytes()),
                Kind::Bool => s[f.at] = 1,
            }
        }
        s
    }

    /// Extreme Low writes exactly its fields at their size and nothing else:
    /// the bytes beside the packed bool are untouched.
    #[test]
    fn a_profile_writes_its_fields_and_nothing_else() {
        let before = structure();
        let mut s = before.clone();
        let done = apply(&mut s, Profile::ExtremeLow);
        assert!(done.iter().all(|(_, o)| *o == Outcome::Written), "{done:?}");
        let mut touched = vec![false; SETTINGS_BYTES];
        for (name, _) in &done {
            let f = FIELDS.iter().find(|f| f.name == *name).unwrap();
            touched[f.at..f.at + f.kind.size()]
                .iter_mut()
                .for_each(|t| *t = true);
        }
        for i in 0..SETTINGS_BYTES {
            if !touched[i] {
                assert_eq!(s[i], before[i], "byte {i:#x} changed");
            }
        }
        assert_eq!(s[0x56c], 0);
        assert_eq!(
            &s[0x56d..0x570],
            &before[0x56d..0x570],
            "neighbours of the bool"
        );
        assert_eq!(f32::from_le_bytes(s[0x524..0x528].try_into().unwrap()), 0.5);
        assert_eq!(u32::from_le_bytes(s[0x4d4..0x4d8].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(s[0x6d0..0x6d4].try_into().unwrap()), 0);
    }

    /// A field not of the expected form (an offset that is wrong) is left
    /// alone; a float already lower is kept.
    #[test]
    fn unexpected_values_are_left_alone_and_lower_floats_kept() {
        let mut s = structure();
        s[0x4d4..0x4d8].copy_from_slice(&0x1234u32.to_le_bytes());
        s[0x56c] = 7;
        s[0x518..0x51c].copy_from_slice(&f32::NAN.to_le_bytes());
        s[0x524..0x528].copy_from_slice(&0.3f32.to_le_bytes());
        let done = apply(&mut s, Profile::SafeLow);
        let of = |n: &str| done.iter().find(|(m, _)| *m == n).map(|(_, o)| *o);
        assert_eq!(of("textureQuality"), Some(Outcome::Unexpected));
        assert_eq!(of("motionBlur"), Some(Outcome::Unexpected));
        assert_eq!(of("grassLod"), Some(Outcome::Unexpected));
        assert_eq!(of("lodScale"), Some(Outcome::Kept));
        assert_eq!(of("volumetricsQuality"), Some(Outcome::Written));
        assert_eq!(
            u32::from_le_bytes(s[0x4d4..0x4d8].try_into().unwrap()),
            0x1234
        );
        assert_eq!(s[0x56c], 7);
        assert_eq!(f32::from_le_bytes(s[0x524..0x528].try_into().unwrap()), 0.3);
        // Safe Low leaves the densities as they are.
        assert_eq!(u32::from_le_bytes(s[0x6d0..0x6d4].try_into().unwrap()), 2);
    }

    #[test]
    fn names_parse() {
        assert_eq!(Profile::parse(" Extreme-Low "), Some(Profile::ExtremeLow));
        assert_eq!(Profile::parse("diag"), Some(Profile::Diagnostic));
        assert_eq!(Profile::parse("ultra"), None);
        assert_eq!(Base::parse("SafeMode"), Some(Base::SafeMode));
        assert_eq!(Base::parse("original"), Some(Base::Original));
        assert_eq!(Base::SafeMode.key(), Some(0x7C3F_7B63));
        assert_eq!(Base::of_key(0xDEAD_BEEF), "Lockhart");
    }

    /// The selector's tail jump decodes to the lookup; a jump written back
    /// the same way reaches it again.
    #[test]
    fn the_tail_jump_reaches_the_lookup() {
        let rel = i32::from_le_bytes(SELECTOR_TAIL_BYTES[1..].try_into().unwrap());
        assert_eq!(
            (SELECTOR_TAIL_JUMP as i64 + 5 + i64::from(rel)) as usize,
            LOOKUP
        );
        assert_eq!(
            jump_bytes(SELECTOR_TAIL_JUMP, LOOKUP),
            Some(SELECTOR_TAIL_BYTES)
        );
        assert_eq!(jump_bytes(0, 0x1_0000_0000), None);
    }

    std::thread_local! {
        /// The keys the fake lookup was asked for, on this test's thread.
        static SEEN: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
        /// The keys the fake table holds.
        static TABLE: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    /// A stand-in for the title's lookup: fills the structure as a preset
    /// would if its table holds the key, else leaves it and returns 0.
    unsafe extern "win64" fn fake_lookup(_manager: *mut u8, key: u32, settings: *mut u8) -> u64 {
        SEEN.with(|s| s.borrow_mut().push(key));
        if !TABLE.with(|t| t.borrow().contains(&key)) {
            return 0;
        }
        let mut s = structure();
        s[KEY_AT..KEY_AT + 4].copy_from_slice(&key.to_le_bytes());
        // SAFETY: the test's own structure, SETTINGS_BYTES long.
        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), settings, SETTINGS_BYTES) };
        1
    }

    fn run(
        table: &[u32],
        base: Base,
        profile: Profile,
        key: u32,
    ) -> (u64, Report, Vec<u8>, Vec<u32>) {
        TABLE.with(|t| *t.borrow_mut() = table.to_vec());
        SEEN.with(|s| s.borrow_mut().clear());
        let mut s = vec![0u8; SETTINGS_BYTES];
        s[KEY_AT..KEY_AT + 4].copy_from_slice(&key.to_le_bytes());
        let setup = Setup { profile, base };
        // SAFETY: a structure of the size the hook takes; a fake lookup.
        let (found, report) = unsafe {
            choose(
                fake_lookup,
                &setup,
                std::ptr::null_mut(),
                key,
                s.as_mut_ptr(),
            )
        };
        (found, report, s, SEEN.with(|s| s.borrow().clone()))
    }

    const SAFE: u32 = 0x7C3F_7B63;
    const ANACONDA: u32 = 0xBEEF_BABE;
    const LOCKHART: u32 = 0xDEAD_BEEF;

    fn key_of(s: &[u8]) -> u32 {
        u32::from_le_bytes(s[KEY_AT..KEY_AT + 4].try_into().unwrap())
    }

    /// SafeMode in the table: looked up once, in place of the selector's
    /// key, and the profile applied after it.
    #[test]
    fn the_base_replaces_the_selectors_choice_then_the_profile_lowers_it() {
        let (found, report, s, seen) = run(
            &[SAFE, ANACONDA],
            Base::SafeMode,
            Profile::SafeLow,
            ANACONDA,
        );
        assert_eq!(found, 1);
        assert_eq!(seen, [SAFE]);
        assert_eq!(
            (report.chose, report.used, report.missing, report.failed),
            (ANACONDA, SAFE, false, false)
        );
        assert_eq!(key_of(&s), SAFE);
        assert_eq!(f32::from_le_bytes(s[0x524..0x528].try_into().unwrap()), 0.6);
        assert_eq!(s[0x56c], 0);
        assert!(
            report.done.iter().all(|(_, o)| *o == Outcome::Written),
            "{:?}",
            report.done
        );
    }

    /// SafeMode missing: the selector's own key is looked up, stored back,
    /// and the profile still lowers that preset.
    #[test]
    fn a_base_missing_from_the_table_falls_back_to_the_selectors_key() {
        let (found, report, s, seen) = run(&[ANACONDA], Base::SafeMode, Profile::SafeLow, ANACONDA);
        assert_eq!(found, 1);
        assert_eq!(seen, [SAFE, ANACONDA]);
        assert_eq!(
            (report.used, report.missing, report.failed),
            (ANACONDA, true, false)
        );
        assert_eq!(key_of(&s), ANACONDA);
        assert_eq!(u32::from_le_bytes(s[0x4d4..0x4d8].try_into().unwrap()), 0);
    }

    /// Nothing in the table: the lookup's failure is returned as it is, and
    /// no field is touched.
    #[test]
    fn a_failed_lookup_writes_nothing() {
        let (found, report, s, seen) = run(&[], Base::SafeMode, Profile::Diagnostic, LOCKHART);
        assert_eq!(found, 0);
        assert_eq!(seen, [SAFE, LOCKHART]);
        assert!(report.failed && report.done.is_empty());
        let mut expected = vec![0u8; SETTINGS_BYTES];
        expected[KEY_AT..KEY_AT + 4].copy_from_slice(&LOCKHART.to_le_bytes());
        assert_eq!(s, expected, "only the selector's own key, put back");
    }

    /// The title's own base, or a base equal to the selector's key: one
    /// lookup, with that key. The base alone changes no field.
    #[test]
    fn the_selectors_key_is_looked_up_once_and_base_only_lowers_nothing() {
        let (_, report, _, seen) = run(&[LOCKHART], Base::Original, Profile::SafeLow, LOCKHART);
        assert_eq!(seen, [LOCKHART]);
        assert_eq!(report.used, LOCKHART);
        let (_, _, _, seen) = run(&[LOCKHART], Base::Lockhart, Profile::SafeLow, LOCKHART);
        assert_eq!(seen, [LOCKHART]);
        let (_, report, s, _) = run(&[SAFE], Base::SafeMode, Profile::BaseOnly, ANACONDA);
        assert!(report.done.is_empty());
        let mut expected = structure();
        expected[KEY_AT..KEY_AT + 4].copy_from_slice(&SAFE.to_le_bytes());
        assert_eq!(s, expected, "the preset as the lookup wrote it");
    }

    /// The hook itself, called through a thunk built as for the title, at a
    /// distance a rel32 jump reaches.
    #[test]
    fn the_hook_works_through_a_thunk() {
        let lookup: Lookup = fake_lookup;
        ORIGINAL.store(lookup as usize, Ordering::Release);
        let _ = SETUP.set(Setup {
            profile: Profile::ExtremeLow,
            base: Base::SafeMode,
        });
        TABLE.with(|t| *t.borrow_mut() = vec![SAFE, LOCKHART]);
        let hook: Lookup = preset_hook;
        let here = the_hook_works_through_a_thunk as *const () as usize;
        let thunk =
            code_near(here, &thunk_bytes(hook as usize as u64)).expect("memory near the test");
        assert!(
            thunk.abs_diff(here) < 0x8000_0000,
            "within reach of a rel32 jump"
        );
        assert!(jump_bytes(here, thunk).is_some());
        // SAFETY: the thunk jumps to `preset_hook`, which has this type.
        let through: Lookup = unsafe { std::mem::transmute::<usize, Lookup>(thunk) };
        let mut s = vec![0u8; SETTINGS_BYTES];
        // SAFETY: a structure of the size the hook takes.
        let found = unsafe { through(std::ptr::null_mut(), LOCKHART, s.as_mut_ptr()) };
        assert_eq!(found, 1);
        assert_eq!(SEEN.with(|s| s.borrow().clone()), [SAFE]);
        assert_eq!(key_of(&s), SAFE);
        assert_eq!(f32::from_le_bytes(s[0x524..0x528].try_into().unwrap()), 0.5);
        assert_eq!(s[0x56c], 0);
    }
}
