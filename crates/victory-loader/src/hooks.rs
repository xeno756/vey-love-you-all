//! Opt-in compatibility hooks for known title builds.
//!
//! Only per-build offsets and short byte signatures live here (asset policy:
//! nothing of the title itself). Every hook checks the build's hash and the
//! exact bytes it replaces, and does nothing otherwise.

use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// First 16 hex digits of the BLAKE3 of the Series X 1.0.2.1 executable.
const BUILD_1_0_2_1: &str = "b99f876b74d4a77f";

/// Environment variable that turns the watchdog hook on.
pub(crate) const NO_WATCHDOG_ENV: &str = "VICTORY_NO_WATCHDOG";

/// One code patch: `rva` must hold exactly `expect` before `new` is written.
struct Patch<'a> {
    what: &'static str,
    rva: usize,
    expect: &'a [u8],
    new: &'a [u8],
}

/// The title's watchdog thread ends the process on purpose when its main
/// thread has not advanced for about 30 one-second ticks. Two places do it:
/// the handler it calls (`mov dword ptr [0], imm32`) and, when the handler
/// returns or is unset, a fallback right after the call (`mov dword ptr [0],
/// 0; int3`). Jumping over the whole block returns the loop to its normal
/// path; the first patch makes the handler harmless if something else calls it.
const WATCHDOG: &[Patch<'static>] = &[
    Patch {
        what: "watchdog: skip the call and the fallback fault",
        rva: 0x2215c27,
        // mov rax, [rip + disp32] (the handler pointer)
        expect: &[0x48, 0x8B, 0x05, 0x1A, 0x07, 0xFA, 0x04],
        // jmp +0x18 (to the loop's normal continuation), then NOPs
        new: &[0xEB, 0x18, 0x90, 0x90, 0x90, 0x90, 0x90],
    },
    Patch {
        what: "watchdog: handler's deliberate fault",
        rva: 0x23cf683,
        expect: &[0xC7, 0x04, 0x25, 0, 0, 0, 0, 0x21, 0xD4, 0x8B, 0x4D],
        // an eleven-byte NOP
        new: &[0x66, 0x66, 0x2E, 0x0F, 0x1F, 0x84, 0, 0, 0, 0, 0],
    },
];

/// Applies the watchdog patches whose bytes match. Returns how many applied.
///
/// # Safety
/// `base` is the mapped image whose original BLAKE3 is `hash`, and no title
/// code is running yet.
pub(crate) unsafe fn disarm_watchdog(base: usize, hash: &blake3::Hash) -> usize {
    if !hash.to_hex().starts_with(BUILD_1_0_2_1) {
        return 0;
    }
    WATCHDOG
        .iter()
        // SAFETY: forwarded from this function's contract.
        .filter(|p| unsafe { apply(base, p) })
        .count()
}

/// Environment variable that pins the title's internal render scale
/// (`0.2..=1.0` of the output size). Testing lever, off by default.
pub(crate) const RENDER_SCALE_ENV: &str = "VICTORY_RENDER_SCALE";
/// Environment variable that makes the title use its own temporal upscaler
/// on every console profile (Series S otherwise picks FSR).
pub(crate) const OWN_UPSCALER_ENV: &str = "VICTORY_OWN_UPSCALER";

/// A patch whose bytes are computed at load time.
struct OwnedPatch {
    what: &'static str,
    rva: usize,
    expect: Vec<u8>,
    new: Vec<u8>,
}

/// The upscaler choice: `mov eax, 6` (own upscaler) then `cmovne eax, ecx`
/// (FSR when the profile asks for it). NOPing the cmovne keeps 6.
const OWN_UPSCALER: Patch<'static> = Patch {
    what: "upscaler: always the title's own",
    rva: 0x7e9426,
    expect: &[0x0F, 0x45, 0xC1],
    new: &[0x0F, 0x1F, 0x00],
};

/// Read-only float constants of this build usable as the FSR scale (D1):
/// value, RVA.
const FSR_SCALES: [(f32, usize); 4] = [
    (0.5, 0x5fc6edc),
    (0.4, 0x5fc6f90),
    (1.0 / 3.0, 0x615a240),
    (0.75, 0x6159b04),
];

