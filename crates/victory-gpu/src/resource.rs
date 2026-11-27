//! Heaps, committed and placed resources, and resource sizes.
//!
//! Heaps and resources are host memory and a resource's GPU address is its
//! CPU address, as on the console; textures also get a host copy, and
//! buffers a host copy when a draw reads them (`shadow`).
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::c_void;
use std::ptr::NonNull;

use victory_formats::texture;

use crate::object::{
    self, E_INVALIDARG, E_OUTOFMEMORY, HEAP, HRESULT, Iid, Object, RESOURCE, S_OK, Table, This,
    guid,
};

/// The interfaces every device child also answers for.
pub(crate) const PAGEABLE_IIDS: [Iid; 3] = [
    guid(
        0x63ee58fb,
        0x1268,
        0x4835,
        [0x86, 0xda, 0xf0, 0x08, 0xce, 0x62, 0xf0, 0xd6],
    ),
    guid(
        0x905db94b,
        0xa00c,
        0x4140,
        [0x9d, 0xf5, 0x2b, 0x64, 0xca, 0x9e, 0xa3, 0x57],
    ),
    guid(
        0xc4fec28f,
        0x7966,
        0x4e95,
        [0x9f, 0x94, 0xf4, 0x31, 0xcb, 0x56, 0xc3, 0xb8],
    ),
];
const HEAP_IIDS: [Iid; 4] = [
    guid(
        0x6b3b2502,
        0x6e51,
        0x45b3,
        [0x90, 0xee, 0x98, 0x84, 0x26, 0x5e, 0x8d, 0xf3],
    ),
    PAGEABLE_IIDS[0],
    PAGEABLE_IIDS[1],
    PAGEABLE_IIDS[2],
];
const RESOURCE_IIDS: [Iid; 4] = [
    guid(
        0x696442be,
        0xa72e,
        0x4059,
        [0xbc, 0x79, 0x5b, 0x5c, 0x98, 0x04, 0x0f, 0xad],
    ),
    PAGEABLE_IIDS[0],
    PAGEABLE_IIDS[1],
    PAGEABLE_IIDS[2],
];

const KIB_64: u64 = 64 << 10;
const MIB_4: u64 = 4 << 20;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeapProperties {
    kind: u32,
    cpu_pages: u32,
    pool: u32,
    creation_nodes: u32,
    visible_nodes: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeapDesc {
    size: u64,
    properties: HeapProperties,
    alignment: u64,
    flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResourceDesc {
    dimension: u32,
    alignment: u64,
    width: u64,
    height: u32,
    depth_or_array_size: u16,
    mip_levels: u16,
    format: u32,
    sample_count: u32,
    sample_quality: u32,
    layout: u32,
    flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AllocationInfo {
    size: u64,
    alignment: u64,
}

const _: () = assert!(size_of::<HeapProperties>() == 20);
const _: () = assert!(size_of::<HeapDesc>() == 48);
const _: () = assert!(size_of::<ResourceDesc>() == 56);

const BUFFER: u32 = 1;
const TEXTURE_3D: u32 = 4;

/// Bytes per element and the element's width in texels (4 for block
/// compression), by DXGI format number. `None` for 1-bit and video
/// formats. The console adds 116/117 (10-bit float color), 118-120 (16-bit
/// depth with 8-bit stencil, two planes: 3 bytes per texel), 189 (signed
/// 10-bit color), 190 (two 4-bit channels) and 191 (24/8 color).
pub(crate) fn element(format: u32) -> Option<(u64, u64)> {
    Some(match format {
        1..=4 => (16, 1),
        5..=8 => (12, 1),
        9..=22 => (8, 1),
        23..=47 | 67 | 87..=93 | 116 | 117 | 189 | 191 => (4, 1),
        118..=120 => (3, 1),
        48..=59 | 68 | 69 | 85 | 86 | 115 => (2, 1),
        60..=65 | 190 => (1, 1),
        70..=72 | 79..=81 => (8, 4),
        73..=78 | 82..=84 | 94..=99 => (16, 4),
        _ => return None,
    })
}

/// A texture's layout in console memory, as the console's layout library
/// gives it (`victory-hle`'s `xg` serves the same): the description's
/// layout field is `0x100 | swizzle mode`, 1 linear, 0 the library's pick.
pub(crate) fn texture_layout(desc: &ResourceDesc) -> Option<texture::Texture> {
    use texture::{Dimension, Swizzle, TextureDesc};
    const TEXTURE_1D: u32 = 2;
    const TEXTURE_2D: u32 = 3;
    const RENDER_TARGET: u32 = 1;
    const DEPTH: u32 = 2;
    let dimension = match desc.dimension {
        TEXTURE_1D => Dimension::Tex1d,
        TEXTURE_2D => Dimension::Tex2d,
        TEXTURE_3D => Dimension::Tex3d,
        _ => return None,
    };
    let swizzle = match desc.layout {
        0 if dimension == Dimension::Tex1d => Swizzle::LINEAR,
        0 if desc.flags & DEPTH != 0 => Swizzle::SW_64KB_Z_X,
        0 if desc.flags & RENDER_TARGET != 0 => Swizzle::SW_64KB_R_X,
        0 => Swizzle::SW_4KB_S,
        1 => Swizzle::LINEAR,
        l if l & !0xff == 0x100 => Swizzle(l & 0xff),
        _ => return None,
    };
    texture::Texture::new(&TextureDesc {
        dimension,
        width: u32::try_from(desc.width).ok()?,
        height: desc.height,
        depth_or_array: u32::from(desc.depth_or_array_size),
        mips: u32::from(desc.mip_levels),
        format: desc.format,
        samples: desc.sample_count.max(1),
        swizzle,
    })
    .ok()
}

/// Bytes and alignment a resource takes: buffers their width (64 KiB
/// aligned), textures their console layout (formats it does not know:
/// every mip of every array slice and sample, packed, 64 KiB aligned),
/// rounded up to the alignment.
fn footprint(desc: &ResourceDesc) -> Option<AllocationInfo> {
    if desc.dimension != BUFFER
        && let Some(t) = texture_layout(desc)
    {
        // As the console: the layout's own alignment unless more is asked
        // (4 KiB for 4 KiB modes, where PC says 64 KiB).
        // The title keeps these sizes in its texture tables, and its streaming
        // asserts when they disagree with the streamed layout.
        let alignment = desc.alignment.max(t.alignment);
        if !alignment.is_power_of_two() {
            return None;
        }
        return Some(AllocationInfo {
            size: t.size.checked_next_multiple_of(alignment)?,
            alignment,
        });
    }
    let bytes = if desc.dimension == BUFFER {
        desc.width
    } else {
        let (element, block) = element(desc.format)?;
        let (depth, slices) = if desc.dimension == TEXTURE_3D {
            (u64::from(desc.depth_or_array_size), 1)
        } else {
            (1, u64::from(desc.depth_or_array_size))
        };
        let height = u64::from(desc.height);
        let mips = mip_count(desc);
        let mut chain = 0u64;
        for mip in 0..mips.min(64) {
            let w = (desc.width >> mip).max(1).div_ceil(block);
            let h = (height >> mip).max(1).div_ceil(block);
            let d = (depth >> mip).max(1);
            chain = chain.checked_add(w.checked_mul(h)?.checked_mul(d)?.checked_mul(element)?)?;
        }
        chain
            .checked_mul(slices)?
            .checked_mul(u64::from(desc.sample_count.max(1)))?
    };
    if bytes == 0 {
        return None;
    }
    let minimum = if desc.sample_count > 1 { MIB_4 } else { KIB_64 };
    let alignment = desc.alignment.max(minimum);
    if !alignment.is_power_of_two() {
        return None;
    }
    Some(AllocationInfo {
        size: bytes.checked_next_multiple_of(KIB_64)?,
        alignment,
    })
}

/// A format as the host has it. The console's 16-bit depth with stencil
/// (and its typeless views) has no PC format: the 24-bit ones stand in.
pub(crate) fn pc_format(format: u32) -> u32 {
    match format {
        118 => 45,
        119 => 46,
        120 => 47,
        other => other,
    }
}

/// The format of a texture the host GPU renders to or writes as unordered
/// access, for the title's `format`. The console's GPU does both with the
/// 16-bit colour formats B5G5R5A1 (86) and B4G4R4A4 (115), and
/// the title has full-screen targets in both; a host that refuses either use
/// (NVIDIA) refused the whole texture, so every pass that wrote it drew
/// nothing. There 8-bit RGBA stands in: the same channels in the same
/// order, every 4-bit level exact and every 5-bit level within 1/510.
/// The shared-exponent R9G9B9E5 (67) is neither a render target nor an
/// unordered access format on PC, and the title's Series S mode makes a
/// 1920x1080 target of it with both uses (console flags 0x300005), which
/// the RX 5600 XT refused (E_INVALIDARG): R11G11B10_FLOAT stands in, the
/// same 4 bytes a texel, unsigned up to 64512 for 65408, with 6, 6 and 5 bits
/// of mantissa per channel for 9 (R16G16B16A16_FLOAT keeps more, at 8).
/// Textures the title only samples stay R9G9B9E5.
/// The title's pipelines and views of such targets name the stand-in too
/// ([`view_format`], the host view checks).
/// shortcut: copies between such a target and a texture the title fills
/// itself in its own format, and read backs of such a target, are skipped
/// ([`stands_in`]); convert them if a pass turns out to need one.
pub(crate) fn target_format(format: u32) -> u32 {
    const STAND_INS: [(u32, u32); 3] = [(86, 28), (115, 28), (67, 26)];
    let Some(i) = STAND_INS.iter().position(|&(f, _)| f == format) else {
        return format;
    };
    #[cfg(windows)]
    {
        use windows::Win32::Graphics::Direct3D12::*;
        static REFUSED: std::sync::OnceLock<[bool; 3]> = std::sync::OnceLock::new();
        let refused = REFUSED.get_or_init(|| {
            STAND_INS.map(|(f, stand_in)| {
                let Some(host) = crate::host::host() else {
                    return false;
                };
                let mut support = D3D12_FEATURE_DATA_FORMAT_SUPPORT {
                    Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT(f as i32),
                    ..Default::default()
                };
                // SAFETY: a plain query into our own structure.
                let asked = unsafe {
                    host.device.CheckFeatureSupport(
                        D3D12_FEATURE_FORMAT_SUPPORT,
                        (&raw mut support).cast(),
                        size_of_val(&support) as u32,
                    )
                };
                let both = D3D12_FORMAT_SUPPORT1_RENDER_TARGET.0
                    | D3D12_FORMAT_SUPPORT1_TYPED_UNORDERED_ACCESS_VIEW.0;
                let refused = asked.is_err() || support.Support1.0 & both != both;
                if refused {
                    tracing::info!(format = f, stand_in, "the host does not render to this format: the stand-in does");
                }
                refused
            })
        });
        if refused[i] {
            return STAND_INS[i].1;
        }
    }
    format
}

/// Whether the host copy of the title texture `this` is in a stand-in
/// format ([`target_format`]): its texels are not laid out as the title's.
#[cfg(windows)]
pub(crate) fn stands_in(this: This) -> bool {
    if this.is_null() {
        return false;
    }
    let own = pc_format(resource(this).desc.format);
    target_format(own) != own
        && host_resource(this)
            // SAFETY: a live resource.
            .is_some_and(|host| unsafe { host.GetDesc() }.Format.0 as u32 == target_format(own))
}

/// Depth textures the title also asks unordered access for (the console
/// allows both; the host copy loses the unordered access): how many were
/// made, and how often such a texture was bound as the depth target,
/// cleared as depth, and asked for an unordered access view the host could
/// not give. Measured to decide what such textures need (a colour twin, or
/// to be colour textures altogether).
pub(crate) static DEPTH_UNORDERED: [std::sync::atomic::AtomicU64; 4] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 4];

/// Whether the title asked for both depth and unordered access for `this`.
#[cfg(windows)]
pub(crate) fn depth_and_unordered(this: This) -> bool {
    !this.is_null() && resource(this).desc.flags & 6 == 6
}

/// The colour twin of a depth texture the title also writes as unordered
/// access. The console allows both on one texture; the host gives a depth
/// texture no unordered access, so shaders write a colour texture of the
/// same size and format family instead, and one is copied into the other
/// when a side is used after the other was written
/// (`HostList::twin_use`).
/// shortcut: whole textures are copied, not the mips and slices written.
/// A depth texture with a stencil plane gets a twin for each plane (
/// the title's tiled lighting writes each pixel's material class into
/// the depth plane and masks into the stencil plane, then draws each class
/// with the depth test "equal"): the stencil plane's is copied through a
/// buffer, the depth plane's is drawn into the plane by a pass of ours
/// (`HostList::pass_plane`). Unordered access views of a multisampled
/// texture stay null.
#[cfg(windows)]
pub(crate) struct Twin {
    /// The colour texture and the state it is in, as a host copy's.
    pub(crate) colour: std::sync::Arc<HostTexture>,
    /// Which side lacks what the other was written last, after the lists
    /// submitted so far: bit 0 the depth texture, bit 1 the twin.
    pub(crate) behind: std::sync::atomic::AtomicU32,
    /// For a depth texture with a stencil plane, `colour` is the twin of
    /// that plane (8 bits a texel) and this the buffer the two are copied
    /// through: no colour format shares a family with a depth-stencil
    /// format, but the host copies between a buffer and one plane.
    pub(crate) through: Option<windows::Win32::Graphics::Direct3D12::ID3D12Resource>,
    /// For a depth texture with a stencil plane: the twin of its depth
    /// plane, 16 bits a texel for the console's 16-bit depth-stencil
    /// formats and a 32-bit float otherwise. No host colour format holds
    /// the bits of a 24-bit depth plane, so the two are not copied: a pass
    /// draws the twin into the plane.
    pub(crate) plane: Option<std::sync::Arc<HostTexture>>,
}

#[cfg(windows)]
impl Drop for Twin {
    fn drop(&mut self) {
        TWINS[0].fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Colour twins alive, depth textures with unordered access left without
/// one, and the copies recorded into depth textures and into twins since
/// the last frame-rate line.
#[cfg(windows)]
pub(crate) static TWINS: [std::sync::atomic::AtomicU64; 4] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 4];

/// Depth planes drawn from their twins by the pass since the last
/// frame-rate line; and since the start, how often a depth plane's twin
/// lacked what was written to the plane (not given to it), and how often a
/// depth plane was used in the list that had written its twin, before the
/// pass (which runs when the list closes).
#[cfg(windows)]
pub(crate) static PLANES: [std::sync::atomic::AtomicU64; 3] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 3];

/// The colour twin of title texture `this`, if it has one.
#[cfg(windows)]
pub(crate) fn twin(this: This) -> Option<std::sync::Arc<Twin>> {
    if this.is_null() {
        return None;
    }
    hosted(this, true).and_then(|made| made.1)
}

/// The twin's texture, for the unordered access views of `this`.
#[cfg(windows)]
pub(crate) fn twin_resource<'a>(
    this: This,
) -> Option<&'a windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    if this.is_null() {
        return None;
    }
    let (_, twin) = hosted(this, true)?;
    // SAFETY: as in `host_resource`: the twin lives as long as the host
    // copy, which a depth texture keeps until the title releases it.
    twin.map(|t| unsafe { &*std::ptr::from_ref(&t.colour.texture) })
}

/// The texture of the twin of the depth plane of `this`, a depth texture
/// with a stencil plane, for the unordered access views of that plane.
#[cfg(windows)]
pub(crate) fn twin_plane_resource<'a>(
    this: This,
) -> Option<&'a windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    if this.is_null() {
        return None;
    }
    let (_, twin) = hosted(this, true)?;
    let twin = twin?;
    // SAFETY: as in `twin_resource`.
    twin.plane
        .as_ref()
        .map(|p| unsafe { &*std::ptr::from_ref(&p.texture) })
}

/// The colour twin for a texture described by `desc` whose host copy is
/// `depth`, if the title asks for depth and unordered access both and the
/// host can copy between the two: the same size, mips and slices in the
/// depth format's typeless family (R16 for 16-bit depth, R32 for 32-bit),
/// or R8 for the stencil plane of a format that has one, with unordered
/// access only. A format with a stencil plane gets a twin for its depth
/// plane too, R16 for the console's 16-bit depth-stencil formats and R32
/// otherwise. `VICTORY_DEPTH_TWINS=0` makes none (such views stay null),
/// `VICTORY_DEPTH_TWINS=stencil` only those of formats with a stencil
/// plane, `VICTORY_DEPTH_TWINS=nodepth` none for the depth planes of such
/// formats.
#[cfg(windows)]
fn twin_texture(
    desc: &ResourceDesc,
    depth: &windows::Win32::Graphics::Direct3D12::ID3D12Resource,
) -> Option<std::sync::Arc<Twin>> {
    use std::sync::atomic::Ordering::Relaxed;
    use windows::Win32::Graphics::Direct3D12::*;
    const DEPTH_AND_UNORDERED: u32 = 6;
    const UNORDERED: u32 = 4;
    const UNORDERED_ACCESS: u32 = 0x8;
    // R32 and R16 typeless: the families of the depth formats of one plane.
    const ONE_PLANE: [u32; 2] = [39, 53];
    if desc.dimension == BUFFER || desc.flags & DEPTH_AND_UNORDERED != DEPTH_AND_UNORDERED {
        return None;
    }
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("VICTORY_DEPTH_TWINS").is_none_or(|v| v != "0")) {
        return None;
    }
    // R32G8X24 and R24G8 typeless: depth with a stencil plane, whose twin
    // is R8 typeless.
    const WITH_STENCIL: [u32; 2] = [19, 44];
    const R8_TYPELESS: u32 = 60;
    let typeless = family(pc_format(desc.format));
    let stencil = WITH_STENCIL.contains(&typeless);
    static STENCIL_ONLY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let stencil_only = *STENCIL_ONLY
        .get_or_init(|| std::env::var_os("VICTORY_DEPTH_TWINS").is_some_and(|v| v == "stencil"));
    if !stencil && stencil_only {
        return None;
    }
    if !(stencil || ONE_PLANE.contains(&typeless)) || desc.sample_count > 1 {
        TWINS[1].fetch_add(1, Relaxed);
        return None;
    }
    let host = crate::host::host()?;
    let mut d = *desc;
    d.layout = 0;
    d.alignment = 0;
    d.flags = UNORDERED;
    d.format = if stencil { R8_TYPELESS } else { typeless };
    let heap = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        ..Default::default()
    };
    let mut texture: Option<ID3D12Resource> = None;
    // SAFETY: the description has the host layout, as in `host_texture`;
    // the out pointer is ours.
    let made = unsafe {
        host.device.CreateCommittedResource(
            &heap,
            D3D12_HEAP_FLAG_NONE,
            (&raw const d).cast(),
            D3D12_RESOURCE_STATES(UNORDERED_ACCESS as i32),
            None,
            &mut texture,
        )
    };
    if let Err(e) = made {
        tracing::warn!(error = %e, desc = ?d, "no colour twin for a depth texture");
    }
    let Some(texture) = texture else {
        TWINS[1].fetch_add(1, Relaxed);
        return None;
    };
    crate::host::name(&texture, || {
        format!("depth twin {}x{} f{}", d.width, d.height, d.format)
    });
    // The buffer a stencil plane and its twin are copied through: as big
    // as the plane's mips and slices laid out for a copy (they follow the
    // depth plane's in the texture's subresources).
    let through = if stencil {
        // SAFETY: a live texture.
        let of = unsafe { depth.GetDesc() };
        let planes = u32::from(of.MipLevels) * u32::from(of.DepthOrArraySize);
        let mut bytes = 0;
        // SAFETY: a plain query; the out pointer is ours.
        unsafe {
            host.device.GetCopyableFootprints(
                &of,
                planes,
                planes,
                0,
                None,
                None,
                None,
                Some(&mut bytes),
            );
        }
        let Some(buffer) = host.uav_buffer(bytes) else {
            TWINS[1].fetch_add(1, Relaxed);
            return None;
        };
        Some(buffer)
    } else {
        None
    };
    // The twin of the depth plane of a format with a stencil plane: R16
    // typeless (53) for the console's 16-bit depth-stencil formats (118 to
    // 120), whose shaders write 16-bit steps, R32 typeless (39) otherwise.
    static NO_PLANE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let no_plane = *NO_PLANE
        .get_or_init(|| std::env::var_os("VICTORY_DEPTH_TWINS").is_some_and(|v| v == "nodepth"));
    let plane = if stencil && !no_plane {
        d.format = if (118..=120).contains(&desc.format) {
            53
        } else {
            39
        };
        let mut texture: Option<ID3D12Resource> = None;
        // SAFETY: as for the first twin.
        let made = unsafe {
            host.device.CreateCommittedResource(
                &heap,
                D3D12_HEAP_FLAG_NONE,
                (&raw const d).cast(),
                D3D12_RESOURCE_STATES(UNORDERED_ACCESS as i32),
                None,
                &mut texture,
            )
        };
        if let Err(e) = made {
            tracing::warn!(error = %e, desc = ?d, "no twin for a depth plane");
        }
        texture.map(|texture| {
            crate::host::name(&texture, || {
                format!("depth plane twin {}x{} f{}", d.width, d.height, d.format)
            });
            std::sync::Arc::new(HostTexture::new(texture, UNORDERED_ACCESS))
        })
    } else {
        None
    };
    TWINS[0].fetch_add(1, Relaxed);
    Some(std::sync::Arc::new(Twin {
        colour: std::sync::Arc::new(HostTexture::new(texture, UNORDERED_ACCESS)),
        behind: std::sync::atomic::AtomicU32::new(0),
        through,
        plane,
    }))
}

