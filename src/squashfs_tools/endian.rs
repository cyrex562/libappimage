use std::cfg;
use std::mem;

/// Endianness constants and utilities for cross-platform compatibility
pub const BIG_ENDIAN: u32 = 0x12345678;
pub const LITTLE_ENDIAN: u32 = 0x78563412;
pub const BYTE_ORDER: u32 = if cfg!(target_endian = "big") { BIG_ENDIAN } else { LITTLE_ENDIAN };

/// Type alias for byte order to maintain compatibility with C code
pub type ByteOrder = u32;

/// Constants for byte order values
pub const BYTE_ORDER_BE: ByteOrder = BIG_ENDIAN;
pub const BYTE_ORDER_LE: ByteOrder = LITTLE_ENDIAN;

/// Helper function to check if the system is big-endian
pub fn is_big_endian() -> bool {
    cfg!(target_endian = "big")
}

/// Helper function to check if the system is little-endian
pub fn is_little_endian() -> bool {
    cfg!(target_endian = "little")
}

/// Helper function to get the system's byte order
pub fn get_byte_order() -> ByteOrder {
    if is_big_endian() {
        BYTE_ORDER_BE
    } else {
        BYTE_ORDER_LE
    }
}

/// Swap a 16-bit value between big and little endian
#[inline]
pub fn swap_le16(src: &[u8], dest: &mut [u8]) {
    dest[0] = src[1];
    dest[1] = src[0];
}

/// Swap a 32-bit value between big and little endian
#[inline]
pub fn swap_le32(src: &[u8], dest: &mut [u8]) {
    dest[0] = src[3];
    dest[1] = src[2];
    dest[2] = src[1];
    dest[3] = src[0];
}

/// Swap a 64-bit value between big and little endian
#[inline]
pub fn swap_le64(src: &[u8], dest: &mut [u8]) {
    dest[0] = src[7];
    dest[1] = src[6];
    dest[2] = src[5];
    dest[3] = src[4];
    dest[4] = src[3];
    dest[5] = src[2];
    dest[6] = src[1];
    dest[7] = src[0];
}

/// Swap a 16-bit value in-place between big and little endian
#[inline]
pub fn inswap_le16(num: u16) -> u16 {
    (num >> 8) | ((num & 0xff) << 8)
}

/// Swap a 32-bit value in-place between big and little endian
#[inline]
pub fn inswap_le32(num: u32) -> u32 {
    (num >> 24) |
    ((num & 0xff0000) >> 8) |
    ((num & 0xff00) << 8) |
    ((num & 0xff) << 24)
}

/// Swap a 64-bit value in-place between big and little endian
#[inline]
pub fn inswap_le64(num: i64) -> i64 {
    let num = num as u64;
    ((num >> 56) |
    ((num & 0xff000000000000) >> 40) |
    ((num & 0xff0000000000) >> 24) |
    ((num & 0xff00000000) >> 8) |
    ((num & 0xff000000) << 8) |
    ((num & 0xff0000) << 24) |
    ((num & 0xff00) << 40) |
    ((num & 0xff) << 56)) as i64
}

/// Swap a slice of 16-bit values between big and little endian
pub fn swap_le16_num(src: &[u8], dest: &mut [u8], n: usize) {
    for i in 0..n {
        let src_offset = i * 2;
        let dest_offset = i * 2;
        swap_le16(&src[src_offset..src_offset + 2], &mut dest[dest_offset..dest_offset + 2]);
    }
}

/// Swap a slice of 32-bit values between big and little endian
pub fn swap_le32_num(src: &[u8], dest: &mut [u8], n: usize) {
    for i in 0..n {
        let src_offset = i * 4;
        let dest_offset = i * 4;
        swap_le32(&src[src_offset..src_offset + 4], &mut dest[dest_offset..dest_offset + 4]);
    }
}