/// The render-scale patches for `scale`, with the image at `base`:
/// - the own upscaler's minimum scale (an immediate, 2/3 by default) set to
///   `scale` and its maximum made equal to the minimum (`vmaxss` → `vmovss`),
///   so the scale is fixed;
/// - FSR's scale (a load of 2/3 on Series S) pointed at a constant equal to
///   `scale`, when this build has one, and its maximum size made the scaled
///   size (vtable slot 11 → slot 10's function).
fn render_scale_patches(base: usize, scale: f32, fsr: bool) -> Vec<OwnedPatch> {
    let mut patches = vec![
        OwnedPatch {
            what: "render scale: own upscaler minimum",
            rva: 0x28f1e7e,
            expect: vec![0xAB, 0xAA, 0x2A, 0x3F],
            new: scale.to_le_bytes().to_vec(),
        },
        OwnedPatch {
            what: "render scale: own upscaler maximum = minimum",
            rva: 0x28f1e87,
            expect: vec![0xC5, 0xFA, 0x5F, 0x86, 0x90, 0, 0, 0],
            new: vec![0xC5, 0xFA, 0x10, 0x86, 0x90, 0, 0, 0],
        },
    ];
    if !fsr {
        return patches;
    }
    match FSR_SCALES.iter().find(|(v, _)| (v - scale).abs() < 1e-3) {
        Some(&(_, constant)) => {
            // vmovss xmm0, [rip + disp32]; the next instruction is at +8.
            const LOAD: usize = 0x28ef6b2;
            let disp = (constant as i64 - (LOAD as i64 + 8)) as i32;
            let mut new = vec![0xC5, 0xFA, 0x10, 0x05];
            new.extend_from_slice(&disp.to_le_bytes());
            patches.push(OwnedPatch {
                what: "render scale: FSR scale",
                rva: LOAD,
                expect: vec![0xC5, 0xFA, 0x10, 0x05, 0xA2, 0xE2, 0x86, 0x03],
                new,
            });
            patches.push(OwnedPatch {
                what: "render scale: FSR maximum = scaled size",
                rva: 0x6300ba8,
                expect: ((base + 0x28ef6f0) as u64).to_le_bytes().to_vec(),
                new: ((base + 0x28ef680) as u64).to_le_bytes().to_vec(),
            });
        }
        None => tracing::warn!(
            scale,
            "no FSR constant for this scale (0.5, 0.4, 1/3, 0.75); FSR keeps its own (use {OWN_UPSCALER_ENV}=1)"
        ),
    }
    patches
}

/// The scale `VICTORY_RENDER_SCALE` asks for, if valid.
fn parse_scale(text: &str) -> Option<f32> {
    let v: f32 = text.trim().parse().ok()?;
    (0.2..=1.0).contains(&v).then_some(v)
}

/// Applies the resolution levers asked for in the environment
/// ([`RENDER_SCALE_ENV`], [`OWN_UPSCALER_ENV`]). Returns how many applied.
///
/// # Safety
/// As `disarm_watchdog`.
pub(crate) unsafe fn resolution_levers(base: usize, hash: &blake3::Hash) -> usize {
    let scale = std::env::var(RENDER_SCALE_ENV).ok();
    let own = std::env::var_os(OWN_UPSCALER_ENV).is_some_and(|v| v != "0");
    if scale.is_none() && !own {
        return 0;
    }
    // Every image the loader maps comes here (the title's DLLs too); only
    // the 1.0.2.1 executable has the patched places.
    if !hash.to_hex().starts_with(BUILD_1_0_2_1) {
        tracing::debug!("resolution levers: not the 1.0.2.1 executable; nothing patched");
        return 0;
    }
    let mut applied = 0;
    if own {
        // SAFETY: forwarded from this function's contract.
        applied += usize::from(unsafe { apply(base, &OWN_UPSCALER) });
    }
    if let Some(text) = scale {
        match parse_scale(&text) {
            Some(scale) => {
                // With the own upscaler forced, FSR is left as it is.
                for p in render_scale_patches(base, scale, !own) {
                    let p = Patch { what: p.what, rva: p.rva, expect: &p.expect, new: &p.new };
                    // SAFETY: forwarded from this function's contract.
                    applied += usize::from(unsafe { apply(base, &p) });
                }
            }
            None => tracing::warn!(text, "{RENDER_SCALE_ENV} wants a number in 0.2..=1.0"),
        }
    }
    applied
}

/// Environment variable that gives the size of the picture the title makes
/// (`WxH`, for instance `1920x1080`), in place of the one it picks for the
/// console (3840x2160 on a Series X). Everything the title sizes from it
/// follows: its back buffer, its interface, and its 3D picture, which is
/// that size times its render scale ([`RENDER_SCALE_ENV`]; two thirds
/// unless set). (PROJECT ONE.)
pub(crate) const OUTPUT_SIZE_ENV: &str = "VICTORY_OUTPUT_SIZE";