/// The typeless format of a format's family (itself if it has none).
pub(crate) fn family(format: u32) -> u32 {
    const FAMILIES: [(u32, u32); 22] = [
        (1, 4),
        (5, 8),
        (9, 14),
        (15, 18),
        (19, 22),
        (23, 25),
        (27, 32),
        (33, 38),
        (39, 43),
        (44, 47),
        (48, 52),
        (53, 59),
        (60, 65),
        (70, 72),
        (73, 75),
        (76, 78),
        (79, 81),
        (82, 84),
        (90, 91),
        (92, 93),
        (94, 96),
        (97, 99),
    ];
    match format {
        87 => 90,
        88 => 92,
        f => FAMILIES
            .iter()
            .find(|&&(first, last)| (first..=last).contains(&f))
            .map_or(f, |&(first, _)| first),
    }
}

/// sRGB formats and their linear twins.
pub(crate) const SRGB: [(u32, u32); 3] = [(29, 28), (91, 87), (93, 88)];

/// The format a view of a title texture must name when its host copy is
/// typeless (see [`DEPTH_FORMATS`], [`SRGB`]): the title's own; `None` if
/// the host copy is typed.
pub(crate) fn view_format(this: This) -> Option<u32> {
    // First: a view of no resource (a typed null view) has none.
    let host = host_resource(this)?;
    let own = pc_format(resource(this).desc.format);
    // SAFETY: a live resource.
    let format = unsafe { host.GetDesc() }.Format.0 as u32;
    // A stand-in target is typed.
    (format != own && format != target_format(own)).then_some(own)
}

/// Depth formats with their typeless family: depth textures that shaders
/// read are made typeless on the host, which only allows views of
/// typeless depth resources.
pub(crate) const DEPTH_FORMATS: [(u32, u32); 4] = [(20, 19), (40, 39), (45, 44), (55, 53)];

/// Mip levels, the full chain for 0.
fn mip_count(desc: &ResourceDesc) -> u32 {
    let depth = if desc.dimension == TEXTURE_3D {
        u64::from(desc.depth_or_array_size)
    } else {
        1
    };
    match desc.mip_levels {
        0 => {
            64 - desc
                .width
                .max(u64::from(desc.height))
                .max(depth)
                .max(1)
                .leading_zeros()
        }
        n => u32::from(n),
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlacedFootprint {
    pub(crate) offset: u64,
    format: u32,
    width: u32,
    height: u32,
    depth: u32,
    row_pitch: u32,
}

impl PlacedFootprint {
    /// The bytes it spans in a buffer: every row pitched but the last.
    pub(crate) fn bytes(&self) -> Option<u64> {
        let (element, block) = element(self.format)?;
        let lines = u64::from(self.height).div_ceil(block) * u64::from(self.depth);
        let last = u64::from(self.width).div_ceil(block) * element;
        Some(u64::from(self.row_pitch) * lines.saturating_sub(1) + last)
    }

    /// For a read back through a host buffer: this footprint at offset 0
    /// with its rows pitched as the host wants them (multiples of 256
    /// bytes), then the bytes that buffer needs, the bytes of a row, the
    /// rows, and the title's own pitch.
    pub(crate) fn host_rows(&self) -> Option<(PlacedFootprint, [u64; 4])> {
        let (element, block) = element(self.format)?;
        let lines = (u64::from(self.height).div_ceil(block) * u64::from(self.depth)).max(1);
        let row = u64::from(self.width).div_ceil(block) * element;
        let pitch = row.max(1).next_multiple_of(256);
        let host = PlacedFootprint {
            offset: 0,
            row_pitch: u32::try_from(pitch).ok()?,
            ..*self
        };
        Some((host, [pitch * lines, row, lines, u64::from(self.row_pitch)]))
    }
}
const _: () = assert!(size_of::<PlacedFootprint>() == 32);

const NO_FOOTPRINT: PlacedFootprint = PlacedFootprint {
    offset: u64::MAX,
    format: 0,
    width: 0,
    height: 0,
    depth: 0,
    row_pitch: 0,
};

/// One subresource's size in an upload buffer: footprint (offset 0), rows
/// and bytes per row. Planes of depth-stencil formats are not told apart.
fn subresource(desc: &ResourceDesc, index: u32) -> Option<(PlacedFootprint, u32, u64)> {
    const PITCH: u64 = 256;
    if desc.dimension == BUFFER {
        let footprint = PlacedFootprint {
            offset: 0,
            format: 0,
            width: u32::try_from(desc.width).ok()?,
            height: 1,
            depth: 1,
            row_pitch: u32::try_from(desc.width.next_multiple_of(PITCH)).ok()?,
        };
        return (index == 0).then_some((footprint, 1, desc.width));
    }
    let mips = mip_count(desc);
    let slices = if desc.dimension == TEXTURE_3D {
        1
    } else {
        u32::from(desc.depth_or_array_size).max(1)
    };
    let (mip, slice) = (index % mips, index / mips);
    if slice >= slices {
        return None;
    }
    let (element, block) = element(desc.format)?;
    let width = (desc.width >> mip).max(1).next_multiple_of(block);
    let height = (u64::from(desc.height) >> mip)
        .max(1)
        .next_multiple_of(block);
    let depth = if desc.dimension == TEXTURE_3D {
        (u32::from(desc.depth_or_array_size) >> mip).max(1)
    } else {
        1
    };
    let row_size = width / block * element;
    let footprint = PlacedFootprint {
        offset: 0,
        format: desc.format,
        width: u32::try_from(width).ok()?,
        height: u32::try_from(height).ok()?,
        depth,
        row_pitch: u32::try_from(row_size.next_multiple_of(PITCH)).ok()?,
    };
    Some((footprint, u32::try_from(height / block).ok()?, row_size))
}

/// Device method: where each subresource goes in an upload buffer, rows
/// 256-byte and subresources 512-byte aligned, as on PC. Unknown sizes give
/// the documented all-ones values.
pub(crate) extern "system" fn copyable_footprints(
    _: This,
    desc: *const ResourceDesc,
    first: u32,
    count: u32,
    base: u64,
    layouts: *mut PlacedFootprint,
    rows: *mut u32,
    row_sizes: *mut u64,
    total: *mut u64,
) {
    const PLACEMENT: u64 = 512;
    // SAFETY: the title passes a description (null is tolerated).
    let desc = unsafe { desc.as_ref() };
    let mut end = base;
    let mut known = desc.is_some();
    for i in 0..count {
        let one = desc.and_then(|d| subresource(d, first + i));
        let (layout, row_count, row_size) = match one {
            Some((mut footprint, row_count, row_size)) => {
                footprint.offset = end.next_multiple_of(PLACEMENT);
                let pitch = u64::from(footprint.row_pitch);
                let lines = u64::from(row_count) * u64::from(footprint.depth);
                end = footprint.offset + pitch * lines.saturating_sub(1) + row_size;
                (footprint, row_count, row_size)
            }
            None => {
                known = false;
                (NO_FOOTPRINT, u32::MAX, u64::MAX)
            }
        };
        let i = i as usize;
        // SAFETY: each output is null or holds `count` entries.
        unsafe {
            if !layouts.is_null() {
                layouts.add(i).write(layout);
            }
            if !rows.is_null() {
                rows.add(i).write(row_count);
            }
            if !row_sizes.is_null() {
                row_sizes.add(i).write(row_size);
            }
        }
    }
    if !known {
        tracing::warn!(?desc, "no footprint for this resource");
    }
    if !total.is_null() {
        // SAFETY: checked non-null.
        unsafe { total.write(if known { end - base } else { u64::MAX }) };
    }
}

/// Device method: the footprint of resources placed one after another.
/// Returns a 16-byte value, through a hidden first argument. Invalid input
/// gives the documented size `u64::MAX`.
pub(crate) extern "system" fn allocation_info(
    out: *mut AllocationInfo,
    _: This,
    _visible: u32,
    count: u32,
    descs: *const ResourceDesc,
) -> *mut AllocationInfo {
    let descs = if descs.is_null() {
        &[][..]
    } else {
        // SAFETY: the title passes `count` descriptions.
        unsafe { std::slice::from_raw_parts(descs, count as usize) }
    };
    // SAFETY: the caller passes room for the value.
    unsafe { out.write(place(descs.iter(), |_, _| ())) };
    out
}

/// The newer description: the plain one, then a sampler feedback mip
/// region (width, height, depth).
#[repr(C)]
pub(crate) struct ResourceDesc1 {
    desc: ResourceDesc,
    mip_region: [u32; 3],
}
const _: () = assert!(size_of::<ResourceDesc1>() == 72);

/// Where one resource goes in [`allocation_info2`]'s placement.
#[repr(C)]
pub(crate) struct AllocationInfo1 {
    offset: u64,
    alignment: u64,
    size: u64,
}

/// Device method 0x360 (`ID3D12Device8`): [`allocation_info`] for newer
/// descriptions, also telling where each resource goes.
pub(crate) extern "system" fn allocation_info2(
    out: *mut AllocationInfo,
    _: This,
    _visible: u32,
    count: u32,
    descs: *const ResourceDesc1,
    each: *mut AllocationInfo1,
) -> *mut AllocationInfo {
    let descs = if descs.is_null() {
        &[][..]
    } else {
        // SAFETY: the title passes `count` descriptions.
        unsafe { std::slice::from_raw_parts(descs, count as usize) }
    };
    let total = place(descs.iter().map(|d| &d.desc), |i, info| {
        if !each.is_null() {
            // SAFETY: the title passes room for `count` entries.
            unsafe { each.add(i).write(info) };
        }
    });
    // SAFETY: the caller passes room for the value.
    unsafe { out.write(total) };
    out
}

/// Resources placed one after another: the whole footprint, telling `each`
/// where every resource goes. Unknown sizes give the documented `u64::MAX`.
fn place<'a>(
    descs: impl Iterator<Item = &'a ResourceDesc>,
    mut each: impl FnMut(usize, AllocationInfo1),
) -> AllocationInfo {
    let mut total = AllocationInfo {
        size: 0,
        alignment: KIB_64,
    };
    for (i, desc) in descs.enumerate() {
        let Some(one) = footprint(desc) else {
            tracing::warn!(?desc, "no size for this resource");
            each(
                i,
                AllocationInfo1 {
                    offset: u64::MAX,
                    alignment: 0,
                    size: u64::MAX,
                },
            );
            return AllocationInfo {
                size: u64::MAX,
                alignment: 0,
            };
        };
        tracing::debug!(?desc, ?one, "allocation info");
        let offset = total.size.next_multiple_of(one.alignment);
        each(
            i,
            AllocationInfo1 {
                offset,
                alignment: one.alignment,
                size: one.size,
            },
        );
        total.size = offset + one.size;
        total.alignment = total.alignment.max(one.alignment);
    }
    total
}

/// Zeroed host memory standing in for GPU memory.
pub(crate) struct Memory {
    ptr: NonNull<u8>,
    layout: Layout,
    /// Set when the memory came from the title's allocator, with these
    /// allocation attributes (it goes back there).
    attributes: Option<u64>,
}

pub type AllocRoutine = extern "system" fn(usize, u64) -> *mut c_void;
pub type FreeRoutine = extern "system" fn(*mut c_void, u64);
static ALLOCATOR: std::sync::OnceLock<(AllocRoutine, FreeRoutine)> = std::sync::OnceLock::new();

/// Committed resources get their memory from the title's general
/// allocator, as on the console, where titles hook it to place graphics
/// memory themselves (the title places its back buffers in a
/// block of its own and checks they fill it). The runtime names it before
/// the device exists.
pub fn use_allocator(alloc: AllocRoutine, free: FreeRoutine) {
    let _ = ALLOCATOR.set((alloc, free));
}

/// Maps the physical pages behind one range of title memory at another,
/// `(to, from, len)` (`from` 0 unmaps); false on failure.
pub type PageMapper = fn(u64, u64, u64) -> bool;
static PAGE_MAPPER: std::sync::OnceLock<PageMapper> = std::sync::OnceLock::new();

/// The runtime's page mapper, for the queues' page-mapping copies (memory
/// is the runtime's, as on the console).
pub fn use_page_mapper(mapper: PageMapper) {
    let _ = PAGE_MAPPER.set(mapper);
}

pub(crate) fn page_mapper() -> Option<PageMapper> {
    PAGE_MAPPER.get().copied()
}

/// Allocation attributes for resource memory: allocator id, page size
/// (bits 14-15: 2 MiB pages from 2 MiB up, else 64 KiB), alignment as log2
/// (bits 24-28) and memory type (bits 29-32: write-combined for GPU-only
/// heaps, cacheable for the CPU-visible ones).
/// shortcut: the console runtime's own choice of page size and memory type
/// is not known; this is the plausible one.
fn resource_attributes(info: AllocationInfo, heap_kind: u32) -> u64 {
    const GRAPHICS_ALLOCATOR: u64 = 128 << 16;
    const DEFAULT_HEAP: u32 = 1;
    let pages: u64 = if info.size >= 2 << 20 { 2 } else { 1 };
    let memory: u64 = if heap_kind == DEFAULT_HEAP { 1 } else { 3 };
    let alignment = u64::from(info.alignment.max(1).trailing_zeros());
    GRAPHICS_ALLOCATOR | pages << 14 | alignment << 24 | memory << 29
}

impl Memory {
    /// A committed resource's memory: from the title's allocator if the
    /// runtime named one, else our own; zeroed either way.
    fn for_resource(info: AllocationInfo, heap_kind: u32) -> Result<Memory, HRESULT> {
        let Some(&(alloc, _)) = ALLOCATOR.get() else {
            return Memory::new(info);
        };
        let layout = Layout::from_size_align(info.size as usize, info.alignment as usize)
            .map_err(|_| E_INVALIDARG)?;
        let attributes = resource_attributes(info, heap_kind);
        let ptr =
            NonNull::new(alloc(layout.size(), attributes).cast::<u8>()).ok_or(E_OUTOFMEMORY)?;
        // SAFETY: the allocator handed out `size` bytes.
        unsafe { ptr.as_ptr().write_bytes(0, layout.size()) };
        Ok(Memory {
            ptr,
            layout,
            attributes: Some(attributes),
        })
    }

    /// `size` zeroed bytes aligned to `alignment` (a power of two).
    pub(crate) fn zeroed(size: u64, alignment: u64) -> Result<Memory, HRESULT> {
        Memory::new(AllocationInfo { size, alignment })
    }

    pub(crate) fn address(&self) -> u64 {
        self.ptr.as_ptr() as u64
    }

    fn new(info: AllocationInfo) -> Result<Memory, HRESULT> {
        let layout = usize::try_from(info.size)
            .ok()
            .zip(usize::try_from(info.alignment).ok())
            .filter(|&(size, _)| size > 0)
            .and_then(|(size, align)| Layout::from_size_align(size, align).ok())
            .ok_or(E_INVALIDARG)?;
        // SAFETY: the size is non-zero.
        let ptr = NonNull::new(unsafe { alloc_zeroed(layout) }).ok_or(E_OUTOFMEMORY)?;
        // The big ones are named in the log: the memory line counts our own
        // allocations by size, and these are the title's memory given out
        // of ours (a heap it made through the device, a descriptor heap).
        if info.size >= 32 << 20 {
            tracing::info!(mb = info.size >> 20, at = ?ptr, "memory for the title from our allocator (a heap or a descriptor heap)");
        }
        Ok(Memory {
            ptr,
            layout,
            attributes: None,
        })
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        match (self.attributes, ALLOCATOR.get()) {
            (Some(attributes), Some(&(_, free))) => free(self.ptr.as_ptr().cast(), attributes),
            // SAFETY: allocated in `new` with this layout.
            _ => unsafe { dealloc(self.ptr.as_ptr(), self.layout) },
        }
    }
}

#[repr(C)]
struct Heap {
    object: Object,
    desc: HeapDesc,
    memory: Memory,
}

pub(crate) extern "system" fn create_heap(
    _: This,
    desc: *const HeapDesc,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes a description (null is tolerated).
    let Some(&desc) = (unsafe { desc.as_ref() }) else {
        return E_INVALIDARG;
    };
    tracing::debug!(
        size = desc.size,
        kind = desc.properties.kind,
        flags = format_args!("{:#x}", desc.flags),
        "heap"
    );
    if desc.alignment != 0 && !desc.alignment.is_power_of_two() {
        return E_INVALIDARG;
    }
    let info = AllocationInfo {
        size: desc.size,
        alignment: desc.alignment.max(KIB_64),
    };
    let memory = match Memory::new(info) {
        Ok(memory) => memory,
        Err(hr) => return hr,
    };
    let heap = Box::new(Heap {
        object: Object::new(&HEAP_FUNCTIONS),
        desc,
        memory,
    });
    object::hand_out(HEAP, heap, iid, &HEAP_IIDS, out)
}

extern "system" fn heap_query(this: This, iid: *const Iid, out: *mut This) -> HRESULT {
    object::answer(HEAP, iid, object::is(iid, &HEAP_IIDS).then_some(this), out)
}
extern "system" fn heap_release(this: This) -> u32 {
    object::release_boxed::<Heap>(this)
}
extern "system" fn heap_desc(this: This, out: *mut HeapDesc) {
    if !out.is_null() {
        // SAFETY: `this` is a heap; the title passes room for a description.
        unsafe { out.write((*this.cast::<Heap>()).desc) };
    }
}

/// A resource's memory: its own, or a range of a heap it holds a reference
/// to.
enum Backing {
    Committed(Memory),
    Placed {
        heap: This,
        offset: u64,
    },
    /// Memory the title mapped itself, at this address.
    Address(*mut u8),
}

impl Drop for Backing {
    fn drop(&mut self) {
        if let Backing::Placed { heap, .. } = *self {
            heap_release(heap);
        }
    }
}

#[repr(C)]
struct Resource {
    object: Object,
    desc: ResourceDesc,
    heap: HeapProperties,
    heap_flags: u32,
    backing: Backing,
    /// Textures also live on the host GPU (decision 0007); buffers do not
    /// yet. With, for a depth texture the title also writes as unordered
    /// access, the colour texture shaders write instead ([`Twin`]).
    #[cfg(windows)]
    host: Hosted,
}

/// A title texture's host copy and the state it is in. The title's barriers
/// name console states (looser than the host's), so we transition from this
/// one. Lists that use the texture hold it until they are reset: the title
/// may release a resource whose commands it recorded but not yet submitted
/// (console memory outlives the object; a host texture must outlive the
/// lists that name it).
#[cfg(windows)]
pub(crate) struct HostTexture {
    pub(crate) texture: windows::Win32::Graphics::Direct3D12::ID3D12Resource,
    pub(crate) state: std::sync::atomic::AtomicU32,
    /// The bytes the host gives it.
    bytes: u64,
    /// Whether its contents come from title memory ([`fills`]): counted in
    /// [`KINDS`] while it lives.
    filled: bool,
    /// How many of the title texture's first levels it leaves out
    /// ([`lowered_levels`]): its level 0 is the title's level `skip`.
    pub(crate) skip: u32,
}

/// What a title texture has on the host: its host copy and, for a depth
/// texture shaders also write, the colour twin.
#[cfg(windows)]
type Made = (std::sync::Arc<HostTexture>, Option<std::sync::Arc<Twin>>);

/// A title texture's host side. The host copy is made when something first
/// uses the texture, not when the title creates it: the title
/// had created 3,871 textures (9.2 GB on the host) before its world's first
/// frame, of which a frame's lists name some hundreds (run 23), on a card
/// with 12 GB. A title texture is memory the title has either way; its host
/// copy is the card's.
#[cfg(windows)]
struct Hosted {
    /// The host copy (and twin) once made. Taken out again only by
    /// [`let_go_idle`] and when the title releases the resource.
    made: std::sync::Mutex<Option<Made>>,
    /// The state and clear value the title created the texture with.
    state: u32,
    clear: Option<[u32; 5]>,
    /// When the host copy was last asked for (`crate::host::moment`),
    /// written under `made`'s lock.
    used: std::sync::atomic::AtomicU64,
    /// The host refused to make it: not tried again.
    refused: std::sync::atomic::AtomicBool,
    /// Its host copy was let go as idle once.
    gone: std::sync::atomic::AtomicBool,
    /// A texture of [`PLACED_TEXTURES`].
    placed: std::sync::atomic::AtomicBool,
    /// Something copied from or into it on the host: its host copy has
    /// every level from then on ([`host_resource_whole`]).
    whole: std::sync::atomic::AtomicBool,
    /// (PROJECT ONE) A draw of a character read it: its host copy leaves out
    /// at most [`detail_skip`] first levels from then on
    /// ([`host_resource_detail`]).
    detail: std::sync::atomic::AtomicBool,
}

/// Counts for the memory line: host copies of textures the title fills
/// alive and their bytes; such copies let go as idle since the start and
/// their bytes; made again after that and their bytes; and host copies made
/// since the start.
#[cfg(windows)]
pub(crate) static KINDS: [std::sync::atomic::AtomicU64; 7] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 7];

