//! The **descriptor-words table**: the console descriptor dwords a title's
//! shader would read at each view-heap slot, which the host's own views do
//! not expose. Translated shaders that read a heap descriptor's dwords as
//! numbers (sizes, levels, the image type, a buffer's stride or record
//! count) instead of using it as a resource read them here
//! ([`console_words`] gives the encoding; `victory-gpu` fills the table).
//!
//! Layout: the root SRV [`super::SLOT_WORDS`], [`WORDS`] dwords per heap
//! slot, so the console heap address of a descriptor dword (32-byte slots)
//! is its byte offset in the table. Slots without a view hold zeros.
/// Dwords per slot: the console's 32-byte descriptor slot.
pub const WORDS: u32 = 8;

/// GFX10 image resource types (`SQ_RSRC_IMG_*`, T# dword 3 bits 28-31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    D1 = 8,
    D2 = 9,
    D3 = 10,
    Cube = 11,
    D1Array = 12,
    D2Array = 13,
    D2Msaa = 14,
    D2MsaaArray = 15,
}

/// What a view-heap slot holds, as the descriptor words describe it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// No view (or one the host could not make): zeros.
    None,
    /// A buffer view: its title GPU address, the console stride field
    /// (0 for a raw view) and its record count (elements of the stride, or
    /// bytes for stride 0).
    Buffer {
        address: u64,
        stride: u32,
        records: u32,
    },
    /// An image view as the host makes it: its first level's size, depth
    /// (3D) or layers (arrays: faces for cubes) and its level count (or
    /// sample count for MSAA), and the title GPU address of its resource.
    Image {
        ty: ImageType,
        width: u32,
        height: u32,
        depth: u32,
        levels: u32,
        address: u64,
    },
}

/// The identity channel selects (X, Y, Z, W) of V# and T# dword 3.
const IDENTITY: u32 = 4 | 5 << 3 | 6 << 6 | 7 << 9;

/// The GFX10 descriptor dwords of `shape`, in the fields
/// `Lower::descriptor_data` rebuilds from a used descriptor's host view:
/// buffers: dword 0-1 address, dword 1 bits 16-29 stride, dword 2 record
/// count, dword 3 identity selects; images: dword 0-1 address (256-byte
/// units), dword 1 bits 30-31 and dword 2 bits 0-13 width - 1, dword 2 bits
/// 14-29 height - 1, dword 3 identity selects, last level (or log2 samples)
/// at 16, type at 28, dword 4 depth or layers - 1.
// shortcut: the base level is 0 (the host view starts at its own level 0,
// so sizes a shader shifts by it come out right); formats, tiling, minimum
// LOD and the metadata dwords read 0.
pub fn console_words(shape: &Shape) -> [u32; WORDS as usize] {
    let mut w = [0u32; WORDS as usize];
    match *shape {
        Shape::None => {}
        Shape::Buffer {
            address,
            stride,
            records,
        } => {
            w[0] = address as u32;
            w[1] = (address >> 32) as u32 & 0xFFFF | (stride & 0x3FFF) << 16;
            w[2] = records;
            w[3] = IDENTITY;
        }
        Shape::Image {
            ty,
            width,
            height,
            depth,
            levels,
            address,
        } => {
            let less = |x: u32| x.max(1) - 1;
            let (wm, hm) = (less(width), less(height));
            let last = match ty {
                ImageType::D2Msaa | ImageType::D2MsaaArray => levels.max(1).ilog2(),
                _ => less(levels),
            };
            w[0] = (address >> 8) as u32;
            w[1] = (address >> 40) as u32 & 0xFF | (wm & 3) << 30;
            w[2] = (wm >> 2) & 0x3FFF | (hm & 0xFFFF) << 14;
            w[3] = IDENTITY | (last & 0xF) << 16 | (ty as u32) << 28;
            w[4] = less(depth) & 0x1FFF;
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_words_hold_size_levels_and_type() {
        let w = console_words(&Shape::Image {
            ty: ImageType::D2,
            width: 8,
            height: 4,
            depth: 1,
            levels: 3,
            address: 0x12_3456_7800,
        });
        // Width - 1 = 7: low two bits in dword 1, the rest in dword 2.
        assert_eq!(w[1] >> 30, 3);
        assert_eq!(w[2] & 0x3FFF, 1);
        assert_eq!(w[2] >> 14 & 0xFFFF, 3);
        assert_eq!(w[3], 0xFAC | 2 << 16 | 9 << 28);
        assert_eq!(w[4], 0);
        assert_eq!(w[0], 0x1234_5678);
        assert_eq!(w[1] & 0xFF, 0);
    }

    #[test]
    fn buffer_words_hold_address_stride_and_records() {
        let w = console_words(&Shape::Buffer {
            address: 0x0001_2345_6780,
            stride: 24,
            records: 100,
        });
        assert_eq!(w[0], 0x2345_6780);
        assert_eq!(w[1], 1 | 24 << 16);
        assert_eq!(w[2], 100);
        assert_eq!(w[3], 0xFAC);
        assert_eq!(console_words(&Shape::None), [0; 8]);
    }
}