/// The title's display settings are reached through one small function
/// that every reader calls: a jump to its body, then padding up to it.
const SETTINGS_GETTER: usize = 0x504c1b0;
/// The getter's body: no argument, the settings returned.
const SETTINGS_BODY: usize = 0x504c1c0;
/// Where the settings hold the width and the height of the picture: what
/// the title gives its device as its size, and what it derives the sizes
/// of its render targets from.
const SETTINGS_SIZE: [usize; 2] = [0x204, 0x208];

/// The size asked for ([`OUTPUT_SIZE_ENV`]).
static OUTPUT_SIZE: [std::sync::atomic::AtomicU32; 2] = [const { std::sync::atomic::AtomicU32::new(0) }; 2];
/// The getter's body in the mapped image.
static SETTINGS_ORIGINAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// The size the title had chosen itself, as first seen (0: not yet).
static TITLE_SIZE: [std::sync::atomic::AtomicU32; 2] = [const { std::sync::atomic::AtomicU32::new(0) }; 2];

/// What the getter's callers reach instead of its body: the settings, with
/// the size asked for written over the title's own each time, so that
/// nothing the title does to its settings meanwhile brings its size back.
extern "system" fn settings_with_output_size() -> *mut u8 {
    use std::sync::atomic::Ordering::Relaxed;
    // SAFETY: set before the patch that brings callers here; the title's
    // own function, which takes nothing and returns its settings.
    let original: extern "system" fn() -> *mut u8 = unsafe { std::mem::transmute(SETTINGS_ORIGINAL.load(Relaxed)) };
    let settings = original();
    if settings.is_null() {
        return settings;
    }
    let size = [OUTPUT_SIZE[0].load(Relaxed), OUTPUT_SIZE[1].load(Relaxed)];
    // SAFETY: the two fields of the settings, which the title reads and
    // writes as plain numbers.
    unsafe {
        let at = |i: usize| settings.add(SETTINGS_SIZE[i]).cast::<u32>();
        let own = [at(0).read_unaligned(), at(1).read_unaligned()];
        if own != size {
            // The title's own choice, once, for the log: what it asked for.
            if own[0] != 0 && TITLE_SIZE[0].swap(own[0], Relaxed) == 0 {
                TITLE_SIZE[1].store(own[1], Relaxed);
                tracing::info!(
                    title_width = own[0],
                    title_height = own[1],
                    width = size[0],
                    height = size[1],
                    "resolution: the title chose its own size; the size asked for is put in its place ({OUTPUT_SIZE_ENV})"
                );
            }
            at(0).write_unaligned(size[0]);
            at(1).write_unaligned(size[1]);
        }
    }
    settings
}

/// Where the title makes its back buffers it sets memory aside for them at
/// its own size (64 MiB on a Series X) and checks that they took all of it
/// but 2 MiB: a breakpoint otherwise, which ends it. Smaller back buffers
/// take less (1920x1080 leaves 49 MiB, 1280x720 56 MiB: both runs of
/// 25/11 ended there, 3.5 s after their start). The branch over the
/// breakpoint is taken whatever is left.
const BACK_BUFFER_MEMORY: Patch<'static> = Patch {
    what: "output size: the back buffers' memory check",
    rva: 0x247af2b,
    // jbe +1 ; int3
    expect: &[0x76, 0x01, 0xCC],
    // jmp +1 ; int3
    new: &[0xEB, 0x01, 0xCC],
};

/// The title's executable in memory while it is given a size that is not
/// its own: where it starts and ends (0: no such size).
static CHECKED_IMAGE: [std::sync::atomic::AtomicUsize; 2] = [const { std::sync::atomic::AtomicUsize::new(0) }; 2];

/// Whether a breakpoint at `rip` is one of the title's own checks to pass
/// over: the title was given a size in place of its own
/// ([`OUTPUT_SIZE_ENV`]), and the breakpoint is in its executable. Its
/// checks compare what it made with what its own size takes (memory set
/// aside, counts), and stop it where they differ; the one known is patched
/// ([`BACK_BUFFER_MEMORY`]), any other is passed over here and said, the
/// first times, with its place, so that it can be looked at.
/// `VICTORY_TITLE_CHECKS=1` keeps them all fatal.
pub(crate) fn title_check(rip: usize) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    let (start, end) = (CHECKED_IMAGE[0].load(Relaxed), CHECKED_IMAGE[1].load(Relaxed));
    if start == 0 || !(start..end).contains(&rip) {
        return false;
    }
    static SAID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if SAID.fetch_add(1, Relaxed) < 64 {
        tracing::warn!(
            rva = format_args!("{:#x}", rip - start),
            "resolution: a check of the title's own stopped it with a size that is not its own; passed over"
        );
    }
    true
}