/// Every title texture alive (not buffers), by address: for the sweep and
/// the memory line. A texture leaves the table before it is freed.
#[cfg(windows)]
static TEXTURES: std::sync::Mutex<std::collections::BTreeSet<usize>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

#[cfg(windows)]
fn textures() -> std::sync::MutexGuard<'static, std::collections::BTreeSet<usize>> {
    TEXTURES.lock().unwrap_or_else(|e| e.into_inner())
}

/// The title textures the host renders to or writes (targets, depth,
/// unordered access), by the address of their memory: what a view the
/// title places in that memory in another format shows ([`alias_of`]). A
/// texture leaves the table before it is freed.
#[cfg(windows)]
static TARGETS_AT: std::sync::Mutex<std::collections::BTreeMap<u64, usize>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// For `this`, a texture made for a view the title placed in memory
/// ([`texture_at`]): the title texture the host writes that lies in the
/// same memory, if there is one of the same size, levels and layers with
/// texels of the same width. The console reads such a texture through the
/// view as it is
/// (the title packs light as R9G9B9E5 into R32_UINT textures
/// with compute shaders, which cannot write R9G9B9E5, and samples them
/// through R9G9B9E5 views placed at the textures' addresses: the sky, the
/// sky and light probes, the bounced light the lighting pass adds). On the
/// host the two are two textures and what the shaders wrote exists in one
/// of them only: the other was filled from title memory, which nothing
/// writes, so everything read through such a view was black.
/// Textures of several levels or layers count too (the screen probes'
/// light atlas has four levels, the camera's sky light cache six layers,
/// the cloud and sky tables are volumes): only textures of one level and
/// one layer were found at first, and the programs behind the others read
/// the title's unwritten memory - the sky light cache's second stage wrote
/// all zeros from a first stage that held light (run 88).
/// `VICTORY_ALIAS_COPY=0`: none is found, as before; `=1`: only textures
/// of one level and one layer, as at first.
#[cfg(windows)]
pub(crate) fn alias_of(this: This) -> Option<This> {
    use std::sync::atomic::Ordering::Relaxed;
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if this.is_null()
        || !*ON.get_or_init(|| std::env::var_os("VICTORY_ALIAS_COPY").is_none_or(|v| v != "0"))
    {
        return None;
    }
    static WHOLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let whole =
        *WHOLE.get_or_init(|| std::env::var_os("VICTORY_ALIAS_COPY").is_none_or(|v| v != "1"));
    let r = resource(this);
    // A level count of 0 asks for a full chain: not counted here.
    let one = |d: &ResourceDesc| {
        d.dimension != BUFFER
            && d.mip_levels >= 1
            && d.depth_or_array_size >= 1
            && (whole || (d.mip_levels == 1 && d.depth_or_array_size == 1))
            && d.sample_count <= 1
    };
    if !r.host.placed.load(Relaxed) || !fills(&r.desc) || !one(&r.desc) {
        return None;
    }
    // Under the table's lock: a texture leaves it before it is freed.
    let targets = TARGETS_AT.lock().unwrap_or_else(|e| e.into_inner());
    let other = *targets.get(&(r.data() as u64))? as This;
    let o = resource(other);
    const DEPTH: u32 = 0x2;
    let same = one(&o.desc)
        && o.desc.dimension == r.desc.dimension
        && o.desc.width == r.desc.width
        && o.desc.height == r.desc.height
        && o.desc.mip_levels == r.desc.mip_levels
        && o.desc.depth_or_array_size == r.desc.depth_or_array_size
        && o.desc.flags & DEPTH == 0
        && target_format(o.desc.format) == o.desc.format
        && element(o.desc.format).is_some_and(|e| e.1 == 1 && Some(e) == element(r.desc.format));
    same.then_some(other)
}

/// Whether host copies are made on first use. On by default;
/// `VICTORY_LAZY_TEXTURES=0` makes each when the title creates its texture,
/// as before.
#[cfg(windows)]
fn lazy() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_LAZY_TEXTURES").is_none_or(|v| v != "0"))
}

/// Whether a texture described by `desc` takes its contents from title
/// memory: one the host neither renders to nor writes (no target, depth or
/// unordered access flag). Its host copy can be made again from that
/// memory; any other texture's contents exist on the host alone.
#[cfg(windows)]
fn fills(desc: &ResourceDesc) -> bool {
    const TARGET_DEPTH_UNORDERED: u32 = 0x7;
    desc.dimension != BUFFER && desc.flags & TARGET_DEPTH_UNORDERED == 0
}

/// Whether the title fills texture `this` itself ([`fills`]).
#[cfg(windows)]
pub(crate) fn title_fills(this: This) -> bool {
    !this.is_null() && fills(&resource(this).desc)
}

/// Whether `this` is a texture whose host copy is not made yet (and may
/// still be): a descriptor slot written with a view of it waits
/// (`crate::descriptor::make_waiting`).
#[cfg(windows)]
pub(crate) fn host_waits(this: This) -> bool {
    if this.is_null() || !lazy() {
        return false;
    }
    let r = resource(this);
    r.desc.dimension != BUFFER
        && !r.host.refused.load(std::sync::atomic::Ordering::Relaxed)
        && r.host.made.lock().unwrap_or_else(|e| e.into_inner()).is_none()
}

/// The host side of title texture `this`: made now if it is not there and
/// `make` says so. `None` for buffers and for textures the host refuses.
#[cfg(windows)]
fn hosted(this: This, make: bool) -> Option<Made> {
    hosted_as(this, make, false)
}

/// [`hosted`]; with `whole`, a host copy with every level of the title's
/// texture: one made without its first levels ([`lowered_levels`]) is let
/// go and made again whole, and stays whole.
#[cfg(windows)]
fn hosted_as(this: This, make: bool, whole: bool) -> Option<Made> {
    use std::sync::atomic::Ordering::Relaxed;
    let r = resource(this);
    if r.desc.dimension == BUFFER {
        return None;
    }
    let mut made = r.host.made.lock().unwrap_or_else(|e| e.into_inner());
    if whole {
        r.host.whole.store(true, Relaxed);
        if made.as_ref().is_some_and(|(host, _)| host.skip > 0) {
            // Under the lock, as the sweep does: the next use must not find
            // the staging state of the copy let go.
            crate::texture::unstage(this);
            if let Some((host, _)) = made.take() {
                LOWERED[2].fetch_add(1, Relaxed);
                // The slots that show it are made again of the whole one
                // before the next submission runs.
                crate::descriptor::texture_replaced(&host.texture);
                #[cfg(feature = "diagnostics")]
                note_let_go(&host.texture, this as usize, "made again whole");
                crate::host::retire(host);
            }
        }
    }
    if made.is_none() {
        if !make || r.host.refused.load(Relaxed) {
            return None;
        }
        let clear = r
            .host
            .clear
            .as_ref()
            .map_or(std::ptr::null(), |c| std::ptr::from_ref(c).cast::<c_void>());
        let skip = if r.host.whole.load(Relaxed) {
            0
        } else if r.host.detail.load(Relaxed) {
            // (PROJECT ONE) A character's texture: fewer levels left out,
            // the pool's pressure still added.
            lowered_levels(&r.desc, (detail_skip() + PRESSURE.load(Relaxed)).min(skip_levels()))
        } else {
            // The run's setting, and more as the pool fills (`pressure`):
            // added to it, or a run that already asks for two levels less
            // got coarser copies in the pool's last quarter alone, which
            // then filled (run of 25/11, 13:58: 36,000 textures in five
            // minutes where 84,000 were made with a larger budget).
            lowered_levels(&r.desc, (skip_levels() + PRESSURE.load(Relaxed)).min(6))
        };
        let Some(texture) = host_texture(&lowered(&r.desc, skip), r.host.state, clear) else {
            r.host.refused.store(true, Relaxed);
            return None;
        };
        let twin = twin_texture(&r.desc, &texture);
        // One line per depth texture with unordered access: its address is
        // the one target dumps and the captured frame's list of commands
        // name.
        if r.desc.flags & 6 == 6 {
            tracing::info!(
                resource = ?this,
                desc = ?r.desc,
                twin = twin.is_some(),
                "depth texture with unordered access"
            );
        }
        let state = crate::host::legal_state(&texture, crate::host::texture_state(r.host.state));
        let mut host = HostTexture::new(texture, state);
        host.filled = fills(&r.desc);
        host.skip = skip;
        if skip > 0 {
            LOWERED[0].fetch_add(1, Relaxed);
            // What the whole copy would have taken beside this one.
            let whole_bytes = host.bytes << (2 * skip);
            LOWERED[1].fetch_add(whole_bytes.saturating_sub(host.bytes), Relaxed);
        }
        if host.filled {
            KINDS[0].fetch_add(1, Relaxed);
            KINDS[1].fetch_add(host.bytes, Relaxed);
        }
        if r.host.gone.load(Relaxed) {
            KINDS[4].fetch_add(1, Relaxed);
            KINDS[5].fetch_add(host.bytes, Relaxed);
        }
        KINDS[6].fetch_add(1, Relaxed);
        *made = Some((std::sync::Arc::new(host), twin));
        // Slots the title wrote with views of it while it had no host copy
        // are made before the next submission runs.
        crate::descriptor::texture_made(this as usize);
    }
    r.host.used.store(crate::host::moment(), Relaxed);
    made.clone()
}

/// (PROJECT ONE) Whether title texture `this` has its host copy, without
/// making it or counting a use (`draw::premake_textures`).
#[cfg(windows)]
pub(crate) fn is_hosted(this: This) -> bool {
    !this.is_null() && hosted(this, false).is_some()
}

/// The host copy of title texture `this` if it is made, without making it
/// (for diagnostics, which must not count as a use).
#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn host_made<'a>(
    this: This,
) -> Option<&'a windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    if this.is_null() {
        return None;
    }
    let (host, _) = hosted(this, false)?;
    // SAFETY: as in `host_resource`.
    Some(unsafe { &*std::ptr::from_ref(&host.texture) })
}

/// Whether more host copies of textures the title fills may be made for
/// shaders that pick textures by index (`draw.rs` `heap_textures`): always
/// up to [`TEXTURE_FLOOR`] bytes of them, beyond that while the card's
/// memory in use by this process is under nine tenths of the budget the
/// system gives it (`VICTORY_TEXTURE_BUDGET_MB` sets a limit on those
/// copies' bytes instead). Past the budget the system moves the card's
/// memory into the PC's, which ended runs 20 and 24.
#[cfg(windows)]
pub(crate) fn room(memory: Option<[u64; 4]>) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    const WITHOUT_BUDGET: u64 = 4 << 30;
    if stream_pool_on() {
        // (PROJECT ONE) By what these copies take of a pool of their own,
        // not by all the card holds: see `pool_budget`.
        return room_in(pool_now(memory));
    }
    let filled = KINDS[1].load(Relaxed);
    if let Some(limit) = texture_limit() {
        return filled < limit;
    }
    if filled < TEXTURE_FLOOR.get().copied().unwrap_or(FLOOR_MOST) {
        return true;
    }
    match memory {
        Some([budget, used, ..]) => used < budget / 10 * 9,
        None => filled < WITHOUT_BUDGET,
    }
}

/// `VICTORY_TEXTURE_BUDGET_MB=<n>`: the bytes host copies of textures the
/// title fills may take, whatever the card says.
#[cfg(windows)]
fn texture_limit() -> Option<u64> {
    static LIMIT: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| {
        let mb = std::env::var("VICTORY_TEXTURE_BUDGET_MB").ok()?;
        mb.trim().parse::<u64>().ok().map(|mb| mb << 20)
    })
}

/// The pool of host copies of textures the title fills, as the card's
/// `memory` (its budget and our use of it) sizes it now ([`pool_budget`]).
/// A pass that makes copies as it goes asks once and then [`room_in`] for
/// each: what it makes is not in `memory` yet.
#[cfg(windows)]
pub(crate) fn pool_now(memory: Option<[u64; 4]>) -> u64 {
    const WITHOUT_BUDGET: u64 = 4 << 30;
    if let Some(limit) = texture_limit() {
        return limit;
    }
    match memory {
        Some([budget, used, ..]) => pool_budget(budget, used, KINDS[1].load(std::sync::atomic::Ordering::Relaxed)),
        None => WITHOUT_BUDGET,
    }
}

/// Whether a pool of `pool` bytes ([`pool_now`]) has room for one more
/// host copy now; and notes what the next copy leaves out ([`PRESSURE`]).
#[cfg(windows)]
pub(crate) fn room_in(pool: u64) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    let filled = KINDS[1].load(Relaxed);
    POOL[0].store(pool, Relaxed);
    POOL[1].store(filled, Relaxed);
    PRESSURE.store(if pressure_on() { pressure(filled, pool) } else { 0 }, Relaxed);
    filled < pool
}

/// `VICTORY_STREAM_PRESSURE=0`: new host copies leave out what the run's
/// own setting says and no more, however full their pool is ([`pressure`]
/// off; the pool and the progress every frame is owed stay).
#[cfg(windows)]
fn pressure_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_STREAM_PRESSURE").is_none_or(|v| v != "0"))
}

/// `VICTORY_STREAM_POOL=0`: [`room`] by the card's whole use, as before
/// (no texture made while the card is past nine tenths of its budget).
#[cfg(windows)]
pub(crate) fn stream_pool_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_STREAM_POOL").is_none_or(|v| v != "0"))
}

/// The least the pool of host copies of textures the title fills is.
const POOL_LEAST: u64 = 512 << 20;

/// The bytes host copies of textures the title fills may take in all (the
/// pool [`room`] fills): nine tenths of the card's `budget`, as before,
/// less what we hold there that cannot be let go (`used` less those
/// copies' `filled` bytes: targets, buffers, upload memory); and
/// [`POOL_LEAST`] at least. The tenth left is the card's headroom (460 to
/// 530 MB of the 4.6 to 5.3 GB this card is given). Within the budget this
/// is the rule before, said of the copies alone: the run of 25/11, 10:08
/// (1280x720, 4246 MB used of 4753) keeps the 730 MB it had.
///
/// The rule before compared all of `used` with the budget. At 2560x1440
/// what cannot be let go is itself past the budget of a 6 GB card (run of
/// 25/11, 9:41: 5.5 to 6.5 GB used for 4.6), so no texture was made for
/// minutes on end: one in seven in seven and a half minutes, and nothing a
/// texture could do brought the use back under.
pub(crate) fn pool_budget(budget: u64, used: u64, filled: u64) -> u64 {
    let fixed = used.saturating_sub(filled);
    (budget / 10 * 9).saturating_sub(fixed).max(POOL_LEAST)
}

/// How many more of their first levels than the run asks new host copies
/// leave out for the room left in a pool of `pool` bytes holding `filled`:
/// none in its first quarter, then one more by quarter, three in the last
/// and past it (no copy gets smaller than [`lowered_smallest`]). Each
/// level is a quarter of the bytes, so the pool's quarters hold 1, 4, 16
/// and 64 times their own size of the title's textures: a pool of 512 MiB
/// takes 10.9 GiB of them, coarser as it fills, where it took 512 MiB and
/// then nothing.
pub(crate) fn pressure(filled: u64, pool: u64) -> u32 {
    (filled.saturating_mul(4) / pool.max(1)).min(PRESSURE_MOST as u64) as u32
}

/// The most levels [`pressure`] leaves out.
pub(crate) const PRESSURE_MOST: u32 = 3;

/// The pool's size and what it holds, as [`room`] last found them.
pub(crate) static POOL: [std::sync::atomic::AtomicU64; 2] = [const { std::sync::atomic::AtomicU64::new(0) }; 2];

/// [`pressure`] as [`room`] last found it: what [`hosted_as`] leaves out of
/// a host copy it makes, where the run's own setting ([`skip_levels`]) asks
/// for less.
pub(crate) static PRESSURE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// [`room`]'s floor at most: what it was for every card.
const FLOOR_MOST: u64 = 2 << 30;

/// [`room`]'s floor, set from the card's budget when the host device is
/// made ([`texture_floor`]).
#[cfg(windows)]
pub(crate) static TEXTURE_FLOOR: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

/// The bytes of host copies of textures the title fills that [`room`]
/// lets be made whatever the card's budget says, for a card whose budget
/// is `budget`: 2 GiB, or a quarter of the budget where that is less. On a
/// card of 12 GB 2 GiB is a sixth of it, on one of 6 GB a third, and what
/// goes past the budget the system moves into the PC's memory shared with
/// the card, which upload memory and the host copies of title memory come
/// from too.
pub(crate) fn texture_floor(budget: Option<u64>) -> u64 {
    budget.filter(|&b| b > 0).map_or(FLOOR_MOST, |b| (b / 4).min(FLOOR_MOST))
}