/// Swap a slice of 64-bit values between big and little endian
pub fn swap_le64_num(src: &[u8], dest: &mut [u8], n: usize) {
    for i in 0..n {
        let src_offset = i * 8;
        let dest_offset = i * 8;
        swap_le64(&src[src_offset..src_offset + 8], &mut dest[dest_offset..dest_offset + 8]);
    }
}

/// Swap a slice of 16-bit values in-place between big and little endian
pub fn inswap_le16_num(slice: &mut [u16]) {
    for value in slice.iter_mut() {
        *value = inswap_le16(*value);
    }
}

/// Swap a slice of 32-bit values in-place between big and little endian
pub fn inswap_le32_num(slice: &mut [u32]) {
    for value in slice.iter_mut() {
        *value = inswap_le32(*value);
    }
}

/// Swap a slice of 64-bit values in-place between big and little endian
pub fn inswap_le64_num(slice: &mut [i64]) {
    for value in slice.iter_mut() {
        *value = inswap_le64(*value);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_order() {
        let byte_order = get_byte_order();
        assert!(byte_order == BYTE_ORDER_BE || byte_order == BYTE_ORDER_LE);
    }

    #[test]
    fn test_endianness() {
        // Only one of these should be true
        assert_ne!(is_big_endian(), is_little_endian());
    }

    #[test]
    fn test_byte_order_consistency() {
        let byte_order = get_byte_order();
        assert_eq!(
            byte_order,
            if is_big_endian() { BYTE_ORDER_BE } else { BYTE_ORDER_LE }
        );
    }

    #[test]
    fn test_swap_le16() {
        let src = [0x12, 0x34];
        let mut dest = [0; 2];
        swap_le16(&src, &mut dest);
        assert_eq!(dest, [0x34, 0x12]);
    }

    #[test]
    fn test_swap_le32() {
        let src = [0x12, 0x34, 0x56, 0x78];
        let mut dest = [0; 4];
        swap_le32(&src, &mut dest);
        assert_eq!(dest, [0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_swap_le64() {
        let src = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let mut dest = [0; 8];
        swap_le64(&src, &mut dest);
        assert_eq!(dest, [0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_inswap_le16() {
        assert_eq!(inswap_le16(0x1234), 0x3412);
    }

    #[test]
    fn test_inswap_le32() {
        assert_eq!(inswap_le32(0x12345678), 0x78563412);
    }

    #[test]
    fn test_inswap_le64() {
        assert_eq!(inswap_le64(0x1234567890ABCDEF), 0xEFCDAB9078563412);
    }

    #[test]
    fn test_swap_le16_num() {
        let src = [0x12, 0x34, 0x56, 0x78];
        let mut dest = [0; 4];
        swap_le16_num(&src, &mut dest, 2);
        assert_eq!(dest, [0x34, 0x12, 0x78, 0x56]);
    }

    #[test]
    fn test_swap_le32_num() {
        let src = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let mut dest = [0; 8];
        swap_le32_num(&src, &mut dest, 2);
        assert_eq!(dest, [0x78, 0x56, 0x34, 0x12, 0xF0, 0xDE, 0xBC, 0x9A]);
    }

    #[test]
    fn test_swap_le64_num() {
        let src = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88
        ];
        let mut dest = [0; 16];
        swap_le64_num(&src, &mut dest, 2);
        assert_eq!(dest, [
            0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12,
            0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11
        ]);
    }

    #[test]
    fn test_inswap_le16_num() {
        let mut slice = [0x1234, 0x5678];
        inswap_le16_num(&mut slice);
        assert_eq!(slice, [0x3412, 0x7856]);
    }

    #[test]
    fn test_inswap_le32_num() {
        let mut slice = [0x12345678, 0x9ABCDEF0];
        inswap_le32_num(&mut slice);
        assert_eq!(slice, [0x78563412, 0xF0DEBC9A]);
    }

    #[test]
    fn test_inswap_le64_num() {
        let mut slice = [0x1234567890ABCDEF, 0x1122334455667788];
        inswap_le64_num(&mut slice);
        assert_eq!(slice, [0xEFCDAB9078563412, 0x8877665544332211]);
    }
} 