/// The size `VICTORY_OUTPUT_SIZE` asks for, if it is one a title can be
/// given: `WxH`, 640x360 to 3840x2160, both even.
fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.trim().split_once(['x', 'X'])?;
    let (w, h) = (w.trim().parse::<u32>().ok()?, h.trim().parse::<u32>().ok()?);
    ((640..=3840).contains(&w) && (360..=2160).contains(&h) && w % 2 == 0 && h % 2 == 0).then_some((w, h))
}

/// The patch that sends the getter's callers to `hook`: over the jump and
/// its padding, `mov rax, hook` then `jmp rax`.
fn output_size_patch(hook: usize) -> OwnedPatch {
    let mut expect = vec![0xE9, 0x0B, 0, 0, 0];
    expect.resize(16, 0xCC);
    let mut new = vec![0x48, 0xB8];
    new.extend_from_slice(&(hook as u64).to_le_bytes());
    new.extend_from_slice(&[0xFF, 0xE0]);
    new.resize(16, 0xCC);
    OwnedPatch {
        what: "output size: the settings getter",
        rva: SETTINGS_GETTER,
        expect,
        new,
    }
}

/// Makes the title take the size [`OUTPUT_SIZE_ENV`] asks for as the size
/// of its picture. Returns whether it applied.
///
/// # Safety
/// As `disarm_watchdog`.
pub(crate) unsafe fn output_size(base: usize, size: usize, hash: &blake3::Hash) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    let Some(text) = std::env::var(OUTPUT_SIZE_ENV).ok().filter(|v| !v.trim().is_empty()) else {
        return false;
    };
    if !hash.to_hex().starts_with(BUILD_1_0_2_1) {
        return false;
    }
    let Some((width, height)) = parse_size(&text) else {
        tracing::warn!(text, "{OUTPUT_SIZE_ENV} wants WxH, 640x360 to 3840x2160");
        return false;
    };
    OUTPUT_SIZE[0].store(width, Relaxed);
    OUTPUT_SIZE[1].store(height, Relaxed);
    SETTINGS_ORIGINAL.store(base + SETTINGS_BODY, Relaxed);
    let p = output_size_patch(settings_with_output_size as *const () as usize);
    let p = Patch { what: p.what, rva: p.rva, expect: &p.expect, new: &p.new };
    // SAFETY: forwarded from this function's contract.
    let applied = unsafe { apply(base, &p) };
    if applied {
        tracing::warn!(width, height, "resolution: the title is given this size as the size of its picture ({OUTPUT_SIZE_ENV})");
        // SAFETY: forwarded from this function's contract.
        unsafe { apply(base, &BACK_BUFFER_MEMORY) };
        if std::env::var_os("VICTORY_TITLE_CHECKS").is_none_or(|v| v != "1") {
            CHECKED_IMAGE[1].store(base + size, Relaxed);
            CHECKED_IMAGE[0].store(base, Relaxed);
        }
    }
    applied
}