/// Lets go of the host copies of textures the title fills that nothing
/// names any more: placed textures were never released (one host texture
/// per address and description ever viewed: 18,639 of them, 2.5 GB, after
/// nine minutes of run 24), nor those of texture objects before the title
/// released them. A host copy goes when
/// - the title fills the texture ([`fills`]) and the host never copied
///   into it: its contents are in title memory;
/// - only the resource holds it: no list not yet reset names it (a list
///   holds the copies of the textures it uses, `HostList::transition`);
/// - only the host copy holds the host texture: no descriptor slot views
///   it (a slot keeps what it views, `HostHeap::kept`), so no shader can
///   read it through a slot made earlier, by a table or by an index;
/// - nothing asked for it for `IDLE_MS` and `IDLE_FRAMES` frames.
///
/// The next use makes the host copy anew and uploads the title's memory
/// again (its staging state goes with it). The title resource stays: its
/// address is what views and lists name.
/// shortcut: [`host_resource`] hands out the host texture without holding
/// it; a thread that asked for it and has not yet recorded its use (which
/// makes the list or the slot hold it) is protected by that idle time alone.
/// Every caller uses it at once, within the title's call.
/// Called before a submission starts, by the thread that submits.
/// `VICTORY_TEXTURE_SWEEP=0` lets nothing go.
#[cfg(windows)]
/// (PROJECT ONE, diagnostics) Host copies of textures let go, newest last:
/// the host object, the title texture, why, when (`now_ms`). A page fault
/// in a texture DRED names freed is matched here by the host address its
/// name carries ([`let_go_note`]).
#[cfg(feature = "diagnostics")]
static LET_GO: std::sync::Mutex<std::collections::VecDeque<(usize, usize, &'static str, u64)>> =
    std::sync::Mutex::new(std::collections::VecDeque::new());

#[cfg(feature = "diagnostics")]
fn note_let_go(texture: &windows::Win32::Graphics::Direct3D12::ID3D12Resource, title: usize, why: &'static str) {
    let mut let_go = LET_GO.lock().unwrap_or_else(|e| e.into_inner());
    if let_go.len() >= 1 << 16 {
        let_go.pop_front();
    }
    let_go.push_back((windows::core::Interface::as_raw(texture) as usize, title, why, crate::host::now_ms()));
}

/// (PROJECT ONE, diagnostics) For each host address in `names` (DRED's
/// freed allocations, named "... host 0x..."), when and why its copy was
/// let go, if it was ([`LET_GO`]).
#[cfg(feature = "diagnostics")]
pub(crate) fn let_go_note(names: &str) -> String {
    let Ok(let_go) = LET_GO.try_lock() else {
        return "; textures let go: not looked at (busy)".to_owned();
    };
    let now = crate::host::now_ms();
    let found: Vec<String> = names
        .split("host 0x")
        .skip(1)
        .filter_map(|rest| usize::from_str_radix(rest.split(|c: char| !c.is_ascii_hexdigit()).next()?, 16).ok())
        .map(|host| match let_go.iter().rev().find(|e| e.0 == host) {
            Some((_, title, why, at)) => format!(
                "{host:#x}: {why}, title texture {title:#x}, {:.2} s ago",
                now.saturating_sub(*at) as f64 / 1000.0
            ),
            None => format!("{host:#x}: not let go by the four ways noted"),
        })
        .collect();
    format!("; textures let go (of the last {}): {found:?}", let_go.len())
}

pub(crate) fn let_go_idle() {
    use std::sync::atomic::Ordering::Relaxed;
    const IDLE_FRAMES: u64 = 4;
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("VICTORY_TEXTURE_SWEEP").is_none_or(|v| v != "0")) {
        return;
    }
    let idle_ms = crate::host::vram_idle_ms(crate::host::host().and_then(|h| h.video_memory()));
    // For the trail (`crate::host::trail`): how many went, their bytes,
    // and the first one.
    let (mut went, mut bytes, mut first) = (0u64, 0u64, None);
    let all = textures();
    for &at in all.iter() {
        // SAFETY: a texture leaves the table before it is freed, under the
        // lock held here (`Drop for Resource`).
        let r = unsafe { &*(at as *const Resource) };
        if !fills(&r.desc) || crate::texture::host_wrote(at) {
            continue;
        }
        let gone = {
            let mut made = r.host.made.lock().unwrap_or_else(|e| e.into_inner());
            let idle = made.as_ref().is_some_and(|(host, _)| {
                crate::host::long_ago(r.host.used.load(Relaxed), idle_ms, IDLE_FRAMES)
                    && std::sync::Arc::strong_count(host) == 1
                    && crate::host::references(&host.texture) == 1
            });
            if idle {
                // Under the lock: the next use must not find the staging
                // state of the texture let go (it says "uploaded").
                crate::texture::unstage(at as This);
                made.take()
            } else {
                None
            }
        };
        if let Some((host, _)) = gone {
            r.host.gone.store(true, Relaxed);
            KINDS[2].fetch_add(1, Relaxed);
            KINDS[3].fetch_add(host.bytes, Relaxed);
            went += 1;
            bytes += host.bytes;
            if first.is_none() {
                first = Some(at);
            }
            #[cfg(feature = "diagnostics")]
            note_let_go(&host.texture, at, "idle (sweep)");
            crate::host::retire(host);
        }
    }
    if went > 0 {
        crate::host::trail(|| {
            format!(
                "sweep: {went} host copies of textures the title fills let go ({} KiB), each held by its title texture alone (no list, no slot); the first: title texture {:#x}",
                bytes >> 10,
                first.unwrap_or(0)
            )
        });
    }
}

/// For the memory line: title textures with a host copy the host renders
/// to or writes and those copies' bytes (twins included); title textures
/// without a host copy and the bytes of title memory they are; host copies
/// of textures the title fills and their bytes; of all title textures,
/// the placed ones with a host copy; the bytes of the twins alone (the
/// layer's own: the console has no second texture for a depth texture
/// shaders write); and, of the host copies of placed textures the title
/// fills, the bytes of those whose title memory is not mapped any more and
/// of those a descriptor slot still shows (the card holds a texture the
/// title has given up for as long as a slot shows it). The last two are
/// estimates from every 16th such texture: asking Windows about the memory
/// of all 16,000 takes too long.
#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn census() -> [u64; 10] {
    const EVERY: u64 = 16;
    let mut count = [0u64; 10];
    let mut placed_filled = 0u64;
    let all = textures();
    for &at in all.iter() {
        // SAFETY: as in `let_go_idle`.
        let r = unsafe { &*(at as *const Resource) };
        let made = r.host.made.lock().unwrap_or_else(|e| e.into_inner());
        match made.as_ref() {
            Some((host, twin)) => {
                let kind = if host.filled { 4 } else { 0 };
                count[kind] += 1;
                count[kind + 1] += host.bytes + twin.as_ref().map_or(0, |t| t.colour.bytes);
                count[7] += twin.as_ref().map_or(0, |t| t.colour.bytes);
                let placed = r.host.placed.load(std::sync::atomic::Ordering::Relaxed);
                if placed {
                    count[6] += 1;
                }
                if placed && host.filled {
                    placed_filled += 1;
                    // Its first page: a texture's smallest mips lie there,
                    // and are mapped first and unmapped last.
                    if placed_filled.is_multiple_of(EVERY) && !crate::texture::committed(r.data().cast_const(), 1) {
                        count[8] += host.bytes * EVERY;
                        if crate::host::references(&host.texture) > 1 {
                            count[9] += host.bytes * EVERY;
                        }
                    }
                }
            }
            None => {
                count[2] += 1;
                count[3] += footprint(&r.desc).map_or(0, |f| f.size);
            }
        }
    }
    count
}

/// Host textures alive (the title's and the twins) and the bytes the host
/// gives them, for the memory line.
#[cfg(windows)]
pub(crate) static HOST_TEXTURES: [std::sync::atomic::AtomicU64; 2] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 2];

/// The bytes the host allocates for `texture`.
#[cfg(windows)]
fn host_bytes(texture: &windows::Win32::Graphics::Direct3D12::ID3D12Resource) -> u64 {
    let Some(host) = crate::host::host() else {
        return 0;
    };
    // SAFETY: plain queries on a live resource and device.
    unsafe {
        let desc = texture.GetDesc();
        host.device.GetResourceAllocationInfo(0, &[desc]).SizeInBytes
    }
}

#[cfg(windows)]
impl HostTexture {
    fn new(texture: windows::Win32::Graphics::Direct3D12::ID3D12Resource, state: u32) -> HostTexture {
        use std::sync::atomic::Ordering::Relaxed;
        let bytes = host_bytes(&texture);
        HOST_TEXTURES[0].fetch_add(1, Relaxed);
        HOST_TEXTURES[1].fetch_add(bytes, Relaxed);
        HostTexture {
            texture,
            state: std::sync::atomic::AtomicU32::new(state),
            bytes,
            filled: false,
            skip: 0,
        }
    }
}

#[cfg(windows)]
impl Drop for HostTexture {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering::Relaxed;
        HOST_TEXTURES[0].fetch_sub(1, Relaxed);
        HOST_TEXTURES[1].fetch_sub(self.bytes, Relaxed);
        if self.filled {
            KINDS[0].fetch_sub(1, Relaxed);
            KINDS[1].fetch_sub(self.bytes, Relaxed);
        }
    }
}

/// How many textures placed in title memory have a resource of ours
/// (never released), for the memory line.
#[cfg(all(windows, feature = "diagnostics"))]
pub(crate) fn placed_textures() -> u64 {
    PLACED_TEXTURES.lock().unwrap_or_else(|e| e.into_inner()).len() as u64
}

/// Moves the host texture to `wanted` (a host texture state); the host
/// resource and the state it leaves, if that is a change.
#[cfg(windows)]
pub(crate) fn host_transition(this: This, wanted: u32) -> Option<(*mut c_void, u32)> {
    use windows::core::Interface;
    let (host, _) = hosted(this, true)?;
    let before = host
        .state
        .swap(wanted, std::sync::atomic::Ordering::Relaxed);
    (before != wanted).then(|| (host.texture.as_raw(), before))
}

/// The host GPU's copy of a title resource, if it has one: made now if
/// this is the texture's first use ([`Hosted`]). Asking is a use; what must
/// not count as one asks `host_made`.
#[cfg(windows)]
pub(crate) fn host_resource<'a>(
    this: This,
) -> Option<&'a windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    if this.is_null() {
        return None;
    }
    let (host, _) = hosted(this, true)?;
    // SAFETY: the resource holds the host copy until the title releases it
    // (lists and slots that name the texture hold it past that), or until
    // `let_go_idle` takes it: only from a texture the title fills, that no
    // list and no slot holds and that nothing asked for in seconds (the
    // asking just made is noted under the same lock).
    Some(unsafe { &*std::ptr::from_ref(&host.texture) })
}

/// [`host_resource`] with every level of the title's texture, for what
/// names levels or texels on the host itself: a copy from or into the
/// texture, a resolve, a present. A host copy made without its first
/// levels ([`lowered_levels`]) is made again whole first.
#[cfg(windows)]
pub(crate) fn host_resource_whole<'a>(
    this: This,
) -> Option<&'a windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    if this.is_null() {
        return None;
    }
    let (host, _) = hosted_as(this, true, true)?;
    // SAFETY: as in `host_resource`.
    Some(unsafe { &*std::ptr::from_ref(&host.texture) })
}

/// (PROJECT ONE) [`host_resource`] for a texture a draw of a character
/// reads (`crate::pipeline::is_detail`): its host copy leaves out at most
/// [`detail_skip`] first levels where the run's setting leaves out more
/// (`VICTORY_TEXTURE_SKIP_MIPS`, 3 in `performance`: a face of 1024 texels
/// was one of 128). A copy made coarser before is let go and made again,
/// as [`host_resource_whole`] does, once; the slots that show it are made
/// again of the new one before the next submission runs.
#[cfg(windows)]
pub(crate) fn host_resource_detail<'a>(
    this: This,
) -> Option<&'a windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    use std::sync::atomic::Ordering::Relaxed;
    if this.is_null() {
        return None;
    }
    let r = resource(this);
    if r.desc.dimension != BUFFER && !r.host.detail.swap(true, Relaxed) {
        let mut made = r.host.made.lock().unwrap_or_else(|e| e.into_inner());
        let wanted = lowered_levels(&r.desc, detail_skip().min(skip_levels()));
        if made.as_ref().is_some_and(|(host, _)| host.skip > wanted) {
            crate::texture::unstage(this);
            if let Some((host, _)) = made.take() {
                DETAILED.fetch_add(1, Relaxed);
                crate::descriptor::texture_replaced(&host.texture);
                #[cfg(feature = "diagnostics")]
                note_let_go(&host.texture, this as usize, "made again finer (a character's)");
                crate::host::retire(host);
            }
        }
    }
    host_resource(this)
}

/// (PROJECT ONE) Host copies of characters' textures made again finer
/// ([`host_resource_detail`]), since the start.
#[cfg(windows)]
pub(crate) static DETAILED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// (PROJECT ONE) `VICTORY_DETAIL_SKIP=<n>` (0 to 6, 1 by default): the most
/// first levels the host copy of a character's texture leaves out.
#[cfg(windows)]
pub(crate) fn detail_skip() -> u32 {
    static SKIP: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *SKIP.get_or_init(|| {
        std::env::var("VICTORY_DETAIL_SKIP").ok().and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(1).min(6)
    })
}

/// How many of title texture `this`'s first levels `texture`, a host copy
/// of it, leaves out: told by the widths, so that it is said of the very
/// copy the caller holds, whatever was made of the texture since.
#[cfg(windows)]
pub(crate) fn levels_left_out(this: This, texture: &windows::Win32::Graphics::Direct3D12::ID3D12Resource) -> u32 {
    if this.is_null() {
        return 0;
    }
    let title = resource(this).desc.width;
    // SAFETY: a live resource.
    let host = unsafe { texture.GetDesc() }.Width;
    if host == 0 || title <= host {
        return 0;
    }
    (title / host).trailing_zeros()
}

/// How many of the title texture's first levels the host copy of `this`
/// leaves out (0 for one not made, or made whole).
#[cfg(all(windows, test))]
pub(crate) fn host_skip(this: This) -> u32 {
    if this.is_null() {
        return 0;
    }
    let r = resource(this);
    if r.desc.dimension == BUFFER {
        return 0;
    }
    r.host.made.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map_or(0, |(host, _)| host.skip)
}

/// `VICTORY_TEXTURE_SKIP_MIPS=<n>` (0 to 6, 0 by default): how many of
/// their first levels the host copies of the textures the title fills
/// leave out, where [`lowered_levels`] allows it. Each level left out
/// halves the width and height of what the card holds and of what is
/// read, detiled and sent up: three quarters of the bytes for one level,
/// fifteen sixteenths for two. The picture loses that detail and nothing
/// else: the views the title makes are made of the levels that are there.
#[cfg(windows)]
pub(crate) fn skip_levels() -> u32 {
    #[cfg(test)]
    if let Some(skip) = SKIP_FOR_TEST.get() {
        return skip;
    }
    static SKIP: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *SKIP.get_or_init(|| {
        let skip = std::env::var("VICTORY_TEXTURE_SKIP_MIPS")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(0)
            .min(6);
        if skip > 0 {
            tracing::info!(skip, "textures the title fills are kept without their first levels where they have them");
        }
        skip
    })
}

#[cfg(test)]
thread_local! {
    /// What [`skip_levels`] answers on a test's own thread.
    static SKIP_FOR_TEST: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// Host copies made without their first levels, the bytes that spared
/// (against the whole copies), and those made again whole because
/// something copied from or into them.
#[cfg(windows)]
pub(crate) static LOWERED: [std::sync::atomic::AtomicU64; 3] = [const { std::sync::atomic::AtomicU64::new(0) }; 3];

/// The smallest width or height a host copy is lowered to.
const LOWERED_SMALLEST: u64 = 128;

/// The smallest width or height for this run: [`LOWERED_SMALLEST`], or
/// what `VICTORY_TEXTURE_SMALLEST` asks for (32 to 1024 texels, a multiple
/// of 4): with 32 and six levels left out, a texture of 2048 texels is one
/// of 32, coarser than a console of 2000 drew.
fn lowered_smallest() -> u64 {
    static SMALLEST: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SMALLEST.get_or_init(|| {
        std::env::var("VICTORY_TEXTURE_SMALLEST")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|v| (32..=1024).contains(v) && v % 4 == 0)
            .unwrap_or(LOWERED_SMALLEST)
    })
}

/// How many first levels (of `wanted`) the host copy of a texture
/// described by `desc` may leave out: none unless it is a 2D texture (or
/// an array of them, a cube) the title fills, in a block-compressed
/// format, with levels to spare; and no more than leave it
/// [`LOWERED_SMALLEST`] wide and high, a whole number of 4 x 4 blocks
/// (the host makes no block-compressed texture of another size), and with
/// one level.
///
/// Block-compressed textures with a chain of levels are the title's
/// streamed material textures, sampled by coordinates between 0 and 1,
/// for which a smaller first level is the same picture with less detail.
/// Textures read by texel (tables, masks, heights, glyphs) are not in such
/// formats or have one level, and keep their size.
fn lowered_levels(desc: &ResourceDesc, wanted: u32) -> u32 {
    const TEXTURE2D: u32 = 3;
    let compressed = matches!(pc_format(desc.format), 70..=84 | 94..=99);
    if wanted == 0
        || desc.dimension != TEXTURE2D
        || desc.flags & 0x7 != 0
        || desc.sample_count > 1
        || !compressed
    {
        return 0;
    }
    let mut skip = wanted.min(u32::from(desc.mip_levels).saturating_sub(1));
    let fits = |skip: u32| {
        let (width, height) = (desc.width >> skip, u64::from(desc.height) >> skip);
        width >= lowered_smallest() && height >= lowered_smallest() && width % 4 == 0 && height % 4 == 0
    };
    while skip > 0 && !fits(skip) {
        skip -= 1;
    }
    skip
}

/// `desc` without its first `skip` levels.
fn lowered(desc: &ResourceDesc, skip: u32) -> ResourceDesc {
    ResourceDesc {
        width: (desc.width >> skip).max(1),
        height: (desc.height >> skip).max(1),
        mip_levels: desc.mip_levels.saturating_sub(skip as u16).max(1),
        ..*desc
    }
}

/// A title texture's host copy, shared, for a list to hold.
#[cfg(windows)]
pub(crate) fn host_copy(this: This) -> Option<std::sync::Arc<HostTexture>> {
    if this.is_null() {
        return None;
    }
    hosted(this, true).map(|made| made.0)
}

/// (PROJECT ONE) A title texture's width and height as the title described
/// it (none for a buffer); makes nothing.
#[cfg(windows)]
pub(crate) fn size_2d(this: This) -> Option<(u64, u32)> {
    if this.is_null() {
        return None;
    }
    let r = resource(this);
    (r.desc.dimension != BUFFER).then_some((r.desc.width, r.desc.height))
}

/// (PROJECT ONE) Host textures made for title textures, and the
/// microseconds their creation took (for the log's texture pass line).
pub(crate) static HOST_TEXTURES_MADE: [std::sync::atomic::AtomicU64; 2] =
    [const { std::sync::atomic::AtomicU64::new(0) }; 2];