/// # Safety
/// See `disarm_watchdog`.
unsafe fn apply(base: usize, p: &Patch<'_>) -> bool {
    let at = (base + p.rva) as *mut u8;
    // SAFETY: the offset is inside the mapped .text of this exact build.
    let now = unsafe { std::slice::from_raw_parts(at, p.expect.len()) };
    if now != p.expect || p.new.len() != p.expect.len() {
        tracing::warn!(what = p.what, "hook skipped: the bytes are not the expected ones");
        return false;
    }
    let mut old = 0;
    // SAFETY: changing the protection of those bytes only, restored below.
    if unsafe { VirtualProtect(at as _, p.new.len(), PAGE_EXECUTE_READWRITE, &mut old) } == 0 {
        return false;
    }
    // SAFETY: writable now; the bytes are the ones checked above.
    unsafe {
        std::ptr::copy_nonoverlapping(p.new.as_ptr(), at, p.new.len());
        VirtualProtect(at as _, p.new.len(), old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), at as _, p.new.len());
    }
    tracing::info!(what = p.what, "hook applied");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_outside_the_floor_are_refused() {
        assert_eq!(parse_scale("0.5"), Some(0.5));
        assert_eq!(parse_scale(" 1 "), Some(1.0));
        assert_eq!(parse_scale("0.1"), None);
        assert_eq!(parse_scale("1.5"), None);
        assert_eq!(parse_scale("half"), None);
    }

    #[test]
    fn half_scale_patches_both_upscalers() {
        let base = 0x7ff0_0000_0000;
        let p = render_scale_patches(base, 0.5, true);
        assert_eq!(p.len(), 4);
        assert_eq!(p[0].new, 0.5f32.to_le_bytes());
        // The FSR load now reads 0.5 at 0x5fc6edc: disp from the next instruction.
        assert_eq!(p[2].new, [0xC5, 0xFA, 0x10, 0x05, 0x22, 0x78, 0x6D, 0x03]);
        assert_eq!(p[3].new, ((base + 0x28ef680) as u64).to_le_bytes());
        assert!(p.iter().all(|p| p.new.len() == p.expect.len()));
    }

    /// A breakpoint is passed over only in the executable of a title given
    /// a size that is not its own.
    #[test]
    fn only_the_titles_own_checks_are_passed_over() {
        use std::sync::atomic::Ordering::Relaxed;
        assert!(!title_check(0x7ff0_0000_1000), "no size given: every breakpoint is fatal");
        CHECKED_IMAGE[1].store(0x7ff0_0b00_0000, Relaxed);
        CHECKED_IMAGE[0].store(0x7ff0_0000_0000, Relaxed);
        assert!(title_check(0x7ff0_0247_af2d));
        assert!(!title_check(0x7ff0_0b00_0000), "past the executable");
        assert!(!title_check(0x7fe0_0000_1000), "before it");
        CHECKED_IMAGE[0].store(0, Relaxed);
        CHECKED_IMAGE[1].store(0, Relaxed);
    }

    /// The back buffers' memory check: the same three bytes with the
    /// branch always taken.
    #[test]
    fn the_back_buffer_check_keeps_its_length() {
        assert_eq!(BACK_BUFFER_MEMORY.expect.len(), BACK_BUFFER_MEMORY.new.len());
        assert_eq!(BACK_BUFFER_MEMORY.new, [0xEB, 0x01, 0xCC]);
    }

    #[test]
    fn output_sizes_are_read_as_width_by_height() {
        assert_eq!(parse_size("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_size(" 1280X720 "), Some((1280, 720)));
        assert_eq!(parse_size("3840x2160"), Some((3840, 2160)));
        assert_eq!(parse_size("1920"), None);
        assert_eq!(parse_size("320x180"), None, "too small for the title");
        assert_eq!(parse_size("7680x4320"), None);
        assert_eq!(parse_size("1921x1080"), None, "odd");
    }

    /// The getter's sixteen bytes become a jump to the hook's own address.
    #[test]
    fn the_settings_getter_is_sent_to_the_hook() {
        let p = output_size_patch(0x7ff8_1234_5678);
        assert_eq!(p.rva, 0x504c1b0);
        assert_eq!(p.expect.len(), 16);
        assert_eq!(p.new.len(), 16);
        assert_eq!(&p.new[..2], [0x48, 0xB8]);
        assert_eq!(p.new[2..10], 0x7ff8_1234_5678u64.to_le_bytes());
        assert_eq!(&p.new[10..], [0xFF, 0xE0, 0xCC, 0xCC, 0xCC, 0xCC]);
    }

    /// The hook hands out the title's settings with the size asked for in
    /// the two fields, whatever the title had put there, and nothing else
    /// changed.
    #[test]
    fn the_hook_puts_the_size_asked_for_in_the_settings() {
        use std::sync::atomic::Ordering::Relaxed;
        static SETTINGS: std::sync::Mutex<[u8; 0x210]> = std::sync::Mutex::new([0xAB; 0x210]);
        extern "system" fn title_getter() -> *mut u8 {
            let mut s = SETTINGS.lock().unwrap();
            s[0x204..0x208].copy_from_slice(&3840u32.to_le_bytes());
            s[0x208..0x20c].copy_from_slice(&2160u32.to_le_bytes());
            s.as_mut_ptr()
        }
        OUTPUT_SIZE[0].store(1920, Relaxed);
        OUTPUT_SIZE[1].store(1080, Relaxed);
        SETTINGS_ORIGINAL.store(title_getter as *const () as usize, Relaxed);
        let got = settings_with_output_size();
        let s = SETTINGS.lock().unwrap();
        assert_eq!(got.cast_const(), s.as_ptr());
        assert_eq!(s[0x204..0x208], 1920u32.to_le_bytes());
        assert_eq!(s[0x208..0x20c], 1080u32.to_le_bytes());
        assert!(s[..0x204].iter().chain(&s[0x20c..]).all(|&b| b == 0xAB));
        assert_eq!([TITLE_SIZE[0].load(Relaxed), TITLE_SIZE[1].load(Relaxed)], [3840, 2160]);
    }

    #[test]
    fn odd_scales_patch_only_the_own_upscaler() {
        assert_eq!(render_scale_patches(0, 0.6, true).len(), 2);
        assert_eq!(render_scale_patches(0, 0.5, false).len(), 2);
    }
}