/// A host texture like the title's: same description, in GPU memory,
/// without the console's layout and flags.
#[cfg(windows)]
fn host_texture(
    desc: &ResourceDesc,
    state: u32,
    clear: *const c_void,
) -> Option<windows::Win32::Graphics::Direct3D12::ID3D12Resource> {
    use windows::Win32::Graphics::Direct3D12::*;
    const PC_FLAGS: u32 = 0xf; // render target, depth, UAV, no shader access
    if desc.dimension == BUFFER {
        return None;
    }
    let host = crate::host::host()?;
    let mut d = *desc;
    d.layout = 0;
    d.alignment = 0;
    d.flags &= PC_FLAGS;
    // PC refuses depth targets that are also unordered-access (the console
    // allows both; the title's shadow maps ask for it).
    // The host copy loses the unordered access: shaders write a colour
    // twin instead ([`Twin`]).
    const DEPTH: u32 = 2;
    const UNORDERED: u32 = 4;
    if d.flags & DEPTH != 0 {
        if d.flags & UNORDERED != 0 {
            DEPTH_UNORDERED[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        d.flags &= !UNORDERED;
    }
    d.format = pc_format(d.format);
    const TARGET: u32 = 1;
    if d.flags & (TARGET | UNORDERED) != 0 {
        d.format = target_format(d.format);
    }
    const DENY_SHADER_RESOURCE: u32 = 8;
    if d.flags & DENY_SHADER_RESOURCE == 0
        && let Some(&(_, typeless)) = DEPTH_FORMATS.iter().find(|&&(f, _)| f == d.format)
    {
        d.format = typeless;
    }
    // The host has no sRGB unordered access: such textures are typeless,
    // their unordered access views plain UNORM.
    if d.flags & UNORDERED != 0 && SRGB.iter().any(|&(srgb, _)| srgb == d.format) {
        d.format = family(d.format);
    }
    // The clear value names the format too: format, then color or depth
    // and stencil (5 words).
    let clear_value = (!clear.is_null()).then(|| {
        // SAFETY: the title passes a whole clear value.
        let mut value = unsafe { clear.cast::<[u32; 5]>().read_unaligned() };
        value[0] = pc_format(value[0]);
        if d.flags & (TARGET | UNORDERED) != 0 {
            value[0] = target_format(value[0]);
        }
        value
    });
    let heap = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        ..Default::default()
    };
    let mut texture: Option<ID3D12Resource> = None;
    let began = std::time::Instant::now();
    // SAFETY: the description has the host layout (checked size, same
    // fields); the out pointer is ours.
    let made = unsafe {
        host.device.CreateCommittedResource(
            &heap,
            D3D12_HEAP_FLAG_NONE,
            (&raw const d).cast(),
            D3D12_RESOURCE_STATES(
                crate::host::legal_for(d.flags, crate::host::texture_state(state)) as i32,
            ),
            // The same layout: format, then color or depth and stencil.
            clear_value.as_ref().map(|v| std::ptr::from_ref(v).cast()),
            &mut texture,
        )
    };
    HOST_TEXTURES_MADE[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    HOST_TEXTURES_MADE[1].fetch_add(began.elapsed().as_micros() as u64, std::sync::atomic::Ordering::Relaxed);
    if let Err(e) = made {
        tracing::warn!(error = %e, desc = ?d, "no host texture");
    }
    if let Some(t) = &texture {
        crate::host::name(t, || {
            format!(
                "texture {}x{} f{} flags {:#x} host {:#x}",
                d.width,
                d.height,
                d.format,
                d.flags,
                windows::core::Interface::as_raw(t) as usize
            )
        });
        if d.flags & (TARGET | DEPTH | UNORDERED) != 0 {
            note_target_size(&d);
        }
    }
    texture
}

/// The sizes of the textures the title renders to or lets shaders write
/// (render targets, depth textures, unordered access), by width and
/// height: how many were made and are alive is not kept, only that the
/// size was asked for and the pixels made in all. They say at which
/// resolution a title really renders, whatever its window shows.
#[cfg(windows)]
static TARGET_SIZES: std::sync::Mutex<std::collections::BTreeMap<(u64, u32), u64>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Pixels of such textures made and not yet let go (all levels and slices
/// counted as the first: a measure, not their memory).
#[cfg(windows)]
static TARGET_PIXELS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(windows)]
fn note_target_size(d: &ResourceDesc) {
    let pixels = d.width * u64::from(d.height) * u64::from(d.depth_or_array_size.max(1));
    TARGET_PIXELS.fetch_add(pixels, std::sync::atomic::Ordering::Relaxed);
    let mut sizes = TARGET_SIZES.lock().unwrap_or_else(|e| e.into_inner());
    let made = sizes.entry((d.width, d.height)).or_insert(0);
    *made += 1;
    // Each size once, when the title first asks for it.
    if *made == 1 {
        tracing::info!(
            width = d.width,
            height = d.height,
            slices = d.depth_or_array_size,
            levels = d.mip_levels,
            format = d.format,
            flags = format_args!("{:#x}", d.flags),
            "a size the title renders to or lets shaders write, first asked for"
        );
    }
}

/// For the frame account: how many such sizes were asked for so far, the
/// largest by pixels, and the pixels made in all.
#[cfg(windows)]
pub(crate) fn target_sizes() -> (usize, (u64, u32), u64) {
    let sizes = TARGET_SIZES.lock().unwrap_or_else(|e| e.into_inner());
    let largest = sizes.keys().copied().max_by_key(|&(w, h)| w * u64::from(h)).unwrap_or((0, 0));
    (sizes.len(), largest, TARGET_PIXELS.load(std::sync::atomic::Ordering::Relaxed))
}

/// A texture's console memory and description; `None` for buffers.
pub(crate) fn texture_memory(this: This) -> Option<(*const u8, ResourceDesc)> {
    if this.is_null() {
        return None;
    }
    let r = resource(this);
    (r.desc.dimension != BUFFER).then(|| (r.data().cast_const(), r.desc))
}

/// A buffer's host memory and size; `None` for textures.
pub(crate) fn buffer_bytes(this: This) -> Option<(*mut u8, u64)> {
    if this.is_null() {
        return None;
    }
    let r = resource(this);
    (r.desc.dimension == BUFFER).then(|| (r.data(), r.desc.width))
}

impl Resource {
    fn data(&self) -> *mut u8 {
        match self.backing {
            Backing::Committed(ref memory) => memory.ptr.as_ptr(),
            // SAFETY: placement checked the range lies inside the heap.
            Backing::Placed { heap, offset } => unsafe {
                (*heap.cast::<Heap>())
                    .memory
                    .ptr
                    .as_ptr()
                    .add(offset as usize)
            },
            Backing::Address(at) => at,
        }
    }
}

fn resource<'a>(this: This) -> &'a Resource {
    // SAFETY: called from resource methods only.
    unsafe { &*this.cast::<Resource>() }
}

// The creation methods' own arguments, passed on.
#[allow(clippy::too_many_arguments)]
fn hand_out_resource(
    desc: ResourceDesc,
    heap: HeapProperties,
    heap_flags: u32,
    backing: Backing,
    state: u32,
    clear: *const c_void,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    #[cfg(not(windows))]
    let _ = (state, clear);
    let resource = Box::new(Resource {
        object: Object::new(&RESOURCE_FUNCTIONS),
        desc,
        heap,
        heap_flags,
        backing,
        #[cfg(windows)]
        host: Hosted {
            made: std::sync::Mutex::new(None),
            state,
            // SAFETY: the title passes a whole clear value (the format,
            // then color or depth and stencil: 5 words) or null.
            clear: (!clear.is_null()).then(|| unsafe { clear.cast::<[u32; 5]>().read_unaligned() }),
            used: std::sync::atomic::AtomicU64::new(0),
            refused: std::sync::atomic::AtomicBool::new(false),
            gone: std::sync::atomic::AtomicBool::new(false),
            placed: std::sync::atomic::AtomicBool::new(false),
            whole: std::sync::atomic::AtomicBool::new(false),
            detail: std::sync::atomic::AtomicBool::new(false),
        },
    });
    // The host copy waits for the texture's first use ([`Hosted`]).
    #[cfg(windows)]
    if desc.dimension != BUFFER {
        let at = std::ptr::from_ref::<Resource>(&resource) as usize;
        textures().insert(at);
        if !fills(&desc) {
            TARGETS_AT
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(resource.data() as u64, at);
        }
        if !lazy() {
            let _ = hosted(at as This, true);
        }
    }
    #[cfg(windows)]
    if desc.dimension == BUFFER {
        let at = std::ptr::from_ref::<Resource>(&resource) as usize;
        crate::shadow::register(at, resource.data() as u64, desc.width);
        if crate::readback::creating_readback() {
            crate::readback::tag(resource.data() as u64, desc.width);
        }
    }
    object::hand_out(RESOURCE, resource, iid, &RESOURCE_IIDS, out)
}

#[cfg(windows)]
impl Drop for Resource {
    fn drop(&mut self) {
        // Out of the table of textures first: the sweep looks at the ones
        // in it, under its lock.
        if self.desc.dimension != BUFFER {
            textures().remove(&(std::ptr::from_ref::<Resource>(self) as usize));
            if !fills(&self.desc) {
                let mut targets = TARGETS_AT.lock().unwrap_or_else(|e| e.into_inner());
                let at = self.data() as u64;
                if targets.get(&at) == Some(&(std::ptr::from_ref::<Resource>(self) as usize)) {
                    targets.remove(&at);
                }
            }
        }
        let made = self
            .host
            .made
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        #[cfg(feature = "diagnostics")]
        if made
            .as_ref()
            .is_some_and(|(h, _)| std::sync::Arc::strong_count(h) > 1)
        {
            // Before lists held their textures, this freed a texture that
            // lists not yet run still named (a GPU page fault).
            static OUTLIVED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = OUTLIVED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n.is_power_of_two() {
                tracing::info!(
                    n,
                    desc = ?self.desc,
                    "a title texture was released while a list not yet reset names its host copy"
                );
            }
        }
        // A running submission may still read the host copy (and a depth
        // texture's colour twin).
        if let Some((host, twin)) = made {
            #[cfg(feature = "diagnostics")]
            note_let_go(&host.texture, std::ptr::from_ref::<Resource>(self) as usize, "title texture released");
            crate::host::retire(host);
            if let Some(twin) = twin {
                crate::host::retire(twin);
            }
        }
        crate::texture::forget(std::ptr::from_ref::<Resource>(self) as usize);
        crate::descriptor::forget(std::ptr::from_ref::<Resource>(self) as usize);
        if self.desc.dimension == BUFFER {
            crate::shadow::unregister(
                std::ptr::from_ref::<Resource>(self) as usize,
                self.data() as u64,
            );
            crate::readback::untag(self.data() as u64);
        }
    }
}

pub(crate) extern "system" fn create_committed(
    _: This,
    heap: *const HeapProperties,
    heap_flags: u32,
    desc: *const ResourceDesc,
    state: u32,
    clear: *const c_void,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes both descriptions (null is tolerated).
    let (Some(&heap), Some(&desc)) = (unsafe { (heap.as_ref(), desc.as_ref()) }) else {
        return E_INVALIDARG;
    };
    let Some(info) = footprint(&desc) else {
        tracing::warn!(?desc, "no size for this resource");
        return E_INVALIDARG;
    };
    tracing::debug!(
        dimension = desc.dimension,
        size = info.size,
        heap = heap.kind,
        "committed resource"
    );
    let memory = match Memory::for_resource(info, heap.kind) {
        Ok(memory) => memory,
        Err(hr) => return hr,
    };
    hand_out_resource(
        desc,
        heap,
        heap_flags,
        Backing::Committed(memory),
        state,
        clear,
        iid,
        out,
    )
}

/// Device method 0x3f8, inferred from the title's calls
/// (title+0x193e1b4: its back buffers, heap flag "allow display"): the
/// committed resource creation of the enhanced-barrier interfaces,
/// `(heap, heap flags, desc1, initial layout, clear value, protected
/// session, castable format count, castable formats, iid, out)`. The host
/// copy starts in the common state, as layouts are not tracked.
#[allow(clippy::too_many_arguments)] // the method's own arguments
pub(crate) extern "system" fn create_committed3(
    this: This,
    heap: *const HeapProperties,
    heap_flags: u32,
    desc: *const ResourceDesc,
    _layout: u32,
    clear: *const c_void,
    _protected: This,
    _castable_count: u32,
    _castable: *const u32,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    const COMMON: u32 = 0;
    create_committed(this, heap, heap_flags, desc, COMMON, clear, iid, out)
}

/// Registered page pools (64 KiB pages the title maps into tiled
/// resources by page index): first address and page count; a handle is an
/// index plus one, a removed pool leaves `None`.
static PAGE_POOLS: std::sync::Mutex<Vec<Option<(u64, u32)>>> = std::sync::Mutex::new(Vec::new());

fn page_pools() -> std::sync::MutexGuard<'static, Vec<Option<(u64, u32)>>> {
    PAGE_POOLS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Device method 0x250: `(address, page count, handle out)`.
pub(crate) extern "system" fn register_page_pool(
    _: This,
    address: u64,
    pages: u32,
    out: *mut usize,
) -> HRESULT {
    if out.is_null() || pages == 0 || !address.is_multiple_of(KIB_64) {
        return E_INVALIDARG;
    }
    tracing::debug!(address = format_args!("{address:#x}"), pages, "page pool");
    let mut pools = page_pools();
    pools.push(Some((address, pages)));
    // SAFETY: checked non-null; the title passes room for a handle.
    unsafe { out.write(pools.len()) };
    S_OK
}

/// Device method 0x258.
pub(crate) extern "system" fn unregister_page_pool(_: This, handle: usize) -> HRESULT {
    match page_pools().get_mut(handle.wrapping_sub(1)) {
        Some(pool @ Some(_)) => {
            *pool = None;
            S_OK
        }
        _ => E_INVALIDARG,
    }
}

/// Device method 0x428, one of the `ID3D12Device9` to `ID3D12Device11`
/// additions. From the title's call: a resource at a virtual
/// address the title mapped itself, `(address, desc1, initial barrier
/// layout, clear value, iid, out)`, as `CreatePlacedResourceX` with
/// enhanced barriers. GPU addresses are CPU addresses, so the resource is
/// a view of that memory. `D3D12_RESOURCE_DESC1` starts like the plain
/// description; its sampler feedback region is not needed.
pub(crate) extern "system" fn create_at_address(
    _: This,
    address: u64,
    desc: *const ResourceDesc,
    layout: u32,
    clear: *const c_void,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes a description (null is tolerated).
    let Some(&desc) = (unsafe { desc.as_ref() }) else {
        return E_INVALIDARG;
    };
    if address == 0 {
        return E_INVALIDARG;
    }
    tracing::debug!(
        address = format_args!("{address:#x}"),
        dimension = desc.dimension,
        width = desc.width,
        layout,
        "resource at an address"
    );
    const COMMON: u32 = 0;
    const DEFAULT: HeapProperties = HeapProperties {
        kind: 1,
        cpu_pages: 0,
        pool: 0,
        creation_nodes: 1,
        visible_nodes: 1,
    };
    hand_out_resource(
        desc,
        DEFAULT,
        0,
        Backing::Address(address as *mut u8),
        COMMON,
        clear,
        iid,
        out,
    )
}

/// Textures the title placed in its own memory and views without a
/// resource object (device 0x230, the title's streamed and
/// interface textures), by address and description: a resource at that
/// address, made the first time a draw reads one.
/// The resource is kept for the run (views and lists name its address);
/// its host texture is let go when nothing names it any more
/// ([`let_go_idle`]) and made again on the next use.
#[cfg(windows)]
static PLACED_TEXTURES: std::sync::Mutex<std::collections::BTreeMap<(u64, [u32; 14]), usize>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The bytes of title memory a texture with the resource description
/// `desc` (as words) takes where the title placed it; `None` for a buffer
/// and for a description without a size.
#[cfg(windows)]
pub(crate) fn placed_bytes(desc: &[u32; 14]) -> Option<u64> {
    // SAFETY: 56 bytes in the description's layout.
    let d = unsafe { desc.as_ptr().cast::<ResourceDesc>().read_unaligned() };
    if d.dimension == BUFFER {
        return None;
    }
    footprint(&d).map(|f| f.size)
}

/// The bytes of title memory the mips from `first` on of a texture with the
/// resource description `desc` take where the title placed it. A title that
/// streams textures describes every mip of them and has memory for the ones
/// it streamed in, which a view of it shows: the title puts the
/// next texture right behind a 1024x1024 one of which 128x128 is there.
/// (Taken for as long as its description, such a texture lay over its
/// neighbours: 109,638 views were taken for stale in 8 minutes and showed
/// nothing from then on, and 700 KB were read for the 12 KB there.)
#[cfg(windows)]
pub(crate) fn placed_bytes_from(desc: &[u32; 14], first: u32) -> Option<u64> {
    // SAFETY: 56 bytes in the description's layout.
    let d = unsafe { desc.as_ptr().cast::<ResourceDesc>().read_unaligned() };
    if first == 0 || d.dimension == BUFFER {
        return placed_bytes(desc);
    }
    let whole = placed_bytes(desc)?;
    let Some(t) = texture_layout(&d).filter(|t| t.planes.len() == 1) else {
        return Some(whole);
    };
    let bytes = t.planes[0].layout.bytes_from(first);
    Some(bytes.next_multiple_of(t.alignment.max(1)).min(whole))
}

/// The first mip a placed texture's view shows: of views of one 2D texture
/// (the others show whole textures).
#[cfg(windows)]
pub(crate) fn first_mip(view: &[u32]) -> u32 {
    const TEXTURE_2D_VIEW: u32 = 4;
    match *view {
        [_, TEXTURE_2D_VIEW, _, _, first, ..] => first.min(15),
        _ => 0,
    }
}

/// The resource for a texture placed at `address` with the resource
/// description `desc` (as words), made on first use.
#[cfg(windows)]
pub(crate) fn texture_at(address: u64, desc: &[u32; 14]) -> Option<This> {
    // Words 1 and 13 of the description are the filler of its layout
    // (after the dimension, and at the end): the title leaves there what
    // its stack held, and two views of one texture that differed only in
    // them each got a resource and a host copy of their own, on a card
    // that fills at world entry. They are not part of what a texture is.
    // `VICTORY_PLACED_FILLER=1`: kept in the key, as before.
    static KEPT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let mut key = *desc;
    if !*KEPT.get_or_init(|| std::env::var_os("VICTORY_PLACED_FILLER").is_some_and(|v| v == "1")) {
        key[1] = 0;
        key[13] = 0;
    }
    let desc = &key;
    let mut map = PLACED_TEXTURES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&r) = map.get(&(address, *desc)) {
        return Some(r as This);
    }
    // SAFETY: 56 bytes in the description's layout.
    let d = unsafe { desc.as_ptr().cast::<ResourceDesc>().read_unaligned() };
    if d.dimension == BUFFER {
        return None;
    }
    let mut out = std::ptr::null();
    let made = create_at_address(
        std::ptr::null(),
        address,
        &d,
        0,
        std::ptr::null(),
        &RESOURCE_IIDS[0],
        &mut out,
    );
    if made != S_OK {
        return None;
    }
    map.insert((address, *desc), out as usize);
    resource(out)
        .host
        .placed
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Some(out)
}

pub(crate) extern "system" fn create_placed(
    _: This,
    heap: This,
    offset: u64,
    desc: *const ResourceDesc,
    state: u32,
    clear: *const c_void,
    iid: *const Iid,
    out: *mut This,
) -> HRESULT {
    // SAFETY: the title passes a description (null is tolerated).
    let Some(&desc) = (unsafe { desc.as_ref() }) else {
        return E_INVALIDARG;
    };
    if heap.is_null() {
        return E_INVALIDARG;
    }
    // SAFETY: the title passes one of our heaps.
    let h = unsafe { &*heap.cast::<Heap>() };
    let fits = footprint(&desc).is_some_and(|info| {
        offset.is_multiple_of(info.alignment)
            && offset
                .checked_add(info.size)
                .is_some_and(|end| end <= h.desc.size)
    });
    if !fits {
        tracing::warn!(
            ?desc,
            offset,
            heap = h.desc.size,
            "placed resource does not fit"
        );
        return E_INVALIDARG;
    }
    object::add_ref(heap);
    let backing = Backing::Placed { heap, offset };
    hand_out_resource(
        desc,
        h.desc.properties,
        h.desc.flags,
        backing,
        state,
        clear,
        iid,
        out,
    )
}

extern "system" fn resource_query(this: This, iid: *const Iid, out: *mut This) -> HRESULT {
    object::answer(
        RESOURCE,
        iid,
        object::is(iid, &RESOURCE_IIDS).then_some(this),
        out,
    )
}
extern "system" fn resource_release(this: This) -> u32 {
    object::release_boxed::<Resource>(this)
}
/// Memory is always mapped (subresources are not told apart yet); a null
/// `out` only asks for residency.
extern "system" fn map(this: This, _sub: u32, _read: *const c_void, out: *mut *mut u8) -> HRESULT {
    if !out.is_null() {
        // SAFETY: checked non-null; the title passes room for a pointer.
        unsafe { out.write(resource(this).data()) };
    }
    S_OK
}
extern "system" fn unmap(_: This, _sub: u32, _written: *const c_void) {}
extern "system" fn resource_desc(this: This, out: *mut ResourceDesc) {
    if !out.is_null() {
        // SAFETY: checked non-null; the title passes room for a description.
        unsafe { out.write(resource(this).desc) };
    }
}
/// Device method `GetCustomHeapProperties`, returning its struct through a
/// hidden first argument: what a standard heap type is as a custom heap.
/// Memory is unified and cache-coherent (as the architecture query says),
/// so every type is in pool L0, default heaps are not CPU-visible and the
/// others are write-back.
pub(crate) extern "system" fn custom_heap_properties(
    out: *mut HeapProperties,
    _: This,
    nodes: u32,
    kind: u32,
) -> *mut HeapProperties {
    const CUSTOM: u32 = 4;
    const NOT_AVAILABLE: u32 = 1;
    const WRITE_BACK: u32 = 3;
    const L0: u32 = 1;
    let nodes = nodes.max(1);
    let cpu_pages = match kind {
        1 => NOT_AVAILABLE,
        2 | 3 => WRITE_BACK,
        _ => 0,
    };
    let properties = HeapProperties {
        kind: CUSTOM,
        cpu_pages,
        pool: if cpu_pages == 0 { 0 } else { L0 },
        creation_nodes: nodes,
        visible_nodes: nodes,
    };
    // SAFETY: the title passes room for the result (null is tolerated).
    if let Some(out) = unsafe { out.as_mut() } {
        *out = properties;
    }
    out
}

extern "system" fn gpu_address(this: This) -> u64 {
    let r = resource(this);
    let at = r.data() as u64;
    // The console's GPU addresses fit in 40 bits; its ray-tracing builder
    // refuses others (diagnostics: which memory breaks that).
    #[cfg(feature = "diagnostics")]
    if at >> 40 != 0 {
        static ONCE: [std::sync::Once; 2] = [std::sync::Once::new(), std::sync::Once::new()];
        let backing = match r.backing {
            Backing::Committed(ref m) => format!("committed, attributes {:?}", m.attributes),
            Backing::Placed { offset, .. } => format!("placed in a heap at {offset:#x}"),
            Backing::Address(_) => "at a title address".into(),
        };
        ONCE[usize::from(r.desc.dimension == BUFFER)].call_once(|| {
            tracing::warn!(address = format_args!("{at:#x}"), backing, desc = ?r.desc, "a GPU address above 40 bits")
        });
    }
    at
}
extern "system" fn heap_properties(
    this: This,
    properties: *mut HeapProperties,
    flags: *mut u32,
) -> HRESULT {
    let r = resource(this);
    // SAFETY: either pointer may be null; otherwise the title passes room.
    unsafe {
        if let Some(p) = properties.as_mut() {
            *p = r.heap;
        }
        if let Some(f) = flags.as_mut() {
            *f = r.heap_flags;
        }
    }
    S_OK
}

/// IUnknown 3, object 4, device child 1, heap 1.
static HEAP_FUNCTIONS: Table<24> = {
    let mut f = object::d3d12_object(object::missing_table::<HEAP, 24>());
    f[0] = heap_query as *const ();
    f[2] = heap_release as *const ();
    f[7] = object::parent_device as *const (); // 0x38
    f[8] = heap_desc as *const (); // 0x40
    Table(f)
};

/// IUnknown 3, object 4, device child 1, resource 10 (7 as on PC, then 3
/// console ones).
static RESOURCE_FUNCTIONS: Table<32> = {
    let mut f = object::d3d12_object(object::missing_table::<RESOURCE, 32>());
    f[0] = resource_query as *const ();
    f[2] = resource_release as *const ();
    f[7] = object::parent_device as *const (); // 0x38
    f[8] = map as *const (); // 0x40
    f[9] = unmap as *const (); // 0x48
    f[10] = resource_desc as *const (); // 0x50
    f[11] = gpu_address as *const (); // 0x58
    f[14] = heap_properties as *const (); // 0x70
    Table(f)
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::slot;

    type Release = extern "system" fn(This) -> u32;
    type Map = extern "system" fn(This, u32, *const c_void, *mut *mut u8) -> HRESULT;
    type Address = extern "system" fn(This) -> u64;

    const PROPERTIES: HeapProperties = HeapProperties {
        kind: 1,
        cpu_pages: 0,
        pool: 0,
        creation_nodes: 0,
        visible_nodes: 0,
    };

    fn buffer(width: u64) -> ResourceDesc {
        ResourceDesc {
            dimension: BUFFER,
            alignment: 0,
            width,
            height: 1,
            depth_or_array_size: 1,
            mip_levels: 1,
            format: 0,
            sample_count: 1,
            sample_quality: 0,
            layout: 1,
            flags: 0,
        }
    }

    fn texture(width: u64, height: u32, format: u32, mips: u16) -> ResourceDesc {
        ResourceDesc {
            dimension: 3,
            width,
            height,
            format,
            mip_levels: mips,
            layout: 0,
            ..buffer(0)
        }
    }

    fn heap(size: u64) -> This {
        let desc = HeapDesc {
            size,
            properties: PROPERTIES,
            alignment: 0,
            flags: 0,
        };
        let mut heap = std::ptr::null();
        assert_eq!(
            create_heap(std::ptr::null(), &desc, &HEAP_IIDS[0], &mut heap),
            S_OK
        );
        heap
    }

    /// Which textures lose first levels on the host, and how many: 2D
    /// textures the title fills, block-compressed, with levels to spare,
    /// down to 128 texels at the smallest.
    #[test]
    fn only_compressed_textures_with_levels_to_spare_are_lowered() {
        const BC1: u32 = 71;
        const BC7: u32 = 98;
        const RGBA8: u32 = 28;
        // 2048 x 2048 with its twelve levels: two levels go (512 x 512).
        assert_eq!(lowered_levels(&texture(2048, 2048, BC7, 12), 2), 2);
        assert_eq!(lowered_levels(&texture(2048, 2048, BC7, 12), 1), 1);
        assert_eq!(lowered_levels(&texture(2048, 2048, BC7, 12), 0), 0);
        // Not below 128 texels: 256 x 256 loses one level, 128 x 128 none.
        assert_eq!(lowered_levels(&texture(256, 256, BC1, 9), 2), 1);
        assert_eq!(lowered_levels(&texture(512, 256, BC1, 10), 2), 1);
        assert_eq!(lowered_levels(&texture(128, 128, BC1, 8), 2), 0);
        // Whole blocks only: 1920 x 1080 halves to 960 x 540, and 270 rows
        // are not whole blocks; 1000 x 600 halves to 500 x 300, no further.
        assert_eq!(lowered_levels(&texture(1920, 1080, BC7, 11), 3), 1);
        assert_eq!(lowered_levels(&texture(1000, 600, BC1, 10), 2), 1);
        assert_eq!(lowered_levels(&texture(1004, 600, BC1, 10), 2), 0);
        // No more than the levels beside the last one.
        assert_eq!(lowered_levels(&texture(4096, 4096, BC1, 2), 4), 1);
        assert_eq!(lowered_levels(&texture(2048, 2048, BC1, 1), 2), 0);
        // Read by texel, or written by the host: kept whole.
        assert_eq!(lowered_levels(&texture(2048, 2048, RGBA8, 12), 2), 0);
        let target = ResourceDesc {
            flags: 1,
            ..texture(2048, 2048, BC7, 12)
        };
        assert_eq!(lowered_levels(&target, 2), 0);
        let volume = ResourceDesc {
            dimension: 4,
            ..texture(256, 256, BC1, 9)
        };
        assert_eq!(lowered_levels(&volume, 2), 0);
        // The description the host copy is made from.
        let d = lowered(&texture(2048, 1024, BC7, 12), 2);
        assert_eq!((d.width, d.height, d.mip_levels), (512, 256, 10));
        let same = lowered(&texture(2048, 1024, BC7, 12), 0);
        assert_eq!((same.width, same.height, same.mip_levels), (2048, 1024, 12));
    }

    /// On the host card: a compressed texture the title fills gets a host
    /// copy without its first two levels; a use that names its levels on
    /// the host (a copy) makes it whole, and it stays whole.
    #[cfg(windows)]
    #[test]
    fn a_lowered_texture_is_made_whole_for_a_copy() {
        if crate::host::host().is_none() {
            eprintln!("no host GPU; skipped");
            return;
        }
        const BC3: u32 = 77;
        SKIP_FOR_TEST.set(Some(2));
        let null = std::ptr::null();
        let mut sampled = null;
        let desc = texture(1024, 512, BC3, 10);
        assert_eq!(
            create_committed(null, &PROPERTIES, 0, &desc, 0, null.cast(), &RESOURCE_IIDS[0], &mut sampled),
            S_OK
        );
        let size = |t: &windows::Win32::Graphics::Direct3D12::ID3D12Resource| {
            // SAFETY: a live resource.
            let d = unsafe { t.GetDesc() };
            (d.Width, d.Height, d.MipLevels, d.Format.0)
        };
        let lowered_before = LOWERED[0].load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(size(host_resource(sampled).expect("a host copy")), (256, 128, 8, BC3 as i32));
        assert_eq!(host_skip(sampled), 2);
        assert!(LOWERED[0].load(std::sync::atomic::Ordering::Relaxed) > lowered_before);
        // The same copy while nothing asks for more.
        assert_eq!(size(host_resource(sampled).expect("a host copy")), (256, 128, 8, BC3 as i32));
        assert_eq!(size(host_resource_whole(sampled).expect("a whole host copy")), (1024, 512, 10, BC3 as i32));
        assert_eq!(host_skip(sampled), 0);
        assert_eq!(size(host_resource(sampled).expect("a host copy")), (1024, 512, 10, BC3 as i32));
        // A slot that showed the lowered copy shows the whole one once the
        // slots are made again, as before each submission.
        let mut other = null;
        assert_eq!(
            create_committed(null, &PROPERTIES, 0, &desc, 0, null.cast(), &RESOURCE_IIDS[0], &mut other),
            S_OK
        );
        if let Some(heap) = crate::descriptor::host::HostHeap::new(false, 8) {
            let raw = |t: &windows::Win32::Graphics::Direct3D12::ID3D12Resource| {
                windows::core::Interface::as_raw(t) as usize
            };
            let slot = [0u64; 4];
            let handle = slot.as_ptr() as u64;
            crate::descriptor::create_srv(null, other, std::ptr::null(), handle);
            let low = host_resource(other).expect("a host copy").clone();
            assert_eq!(size(&low).0, 256);
            assert_eq!(levels_left_out(other, &low), 2);
            heap.mirror(3, handle, true);
            assert_eq!(heap.shown(3), Some(raw(&low)));
            let whole = raw(host_resource_whole(other).expect("a whole host copy"));
            assert_ne!(whole, raw(&low));
            // Not before: the slot holds the copy it shows.
            assert_eq!(heap.shown(3), Some(raw(&low)));
            heap.remake_showing(&[raw(&low)]);
            assert_eq!(heap.shown(3), Some(whole));
            assert_eq!(levels_left_out(other, host_resource(other).expect("a host copy")), 0);
        }
        // A texture the host renders to is never lowered.
        let mut target = null;
        let rendered = ResourceDesc {
            flags: 1,
            ..texture(1024, 512, 28, 1)
        };
        assert_eq!(
            create_committed(null, &PROPERTIES, 0, &rendered, 0, null.cast(), &RESOURCE_IIDS[0], &mut target),
            S_OK
        );
        assert_eq!(size(host_resource(target).expect("a target")).0, 1024);
        assert_eq!(host_skip(target), 0);
        SKIP_FOR_TEST.set(None);
    }

    /// (PROJECT ONE) On the host card: with three levels left out by the
    /// run, a texture a character's draw reads is made again with one, once,
    /// and stays so; one such a draw reads first is made with one at once.
    #[cfg(windows)]
    #[test]
    fn a_characters_texture_keeps_more_of_its_levels() {
        if crate::host::host().is_none() {
            eprintln!("no host GPU; skipped");
            return;
        }
        const BC3: u32 = 77;
        SKIP_FOR_TEST.set(Some(3));
        let null = std::ptr::null();
        let size = |t: &windows::Win32::Graphics::Direct3D12::ID3D12Resource| {
            // SAFETY: a live resource.
            let d = unsafe { t.GetDesc() };
            (d.Width, d.Height, d.MipLevels)
        };
        let desc = texture(2048, 1024, BC3, 12);
        let mut face = null;
        assert_eq!(create_committed(null, &PROPERTIES, 0, &desc, 0, null.cast(), &RESOURCE_IIDS[0], &mut face), S_OK);
        assert_eq!(size(host_resource(face).expect("a host copy")), (256, 128, 9));
        let made_again = DETAILED.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(detail_skip(), 1, "the default");
        assert_eq!(size(host_resource_detail(face).expect("a finer copy")), (1024, 512, 11));
        assert_eq!(host_skip(face), 1);
        assert_eq!(size(host_resource_detail(face).expect("the same")), (1024, 512, 11));
        assert_eq!(size(host_resource(face).expect("it stays")), (1024, 512, 11));
        assert_eq!(DETAILED.load(std::sync::atomic::Ordering::Relaxed), made_again + 1, "made again once");
        let mut first = null;
        assert_eq!(create_committed(null, &PROPERTIES, 0, &desc, 0, null.cast(), &RESOURCE_IIDS[0], &mut first), S_OK);
        assert_eq!(size(host_resource_detail(first).expect("a finer copy")), (1024, 512, 11));
        assert_eq!(DETAILED.load(std::sync::atomic::Ordering::Relaxed), made_again + 1, "nothing made again");
        SKIP_FOR_TEST.set(None);
    }

    /// Textures are made whatever the card's budget up to 2 GiB, or a
    /// quarter of the budget where that is less; without a budget (or one
    /// of 0), 2 GiB as before.
    #[test]
    fn the_texture_floor_is_a_quarter_of_the_card_at_most_2_gib() {
        const GB: u64 = 1 << 30;
        assert_eq!(texture_floor(None), 2 * GB);
        assert_eq!(texture_floor(Some(0)), 2 * GB);
        assert_eq!(texture_floor(Some(11 * GB)), 2 * GB, "a card of 12 GB: as before");
        assert_eq!(texture_floor(Some(8 * GB)), 2 * GB);
        assert_eq!(texture_floor(Some(5 * GB + GB / 2)), (5 * GB + GB / 2) / 4, "a card of 6 GB");
        assert_eq!(texture_floor(Some(GB)), GB / 4);
    }

    #[test]
    fn heaps_are_zeroed_aligned_memory() {
        let heap = heap(1 << 20);
        // SAFETY: a heap we just made.
        let memory = unsafe { &(*heap.cast::<Heap>()).memory };
        assert_eq!(memory.ptr.as_ptr() as u64 % KIB_64, 0);
        let mut back = std::mem::MaybeUninit::<HeapDesc>::uninit();
        slot::<extern "system" fn(This, *mut HeapDesc)>(heap, 0x40)(heap, back.as_mut_ptr());
        // SAFETY: written above.
        assert_eq!(unsafe { back.assume_init() }.size, 1 << 20);
        assert_eq!(slot::<Release>(heap, 0x10)(heap), 0);

        let bad = HeapDesc {
            size: 1 << 20,
            properties: PROPERTIES,
            alignment: 3,
            flags: 0,
        };
        let mut out = std::ptr::null();
        assert_eq!(
            create_heap(std::ptr::null(), &bad, &HEAP_IIDS[0], &mut out),
            E_INVALIDARG
        );
    }

    #[test]
    fn device_setup_heap_properties_and_resources_at_addresses() {
        let device = crate::device::device();
        // GetCustomHeapProperties (0xd0): the struct comes back through the
        // hidden first argument, which is also returned.
        type Custom =
            extern "system" fn(*mut HeapProperties, This, u32, u32) -> *mut HeapProperties;
        let mut p = PROPERTIES;
        let back = slot::<Custom>(device, 0xd0)(&mut p, device, 1, 2);
        assert_eq!(back, &raw mut p);
        assert_eq!(
            (
                p.kind,
                p.cpu_pages,
                p.pool,
                p.creation_nodes,
                p.visible_nodes
            ),
            (4, 3, 1, 1, 1),
            "upload: custom, write-back, L0"
        );
        slot::<Custom>(device, 0xd0)(&mut p, device, 1, 1);
        assert_eq!(p.cpu_pages, 1, "default: not CPU-visible");

        // 0x428: a buffer over memory the title owns, as the title asks.
        type At = extern "system" fn(
            This,
            u64,
            *const ResourceDesc,
            u32,
            *const c_void,
            *const Iid,
            *mut This,
        ) -> HRESULT;
        let memory = vec![0u8; 0x1_0000];
        let address = memory.as_ptr() as u64;
        let mut r = std::ptr::null();
        let create = slot::<At>(device, 0x428);
        let desc = buffer(0x1_0000);
        let null = std::ptr::null();
        assert_eq!(
            create(
                device,
                address,
                &desc,
                u32::MAX,
                null,
                &RESOURCE_IIDS[0],
                &mut r
            ),
            0
        );
        assert_eq!(slot::<Address>(r, 0x58)(r), address);
        assert_eq!(slot::<Release>(r, 0x10)(r), 0);
        // The title's memory is still there after the release.
        assert_eq!(memory[0xffff], 0);
        assert_ne!(
            create(device, 0, &desc, u32::MAX, null, &RESOURCE_IIDS[0], &mut r),
            0
        );
    }

    #[test]
    fn sizes_cover_every_mip() {
        let size = |d: &ResourceDesc| footprint(d).map(|i| i.size);
        assert_eq!(size(&buffer(256)), Some(KIB_64));
        // Textures take their console layout and its alignment (4 KiB
        // standard swizzle when the description leaves it to the library):
        // 512x512 BC7 with 10 mips is 0x56000 bytes, 4 KiB aligned, as on
        // the console.
        let layout = |d: &ResourceDesc| texture_layout(d).unwrap().size;
        let rgba = texture(256, 256, 28, 0);
        assert_eq!(size(&rgba), Some(layout(&rgba).next_multiple_of(4096)));
        let bc7 = footprint(&texture(512, 512, 98, 10));
        let console = AllocationInfo {
            size: 0x56000,
            alignment: 0x1000,
        };
        assert_eq!(bc7, Some(console));
        assert!(layout(&rgba) >= 4 * (256 * 256 * 4) / 3, "every mip");
        // BC1 is 8 bytes per 4x4 block: 1024x1024, one mip = 512 KiB.
        assert_eq!(size(&texture(1024, 1024, 71, 1)), Some(512 << 10));
        assert_eq!(size(&texture(16, 16, 66, 1)), None, "1-bit format");
        // Linear layout (1) and an explicit mode (64 KiB display, 0x10a).
        let linear = ResourceDesc {
            layout: 1,
            ..texture(100, 60, 28, 1)
        };
        assert_eq!(
            texture_layout(&linear).unwrap().planes[0]
                .layout
                .desc
                .swizzle
                .0,
            0
        );
        let display = ResourceDesc {
            layout: 0x10a,
            ..texture(100, 60, 28, 1)
        };
        assert_eq!(
            texture_layout(&display).unwrap().planes[0]
                .layout
                .desc
                .swizzle
                .0,
            10
        );

        let mut info = AllocationInfo {
            size: 0,
            alignment: 0,
        };
        let descs = [buffer(100), texture(256, 256, 28, 0)];
        allocation_info(&mut info, std::ptr::null(), 0, 2, descs.as_ptr());
        assert_eq!(
            info,
            AllocationInfo {
                size: KIB_64 + size(&descs[1]).unwrap(),
                alignment: KIB_64
            }
        );
        allocation_info(&mut info, std::ptr::null(), 0, 1, &texture(1, 1, 999, 1));
        assert_eq!(info.size, u64::MAX);
    }

    #[test]
    fn upload_footprints_follow_the_pc_rules() {
        let null = std::ptr::null();
        let mut layouts = [NO_FOOTPRINT; 2];
        let (mut rows, mut sizes, mut total) = ([0; 2], [0; 2], 0);
        let mut footprints = |desc: &ResourceDesc, first, count| {
            let (l, r, s) = (layouts.as_mut_ptr(), rows.as_mut_ptr(), sizes.as_mut_ptr());
            copyable_footprints(null, desc, first, count, 0, l, r, s, &mut total);
            (layouts, rows, sizes, total)
        };
        // 100x60 RGBA8, 2 mips: 400-byte rows pitched to 512.
        let (l, r, s, t) = footprints(&texture(100, 60, 28, 2), 0, 2);
        assert_eq!((l[0].row_pitch, r[0], s[0]), (512, 60, 400));
        // Mip 1 (50x30, 200-byte rows) starts after 59 pitched rows and one
        // packed row, rounded up to 512.
        assert_eq!(l[1].offset, (512 * 59 + 400u64).next_multiple_of(512));
        assert_eq!((l[1].width, l[1].height, r[1]), (50, 30, 30));
        assert_eq!(t, l[1].offset + 256 * 29 + 200);
        // BC1: rows of 4x4 blocks, 8 bytes each.
        let (_, r, s, _) = footprints(&texture(64, 64, 71, 1), 0, 1);
        assert_eq!((r[0], s[0]), (16, 128));
        let (_, _, _, t) = footprints(&texture(64, 64, 71, 1), 1, 1);
        assert_eq!(t, u64::MAX, "no second subresource");
    }

    /// A 4x4 texture filled from a title buffer through our list and queue,
    /// then read back from the host GPU.
    #[cfg(windows)]
    #[test]
    fn buffer_data_reaches_the_host_texture() {
        use windows::Win32::Graphics::Direct3D12::*;
        let Some(host) = crate::host::host() else {
            return; // no host GPU here
        };
        let null = std::ptr::null();
        let iid = &RESOURCE_IIDS[0];
        const COPY_DEST: u32 = 0x400;
        let (mut image, mut source) = (null, null);
        let rgba = texture(4, 4, 28, 1);
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &rgba,
                COPY_DEST,
                null.cast(),
                iid,
                &mut image
            ),
            S_OK
        );
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &buffer(1024),
                0,
                null.cast(),
                iid,
                &mut source
            ),
            S_OK
        );
        let (data, _) = buffer_bytes(source).unwrap();
        // SAFETY: the buffer holds 1024 bytes.
        let bytes = unsafe { std::slice::from_raw_parts_mut(data, 1024) };
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }

        let queue_iid = guid(
            0x0ec870a6,
            0x5d7e,
            0x4c22,
            [0x8c, 0xfc, 0x5b, 0xaa, 0xe0, 0x76, 0x16, 0xed],
        );
        let allocator_iid = guid(
            0x6102dee4,
            0xaf59,
            0x4b09,
            [0xb9, 0x99, 0xb4, 0x4d, 0x73, 0xf0, 0x9b, 0x24],
        );
        let list_iid = guid(
            0x5b160d0f,
            0xac1b,
            0x4185,
            [0x8b, 0xa8, 0xb3, 0xae, 0x42, 0xa5, 0xa4, 0x55],
        );
        let (mut queue, mut allocator, mut list) = (null, null, null);
        let queue_desc = [0u32; 4];
        assert_eq!(
            crate::queue::create(null, queue_desc.as_ptr().cast(), &queue_iid, &mut queue),
            S_OK
        );
        assert_eq!(
            crate::list::create_allocator(null, 0, &allocator_iid, &mut allocator),
            S_OK
        );
        assert_eq!(
            crate::list::create_list(null, 0, 0, allocator, null, &list_iid, &mut list),
            S_OK
        );

        #[repr(C)]
        struct Location {
            resource: This,
            kind: u32,
            footprint: PlacedFootprint,
        }
        let rows = PlacedFootprint {
            offset: 0,
            format: 28,
            width: 4,
            height: 4,
            depth: 1,
            row_pitch: 256,
        };
        let to = Location {
            resource: image,
            kind: 0,
            footprint: PlacedFootprint { offset: 0, ..rows },
        };
        let from = Location {
            resource: source,
            kind: 1,
            footprint: rows,
        };
        type Copy =
            extern "system" fn(This, *const Location, u32, u32, u32, *const Location, *const u8);
        slot::<Copy>(list, 0x80)(list, &to, 0, 0, 0, &from, null.cast());
        slot::<extern "system" fn(This) -> HRESULT>(list, 0x48)(list);
        // The upload list holds the texture from its recording on: checked
        // before the submission, the one point where both paths hold it
        // through the title's list (with `late` the submission takes the
        // host list, and the flight lets it go when the card has run it,
        // which can be at once).
        assert!(std::sync::Arc::strong_count(&host_copy(image).unwrap()) > 2);
        slot::<extern "system" fn(This, u32, *const This)>(queue, 0x50)(queue, 1, &list);

        // Read the host texture back into a readback buffer.
        let readback_heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_READBACK,
            ..Default::default()
        };
        let readback_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: 1024,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let mut readback: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a readback buffer.
        unsafe {
            host.device
                .CreateCommittedResource(
                    &readback_heap,
                    D3D12_HEAP_FLAG_NONE,
                    &readback_desc,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    None,
                    &mut readback,
                )
                .unwrap();
        }
        let readback = readback.unwrap();
        let host_image = host_resource(image).unwrap();
        let back = host.new_list().unwrap();
        let (_, before) = host_transition(image, 0x800).unwrap();
        // Waited for, the upload list holds the texture until it is reset.
        if !crate::host::late() {
            assert!(std::sync::Arc::strong_count(&host_copy(image).unwrap()) > 2);
        }
        use windows::core::Interface;
        let out_to = Location {
            resource: readback.as_raw().cast_const().cast(),
            kind: 1,
            footprint: rows,
        };
        let out_from = Location {
            resource: host_image.as_raw().cast_const().cast(),
            kind: 0,
            footprint: to.footprint,
        };
        // SAFETY: an open list, live resources, locations in the host layout.
        unsafe {
            back.list.ResourceBarrier(&[crate::host::transition(
                host_image,
                D3D12_RESOURCE_STATES(before as i32),
                D3D12_RESOURCE_STATE_COPY_SOURCE,
            )]);
            back.list.CopyTextureRegion(
                (&raw const out_to).cast(),
                0,
                0,
                0,
                (&raw const out_from).cast(),
                None,
            );
        }
        back.close();
        host.execute(&[&back]);
        let mut mapped = std::ptr::null_mut();
        // SAFETY: the copy finished (execute waits); the buffer is 1024 bytes.
        let read = unsafe {
            readback
                .Map(
                    0,
                    Some(&D3D12_RANGE {
                        Begin: 0,
                        End: 1024,
                    }),
                    Some(&mut mapped),
                )
                .unwrap();
            std::slice::from_raw_parts(mapped.cast::<u8>(), 1024)
        };
        for row in 0..4 {
            let at = row * 256;
            assert_eq!(read[at..at + 16], bytes[at..at + 16], "row {row}");
        }
    }

    /// A 16-bit depth texture the title also asks unordered access for gets
    /// a colour twin, and the host copies one into the other, both ways: a
    /// pattern put into the twin is copied to the depth texture, the twin
    /// is overwritten with zeros, and the pattern comes back from the depth
    /// texture. A copy the host does not allow ends the test (with the
    /// debug layer's message under `VICTORY_D3D12_DEBUG=1` in a diagnostics
    /// build).
    #[cfg(windows)]
    #[test]
    fn a_depth_texture_and_its_twin_copy_into_each_other() {
        use windows::Win32::Graphics::Direct3D12::*;
        use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
        let Some(host) = crate::host::host() else {
            return; // no host GPU here
        };
        let null = std::ptr::null();
        const DEPTH_WRITE: u32 = 0x10;
        let desc = ResourceDesc {
            flags: 6,
            ..texture(4, 4, 55, 1)
        };
        let mut depth = null;
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &desc,
                DEPTH_WRITE,
                null.cast(),
                &RESOURCE_IIDS[0],
                &mut depth
            ),
            S_OK
        );
        let host_depth = host_resource(depth).unwrap();
        let Some(twin) = twin_resource(depth) else {
            return; // twins are turned off
        };
        // SAFETY: live resources.
        let (d, t) = unsafe { (host_depth.GetDesc(), twin.GetDesc()) };
        // Both R16 typeless: one a depth texture, one with unordered access.
        assert_eq!((d.Format.0, d.Flags.0), (53, 2));
        assert_eq!((t.Format.0, t.Flags.0), (53, 4));

        // Four rows of four 16-bit texels, 256 bytes apart as copies place
        // them: the pattern, then as many zeros.
        let mut pattern = vec![0u8; 1024];
        for row in 0..4 {
            for b in 0..8 {
                pattern[row * 256 + b] = (1 + row * 8 + b) as u8;
            }
        }
        let (upload, data) = host.upload_buffer(2048).unwrap();
        // SAFETY: the buffer's mapping, 2048 bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(pattern.as_ptr(), data, 1024);
            std::ptr::write_bytes(data.add(1024), 0, 1024);
        }
        let readback_heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_READBACK,
            ..Default::default()
        };
        let readback_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: 1024,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let mut readback: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a readback buffer.
        unsafe {
            host.device
                .CreateCommittedResource(
                    &readback_heap,
                    D3D12_HEAP_FLAG_NONE,
                    &readback_desc,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    None,
                    &mut readback,
                )
                .unwrap();
        }
        let readback = readback.unwrap();

        // A texture's one subresource; the four rows of a buffer at `offset`.
        let whole = |texture: &ID3D12Resource| D3D12_TEXTURE_COPY_LOCATION {
            // SAFETY: a borrowed pointer the location never releases.
            pResource: unsafe { std::mem::transmute_copy(texture) },
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: 0,
            },
        };
        let rows = |buffer: &ID3D12Resource, offset: u64| D3D12_TEXTURE_COPY_LOCATION {
            // SAFETY: as above.
            pResource: unsafe { std::mem::transmute_copy(buffer) },
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                    Offset: offset,
                    Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                        Format: DXGI_FORMAT(53),
                        Width: 4,
                        Height: 4,
                        Depth: 1,
                        RowPitch: 256,
                    },
                },
            },
        };
        let (written, tested, source, dest) = (
            D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
            D3D12_RESOURCE_STATE_DEPTH_WRITE,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
            D3D12_RESOURCE_STATE_COPY_DEST,
        );
        let moved = crate::host::transition;
        let list = host.new_list().unwrap();
        // SAFETY: an open list and live resources; the two textures are one
        // size in one format family.
        unsafe {
            let l = &list.list;
            // The pattern into the twin, the twin into the depth texture.
            l.ResourceBarrier(&[moved(twin, written, dest), moved(host_depth, tested, dest)]);
            l.CopyTextureRegion(&whole(twin), 0, 0, 0, &rows(&upload, 0), None);
            l.ResourceBarrier(&[moved(twin, dest, source)]);
            l.CopyResource(host_depth, twin);
            // Zeros over the twin, then the depth texture back into it.
            l.ResourceBarrier(&[moved(twin, source, dest)]);
            l.CopyTextureRegion(&whole(twin), 0, 0, 0, &rows(&upload, 1024), None);
            l.ResourceBarrier(&[moved(host_depth, dest, source)]);
            l.CopyResource(twin, host_depth);
            // The twin read back; both textures to the states they had.
            l.ResourceBarrier(&[moved(twin, dest, source)]);
            l.CopyTextureRegion(&rows(&readback, 0), 0, 0, 0, &whole(twin), None);
            l.ResourceBarrier(&[
                moved(twin, source, written),
                moved(host_depth, source, tested),
            ]);
        }
        list.close();
        host.execute(&[&list]);
        let mut mapped = std::ptr::null_mut();
        // SAFETY: the copies finished (execute waits); the buffer is 1024 bytes.
        let read = unsafe {
            readback
                .Map(
                    0,
                    Some(&D3D12_RANGE {
                        Begin: 0,
                        End: 1024,
                    }),
                    Some(&mut mapped),
                )
                .unwrap();
            std::slice::from_raw_parts(mapped.cast::<u8>(), 1024)
        };
        for row in 0..4 {
            let at = row * 256;
            assert_eq!(read[at..at + 8], pattern[at..at + 8], "row {row}");
        }
    }

    /// A depth texture with a stencil plane that the title also asks
    /// unordered access for gets a twin of the stencil plane, and what
    /// shaders write there is in the stencil plane once the list is closed:
    /// a pattern put into the twin is read back from the plane.
    #[cfg(windows)]
    #[test]
    fn a_stencil_planes_twin_reaches_the_plane() {
        use std::sync::atomic::Ordering::Relaxed;
        use windows::Win32::Graphics::Direct3D12::*;
        use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
        let Some(host) = crate::host::host() else {
            return; // no host GPU here
        };
        let null = std::ptr::null();
        const DEPTH_WRITE: u32 = 0x10;
        let desc = ResourceDesc {
            flags: 6,
            ..texture(4, 4, 45, 1)
        };
        let mut depth = null;
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &desc,
                DEPTH_WRITE,
                null.cast(),
                &RESOURCE_IIDS[0],
                &mut depth
            ),
            S_OK
        );
        let host_depth = host_resource(depth).unwrap();
        let Some(twin) = twin_resource(depth) else {
            return; // twins are turned off
        };
        // SAFETY: live resources.
        let (d, t) = unsafe { (host_depth.GetDesc(), twin.GetDesc()) };
        // R24G8 typeless with depth, R8 typeless with unordered access.
        assert_eq!((d.Format.0, d.Flags.0), (44, 2));
        assert_eq!((t.Format.0, t.Flags.0), (60, 4));

        // Four rows of four texels, 256 bytes apart as copies place them.
        let mut pattern = vec![0u8; 1024];
        for row in 0..4 {
            for b in 0..4 {
                pattern[row * 256 + b] = (1 + row * 4 + b) as u8;
            }
        }
        let (upload, data) = host.upload_buffer(1024).unwrap();
        // SAFETY: the buffer's mapping, 1024 bytes.
        unsafe { std::ptr::copy_nonoverlapping(pattern.as_ptr(), data, 1024) };
        let readback_heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_READBACK,
            ..Default::default()
        };
        let readback_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            // Room for the rows as the host places them (512 bytes apart
            // for the stencil plane on NVIDIA, not 256).
            Width: 4096,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let mut readback: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a readback buffer.
        unsafe {
            host.device
                .CreateCommittedResource(
                    &readback_heap,
                    D3D12_HEAP_FLAG_NONE,
                    &readback_desc,
                    D3D12_RESOURCE_STATE_COPY_DEST,
                    None,
                    &mut readback,
                )
                .unwrap();
        }
        let readback = readback.unwrap();
        // How the host lays the stencil plane (subresource 1) out in a buffer.
        let mut plane = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        // SAFETY: a plain query writing one footprint.
        unsafe {
            host.device
                .GetCopyableFootprints(&d, 1, 1, 0, Some(&mut plane), None, None, None);
        }
        let pattern_rows = D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
            Offset: 0,
            Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                Format: DXGI_FORMAT(60),
                Width: 4,
                Height: 4,
                Depth: 1,
                RowPitch: 256,
            },
        };

        // A subresource of a texture; rows of a buffer.
        let whole = |texture: &ID3D12Resource, index: u32| D3D12_TEXTURE_COPY_LOCATION {
            // SAFETY: a borrowed pointer the location never releases.
            pResource: unsafe { std::mem::transmute_copy(texture) },
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: index,
            },
        };
        let rows = |buffer: &ID3D12Resource, placed: D3D12_PLACED_SUBRESOURCE_FOOTPRINT| {
            D3D12_TEXTURE_COPY_LOCATION {
                // SAFETY: as above.
                pResource: unsafe { std::mem::transmute_copy(buffer) },
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: placed,
                },
            }
        };
        let (written, source, dest) = (
            D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
            D3D12_RESOURCE_STATE_COPY_DEST,
        );
        let moved = crate::host::transition;

        // The pattern into the twin, where a shader would write it.
        let fill = host.new_list().unwrap();
        // SAFETY: an open list and live resources.
        unsafe {
            let l = &fill.list;
            l.ResourceBarrier(&[moved(twin, written, dest)]);
            l.CopyTextureRegion(&whole(twin, 0), 0, 0, 0, &rows(&upload, pattern_rows), None);
            l.ResourceBarrier(&[moved(twin, dest, written)]);
        }
        fill.close();
        host.execute(&[&fill]);

        // A list in which a shader writes the twin: closing it copies the
        // twin into the stencil plane.
        let list = host.new_list().unwrap();
        assert!(list.twin_use(depth, true, true));
        list.twins_written();
        list.close();
        host.execute(&[&list]);

        // The stencil plane read back from the state the list left it in.
        let state = host_copy(depth).unwrap().state.load(Relaxed);
        let state = D3D12_RESOURCE_STATES(state as i32);
        let back = host.new_list().unwrap();
        // SAFETY: an open list and live resources; a whole subresource.
        unsafe {
            let l = &back.list;
            l.ResourceBarrier(&[moved(host_depth, state, source)]);
            let (to, from) = (rows(&readback, plane), whole(host_depth, 1));
            l.CopyTextureRegion(&to, 0, 0, 0, &from, None);
            l.ResourceBarrier(&[moved(host_depth, source, state)]);
        }
        back.close();
        host.execute(&[&back]);
        let mut mapped = std::ptr::null_mut();
        // SAFETY: the copies finished (execute waits); the buffer is 4096 bytes.
        let read = unsafe {
            readback
                .Map(
                    0,
                    Some(&D3D12_RANGE {
                        Begin: 0,
                        End: 4096,
                    }),
                    Some(&mut mapped),
                )
                .unwrap();
            std::slice::from_raw_parts(mapped.cast::<u8>(), 4096)
        };
        let pitch = plane.Footprint.RowPitch as usize;
        for row in 0..4 {
            let (at, from) = (row * pitch, row * 256);
            assert_eq!(read[at..at + 4], pattern[from..from + 4], "row {row}");
        }
    }

    /// The depth plane of a depth-stencil texture is drawn from the plane's
    /// twin when the list that wrote the twin closes: class numbers are put
    /// into the twin of a texture in the console's 16-bit depth-stencil
    /// format, and draws with the depth test "equal" at the depth the
    /// title's lighting computes for a class (the number times the float
    /// nearest 1/65535) then cover the texels of that class and no other.
    #[cfg(windows)]
    #[test]
    fn a_depth_planes_twin_is_drawn_into_the_plane() {
        use std::sync::atomic::Ordering::Relaxed;
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY;
        use windows::Win32::Graphics::Direct3D12::*;
        use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
        use windows::core::s;
        let Some(host) = crate::host::host() else {
            return; // no host GPU here
        };
        let null = std::ptr::null();
        const DEPTH_WRITE: u32 = 0x10;
        let desc = ResourceDesc {
            flags: 6,
            ..texture(4, 4, 118, 1)
        };
        let mut depth = null;
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &desc,
                DEPTH_WRITE,
                null.cast(),
                &RESOURCE_IIDS[0],
                &mut depth
            ),
            S_OK
        );
        let host_depth = host_resource(depth).unwrap();
        let Some(plane) = twin_plane_resource(depth) else {
            return; // these twins are turned off
        };
        // SAFETY: live resources.
        let (d, t) = unsafe { (host_depth.GetDesc(), plane.GetDesc()) };
        // R24G8 typeless with depth, R16 typeless with unordered access.
        assert_eq!((d.Format.0, d.Flags.0), (44, 2));
        assert_eq!((t.Format.0, t.Flags.0), (53, 4));

        // Four rows of four class numbers (16 bits each), 256 bytes apart
        // as copies place them. 128 and 129 are the classes whose 24-bit
        // depth lies nearest a half step.
        const CLASSES: [u16; 16] = [
            8, 65, 128, 129, 65, 8, 129, 128, 0, 255, 65, 64, 129, 130, 8, 65,
        ];
        let mut pattern = vec![0u8; 1024];
        for (i, class) in CLASSES.into_iter().enumerate() {
            let at = i / 4 * 256 + i % 4 * 2;
            pattern[at..at + 2].copy_from_slice(&class.to_le_bytes());
        }
        let (upload, data) = host.upload_buffer(1024).unwrap();
        // SAFETY: the buffer's mapping, 1024 bytes.
        unsafe { std::ptr::copy_nonoverlapping(pattern.as_ptr(), data, 1024) };
        let buffer = |kind: D3D12_HEAP_TYPE, state: D3D12_RESOURCE_STATES| {
            let heap = D3D12_HEAP_PROPERTIES {
                Type: kind,
                ..Default::default()
            };
            let desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: 1024,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                ..Default::default()
            };
            let mut made: Option<ID3D12Resource> = None;
            // SAFETY: plain creation of a buffer.
            unsafe {
                host.device
                    .CreateCommittedResource(
                        &heap,
                        D3D12_HEAP_FLAG_NONE,
                        &desc,
                        state,
                        None,
                        &mut made,
                    )
                    .unwrap();
            }
            made.unwrap()
        };
        let readback = buffer(D3D12_HEAP_TYPE_READBACK, D3D12_RESOURCE_STATE_COPY_DEST);
        // The colour target the draws cover: 4x4, 8 bits a texel.
        let target = {
            let heap = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_DEFAULT,
                ..Default::default()
            };
            let desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                Width: 4,
                Height: 4,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: DXGI_FORMAT(61),
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,
                ..Default::default()
            };
            let mut made: Option<ID3D12Resource> = None;
            // SAFETY: plain creation of a texture.
            unsafe {
                host.device
                    .CreateCommittedResource(
                        &heap,
                        D3D12_HEAP_FLAG_NONE,
                        &desc,
                        D3D12_RESOURCE_STATE_RENDER_TARGET,
                        None,
                        &mut made,
                    )
                    .unwrap();
            }
            made.unwrap()
        };
        let rows_of = |format: i32| D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
            Offset: 0,
            Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                Format: DXGI_FORMAT(format),
                Width: 4,
                Height: 4,
                Depth: 1,
                RowPitch: 256,
            },
        };
        // A texture's one subresource; rows of a buffer.
        let whole = |texture: &ID3D12Resource| D3D12_TEXTURE_COPY_LOCATION {
            // SAFETY: a borrowed pointer the location never releases.
            pResource: unsafe { std::mem::transmute_copy(texture) },
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: 0,
            },
        };
        let rows = |buffer: &ID3D12Resource, placed: D3D12_PLACED_SUBRESOURCE_FOOTPRINT| {
            D3D12_TEXTURE_COPY_LOCATION {
                // SAFETY: as above.
                pResource: unsafe { std::mem::transmute_copy(buffer) },
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: placed,
                },
            }
        };
        let (written, source, dest, tested, drawn) = (
            D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_DEPTH_WRITE,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        let moved = crate::host::transition;

        // The class numbers into the twin, where a shader would write them.
        let fill = host.new_list().unwrap();
        // SAFETY: an open list and live resources.
        unsafe {
            let l = &fill.list;
            l.ResourceBarrier(&[moved(plane, written, dest)]);
            l.CopyTextureRegion(&whole(plane), 0, 0, 0, &rows(&upload, rows_of(53)), None);
            l.ResourceBarrier(&[moved(plane, dest, written)]);
        }
        fill.close();
        host.execute(&[&fill]);

        // A list in which a shader writes the twins: closing it draws the
        // depth plane's twin into the plane.
        let list = host.new_list().unwrap();
        assert!(list.twin_use(depth, true, true));
        list.twins_written();
        list.close();
        host.execute(&[&list]);

        // The draws: a triangle over the target at the depth of a class,
        // as the title's lighting places its rectangles.
        const SOURCE: &str = r#"
#define RS "RootConstants(num32BitConstants = 1, b0)"
cbuffer Drawn : register(b0) { uint number; };
[RootSignature(RS)]
float4 corner(uint i : SV_VertexID) : SV_Position {
    float z = float(number) * asfloat(0x37800080);
    return float4(float(i & 1) * 4.0 - 1.0, float(i >> 1) * 4.0 - 1.0, z, 1.0);
}
float4 lit() : SV_Target { return 1.0; }
"#;
        let vs = crate::host::compile(SOURCE, s!("corner"), s!("vs_5_1")).unwrap();
        let ps = crate::host::compile(SOURCE, s!("lit"), s!("ps_5_1")).unwrap();
        let code = |bytes: &[u8]| D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytes.as_ptr().cast(),
            BytecodeLength: bytes.len(),
        };
        let mut blend = D3D12_BLEND_DESC::default();
        blend.RenderTarget[0].RenderTargetWriteMask = 0xf;
        let mut formats = [DXGI_FORMAT(0); 8];
        formats[0] = DXGI_FORMAT(61);
        // SAFETY: plain creation; the description points at live data.
        let (signature, pipeline) = unsafe {
            let signature: ID3D12RootSignature =
                host.device.CreateRootSignature(0, &vs[..]).unwrap();
            let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: std::mem::transmute_copy(&signature),
                VS: code(&vs[..]),
                PS: code(&ps[..]),
                BlendState: blend,
                SampleMask: u32::MAX,
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                // As the title's lighting: test "equal", nothing written.
                DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
                    DepthEnable: true.into(),
                    DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
                    DepthFunc: D3D12_COMPARISON_FUNC_EQUAL,
                    ..Default::default()
                },
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                RTVFormats: formats,
                DSVFormat: DXGI_FORMAT(45),
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                ..Default::default()
            };
            let pipeline: ID3D12PipelineState =
                host.device.CreateGraphicsPipelineState(&desc).unwrap();
            (signature, pipeline)
        };
        let state = host_copy(depth).unwrap().state.load(Relaxed);
        let state = D3D12_RESOURCE_STATES(state as i32);
        let colour = host.rtv(Some(&target), None);
        let depth_view = host.dsv(
            host_depth,
            Some(&D3D12_DEPTH_STENCIL_VIEW_DESC {
                Format: DXGI_FORMAT(45),
                ViewDimension: D3D12_DSV_DIMENSION_TEXTURE2D,
                ..Default::default()
            }),
        );
        let back = host.new_list().unwrap();
        // SAFETY: an open list and live resources and views.
        unsafe {
            let l = &back.list;
            if state != tested {
                l.ResourceBarrier(&[moved(host_depth, state, tested)]);
            }
            l.ClearRenderTargetView(colour, &[0.0; 4], None);
            l.SetGraphicsRootSignature(&signature);
            l.SetPipelineState(&pipeline);
            l.OMSetRenderTargets(1, Some(&colour), false, Some(&depth_view));
            l.RSSetViewports(&[D3D12_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: 4.0,
                Height: 4.0,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            }]);
            l.RSSetScissorRects(&[RECT {
                left: 0,
                top: 0,
                right: 4,
                bottom: 4,
            }]);
            l.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY(4));
            // Classes 65 and 129 are drawn; the others are not.
            for class in [65u32, 129] {
                l.SetGraphicsRoot32BitConstant(0, class, 0);
                l.DrawInstanced(3, 1, 0, 0);
            }
            l.ResourceBarrier(&[moved(&target, drawn, source)]);
            let (to, from) = (rows(&readback, rows_of(61)), whole(&target));
            l.CopyTextureRegion(&to, 0, 0, 0, &from, None);
        }
        back.close();
        host.execute(&[&back]);
        let mut mapped = std::ptr::null_mut();
        // SAFETY: the copy finished (execute waits); the buffer is 1024 bytes.
        let read = unsafe {
            readback
                .Map(
                    0,
                    Some(&D3D12_RANGE {
                        Begin: 0,
                        End: 1024,
                    }),
                    Some(&mut mapped),
                )
                .unwrap();
            std::slice::from_raw_parts(mapped.cast::<u8>(), 1024)
        };
        for (i, class) in CLASSES.into_iter().enumerate() {
            let covered = read[i / 4 * 256 + i % 4] != 0;
            let drawn = matches!(class, 65 | 129);
            assert_eq!(covered, drawn, "texel {i}, class {class}");
        }
    }

    /// The 1920x1080 R9G9B9E5 texture with render target and unordered
    /// access that the title makes in Series S mode, asked of the host
    /// device as it is, committed and reserved: what the driver says goes
    /// to the test's output, with the bytes the host gives the texture
    /// sampled and the stand-ins as targets. A host that refuses it gets
    /// R11G11B10_FLOAT ([`target_format`]), which it takes.
    #[cfg(windows)]
    #[test]
    fn the_host_is_asked_for_the_series_s_r9g9b9e5_target() {
        use windows::Win32::Graphics::Direct3D12::*;
        use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
        let Some(host) = crate::host::host() else {
            eprintln!("no host GPU; skipped");
            return;
        };
        let desc = |format: u32, flags: D3D12_RESOURCE_FLAGS| D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: 1920,
            Height: 1080,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT(format as i32),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Flags: flags,
            ..Default::default()
        };
        let both = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET | D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS;
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let committed = |d: &D3D12_RESOURCE_DESC| {
            let mut made: Option<ID3D12Resource> = None;
            // SAFETY: plain creation; the out pointer is ours.
            unsafe {
                host.device.CreateCommittedResource(
                    &heap,
                    D3D12_HEAP_FLAG_NONE,
                    d,
                    D3D12_RESOURCE_STATE_COMMON,
                    None,
                    &mut made,
                )
            }
        };
        // Reserved textures take the undefined 64 KiB swizzle.
        let reserved = |d: &D3D12_RESOURCE_DESC| {
            let d = D3D12_RESOURCE_DESC {
                Layout: D3D12_TEXTURE_LAYOUT_64KB_UNDEFINED_SWIZZLE,
                ..*d
            };
            let mut made: Option<ID3D12Resource> = None;
            // SAFETY: as above.
            unsafe { host.device.CreateReservedResource(&d, D3D12_RESOURCE_STATE_COMMON, None, &mut made) }
        };
        let mut support = D3D12_FEATURE_DATA_FORMAT_SUPPORT {
            Format: DXGI_FORMAT(67),
            ..Default::default()
        };
        // SAFETY: a plain query into our own structure.
        let queried = unsafe {
            host.device.CheckFeatureSupport(
                D3D12_FEATURE_FORMAT_SUPPORT,
                (&raw mut support).cast(),
                size_of_val(&support) as u32,
            )
        };
        let asked = committed(&desc(67, both));
        eprintln!(
            "{}: 1920x1080 R9G9B9E5 with render target and unordered access: committed {asked:?}, reserved {:?}; format support {:?} {:#x} {:#x}",
            host.adapter_note(),
            reserved(&desc(67, both)),
            queried,
            support.Support1.0,
            support.Support2.0
        );
        let stand_in = target_format(67);
        if asked.is_err() {
            assert_eq!(stand_in, 26, "a host that refuses the target gets R11G11B10_FLOAT");
        }
        committed(&desc(stand_in, both)).expect("the target in the format the host is given");
        // SAFETY: plain queries of valid descriptions.
        let bytes = |d: D3D12_RESOURCE_DESC| unsafe { host.device.GetResourceAllocationInfo(0, &[d]) }.SizeInBytes;
        let per = |d| {
            let b = bytes(d);
            format!("{b} bytes ({:.3} a pixel)", b as f64 / (1920.0 * 1080.0))
        };
        eprintln!(
            "R9G9B9E5 sampled {}, R11G11B10_FLOAT target {}, R16G16B16A16_FLOAT target {}",
            per(desc(67, D3D12_RESOURCE_FLAG_NONE)),
            per(desc(26, both)),
            per(desc(10, both))
        );
    }

    /// The same target made by the title (console flags 0x300005, with a
    /// clear value): its host copy is in the format [`target_format`] gives
    /// (R11G11B10_FLOAT where the host refuses R9G9B9E5) with both uses,
    /// made with the clear value in that format; a render target view and
    /// a pipeline's target formats that name R9G9B9E5 name it too, and a
    /// texture the title only samples stays R9G9B9E5. The card then clears
    /// the target through that view and one texel is read back.
    #[cfg(windows)]
    #[test]
    fn an_r9g9b9e5_target_gets_a_format_the_host_renders_to() {
        use std::sync::atomic::Ordering::Relaxed;
        use windows::Win32::Graphics::Direct3D12::*;
        use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;
        let Some(host) = crate::host::host() else {
            eprintln!("no host GPU; skipped");
            return;
        };
        let null = std::ptr::null();
        const RENDER_TARGET: u32 = 0x4;
        let desc = ResourceDesc {
            flags: 0x30_0005,
            ..texture(1920, 1080, 67, 1)
        };
        // The format, then the colour.
        let colour = [0.25f32, 0.5, 1.0, 0.0];
        let mut clear = [67u32; 5];
        for (word, c) in clear[1..].iter_mut().zip(colour) {
            *word = c.to_bits();
        }
        let mut target = null;
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &desc,
                RENDER_TARGET,
                clear.as_ptr().cast(),
                &RESOURCE_IIDS[0],
                &mut target
            ),
            S_OK
        );
        let stand_in = target_format(67);
        let made = host_resource(target).expect("a host copy of the target");
        // SAFETY: a live resource.
        let d = unsafe { made.GetDesc() };
        assert_eq!((d.Format.0 as u32, d.Flags.0), (stand_in, 5));
        assert_eq!(stands_in(target), stand_in != 67);
        assert_eq!(view_format(target), None, "a typed host copy");

        let mut sampled = null;
        let only_sampled = texture(1920, 1080, 67, 1);
        assert_eq!(
            create_committed(
                null,
                &PROPERTIES,
                0,
                &only_sampled,
                0,
                null.cast(),
                &RESOURCE_IIDS[0],
                &mut sampled
            ),
            S_OK
        );
        // SAFETY: a live resource.
        let s = unsafe { host_resource(sampled).expect("a sampled texture").GetDesc() };
        assert_eq!((s.Format.0, s.Flags.0), (67, 0));
        assert!(!stands_in(sampled));

        // The title's render target view in R9G9B9E5 (2D, mip 0).
        let mut slot = [0u64; 16];
        let handle = slot.as_mut_ptr() as u64;
        crate::descriptor::create_rtv(null, target, &[67, 4, 0, 0, 0, 0], handle);
        let view = crate::descriptor::rtv_record(handle).unwrap().desc().unwrap();
        assert_eq!(view.Format.0 as u32, stand_in);
        // A pipeline stream's target formats (subobject 15: eight formats,
        // then the count).
        let mut formats = [0u32; 10];
        (formats[0], formats[1], formats[9]) = (15, 67, 1);
        let mut state = crate::pipeline::host::GraphicsState::default();
        // SAFETY: a whole subobject of that type.
        unsafe { state.read(15, formats.as_ptr().cast()) };
        assert_eq!(state.targets.RTFormats[0].0 as u32, stand_in);

        // Cleared on the card through the view; texel (0, 0) read back.
        let readback = {
            let heap = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_READBACK,
                ..Default::default()
            };
            let desc = D3D12_RESOURCE_DESC {
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: 256,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                ..Default::default()
            };
            let mut made: Option<ID3D12Resource> = None;
            // SAFETY: plain creation of a readback buffer.
            unsafe {
                host.device
                    .CreateCommittedResource(
                        &heap,
                        D3D12_HEAP_FLAG_NONE,
                        &desc,
                        D3D12_RESOURCE_STATE_COPY_DEST,
                        None,
                        &mut made,
                    )
                    .unwrap();
            }
            made.unwrap()
        };
        let to = D3D12_TEXTURE_COPY_LOCATION {
            // SAFETY: a borrowed pointer the location never releases.
            pResource: unsafe { std::mem::transmute_copy(&readback) },
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                    Offset: 0,
                    Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                        Format: d.Format,
                        Width: 1,
                        Height: 1,
                        Depth: 1,
                        RowPitch: 256,
                    },
                },
            },
        };
        let from = D3D12_TEXTURE_COPY_LOCATION {
            // SAFETY: as above.
            pResource: unsafe { std::mem::transmute_copy(made) },
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                SubresourceIndex: 0,
            },
        };
        let texel = D3D12_BOX {
            left: 0,
            top: 0,
            front: 0,
            right: 1,
            bottom: 1,
            back: 1,
        };
        let state = D3D12_RESOURCE_STATES(host_copy(target).unwrap().state.load(Relaxed) as i32);
        let (drawn, source) = (D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_COPY_SOURCE);
        let moved = crate::host::transition;
        let rtv = host.rtv(Some(made), Some(&view));
        let list = host.new_list().unwrap();
        // SAFETY: an open list, live resources and a fresh view.
        unsafe {
            let l = &list.list;
            if state != drawn {
                l.ResourceBarrier(&[moved(made, state, drawn)]);
            }
            l.ClearRenderTargetView(rtv, &colour, None);
            l.ResourceBarrier(&[moved(made, drawn, source)]);
            l.CopyTextureRegion(&to, 0, 0, 0, &from, Some(&texel));
            l.ResourceBarrier(&[moved(made, source, state)]);
        }
        list.close();
        host.execute(&[&list]);
        // SAFETY: a plain query.
        assert!(unsafe { host.device.GetDeviceRemovedReason() }.is_ok());
        let mut mapped = std::ptr::null_mut();
        // SAFETY: the copy finished (execute waits); 4 bytes are read.
        let read = unsafe {
            readback
                .Map(0, Some(&D3D12_RANGE { Begin: 0, End: 4 }), Some(&mut mapped))
                .unwrap();
            mapped.cast::<u32>().read_unaligned()
        };
        if stand_in == 26 {
            // 0.25, 0.5 and 1.0 as 11-, 11- and 10-bit floats (biased
            // exponents 13, 14 and 15, no mantissa).
            assert_eq!(read, 13 << 6 | (14 << 6) << 11 | (15 << 5) << 22, "{read:#x}");
        }
    }

    #[test]
    fn committed_and_placed_resources_map_at_their_gpu_address() {
        let null = std::ptr::null();
        let mut committed = null;
        let hr = create_committed(
            null,
            &PROPERTIES,
            0,
            &buffer(256),
            0,
            null.cast(),
            &RESOURCE_IIDS[0],
            &mut committed,
        );
        assert_eq!(hr, S_OK);
        let mut data = std::ptr::null_mut();
        assert_eq!(
            slot::<Map>(committed, 0x40)(committed, 0, null.cast(), &mut data),
            S_OK
        );
        assert_eq!(slot::<Address>(committed, 0x58)(committed), data as u64);
        // SAFETY: the buffer is at least 256 zeroed bytes.
        assert_eq!(unsafe { *data.add(255) }, 0);
        assert_eq!(slot::<Release>(committed, 0x10)(committed), 0);

        let heap = heap(4 * KIB_64);
        let place = |offset, desc: &ResourceDesc, out: &mut This| {
            create_placed(
                null,
                heap,
                offset,
                desc,
                0,
                null.cast(),
                &RESOURCE_IIDS[0],
                out,
            )
        };
        let mut placed = null;
        assert_eq!(place(KIB_64, &buffer(KIB_64), &mut placed), S_OK);
        // SAFETY: a heap we made.
        let base = unsafe { (*heap.cast::<Heap>()).memory.ptr.as_ptr() } as u64;
        assert_eq!(slot::<Address>(placed, 0x58)(placed), base + KIB_64);
        let mut other = null;
        assert_eq!(
            place(4 * KIB_64, &buffer(1), &mut other),
            E_INVALIDARG,
            "past the end"
        );
        assert_eq!(
            place(100, &buffer(1), &mut other),
            E_INVALIDARG,
            "misaligned"
        );
        // The resource keeps the heap alive.
        assert_eq!(slot::<Release>(heap, 0x10)(heap), 1);
        assert_eq!(slot::<Release>(placed, 0x10)(placed), 0);
    }

    #[test]
    fn the_texture_pool_is_what_the_budget_leaves_and_never_under_its_least() {
        const MIB: u64 = 1 << 20;
        // A card within its budget: nine tenths of it, less what is not
        // textures.
        assert_eq!(pool_budget(8000 * MIB, 3000 * MIB, 1000 * MIB), (7200 - 2000) * MIB);
        // The run of 25/11, 9:41: 6245 MiB used for a budget of 4625, the
        // rule before made nothing; the pool keeps its least.
        assert_eq!(pool_budget(4625 * MIB, 6245 * MIB, 1200 * MIB), POOL_LEAST);
        assert_eq!(pressure(1200 * MIB, POOL_LEAST), PRESSURE_MOST, "full: the copies made from here on are the coarsest");
        // The run of 25/11, 10:08 (1280x720, textures lowered): 4246 used,
        // within the budget, and the pool is what the rule before left.
        let within = pool_budget(4753 * MIB, 4246 * MIB, 700 * MIB);
        assert!((731 * MIB..732 * MIB).contains(&within), "{within}");
        // No budget told, or nothing used yet.
        assert_eq!(pool_budget(0, 0, 0), POOL_LEAST);
        let empty = pool_budget(5335 * MIB, 400 * MIB, 0);
        assert!((4401 * MIB..4402 * MIB).contains(&empty), "{empty}");
    }

    #[test]
    fn new_copies_get_coarser_as_the_pool_fills() {
        const MIB: u64 = 1 << 20;
        let pool = 512 * MIB;
        assert_eq!(pressure(0, pool), 0);
        assert_eq!(pressure(127 * MIB, pool), 0);
        assert_eq!(pressure(128 * MIB, pool), 1);
        assert_eq!(pressure(256 * MIB, pool), 2);
        assert_eq!(pressure(384 * MIB, pool), 3);
        assert_eq!(pressure(4096 * MIB, pool), PRESSURE_MOST, "past the pool, no coarser than the coarsest");
        assert_eq!(pressure(1, 0), PRESSURE_MOST, "no pool at all");
        // What the quarters hold of the title's textures: 1 + 4 + 16 + 64
        // quarters of the pool.
        let held: u64 = (0..4).map(|q| (pool / 4) << (2 * q)).sum();
        assert_eq!(held, 10_880 * MIB);
    }

    #[test]
    fn texture_floor_scales_with_vram_budget() {
        const GIB: u64 = 1 << 30;
        assert_eq!(texture_floor(None), 2 * GIB);
        assert_eq!(texture_floor(Some(4 * GIB)), GIB);
        assert_eq!(texture_floor(Some(6 * GIB)), 6 * GIB / 4);
        assert_eq!(texture_floor(Some(8 * GIB)), 2 * GIB);
        assert_eq!(texture_floor(Some(12 * GIB)), 2 * GIB);
        assert_eq!(texture_floor(Some(24 * GIB)), 2 * GIB);
        assert_eq!(texture_floor(Some(32 * GIB)), 2 * GIB);
    }
}
