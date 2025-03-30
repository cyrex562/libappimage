/// Swap bytes in a 16-bit value
/// 
/// # Arguments
/// * `value` - The 16-bit value to swap
/// 
/// # Returns
/// * `u16` - The value with bytes swapped
pub fn bswap_16(value: u16) -> u16 {
    ((value & 0xff) << 8) | (value >> 8)
}

/// Swap bytes in a 32-bit value
/// 
/// # Arguments
/// * `value` - The 32-bit value to swap
/// 
/// # Returns
/// * `u32` - The value with bytes swapped
pub fn bswap_32(value: u32) -> u32 {
    ((bswap_16((value & 0xffff) as u16) as u32) << 16) |
    (bswap_16((value >> 16) as u16) as u32)
}

/// Swap bytes in a 64-bit value
/// 
/// # Arguments
/// * `value` - The 64-bit value to swap
/// 
/// # Returns
/// * `u64` - The value with bytes swapped
pub fn bswap_64(value: u64) -> u64 {
    ((bswap_32((value & 0xffffffff) as u32) as u64) << 32) |
    (bswap_32((value >> 32) as u32) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bswap_16() {
        assert_eq!(bswap_16(0x1234), 0x3412);
        assert_eq!(bswap_16(0xABCD), 0xCDAB);
        assert_eq!(bswap_16(0xFFFF), 0xFFFF);
        assert_eq!(bswap_16(0x0000), 0x0000);
    }

    #[test]
    fn test_bswap_32() {
        assert_eq!(bswap_32(0x12345678), 0x78563412);
        assert_eq!(bswap_32(0xABCDEF01), 0x01EFCDAB);
        assert_eq!(bswap_32(0xFFFFFFFF), 0xFFFFFFFF);
        assert_eq!(bswap_32(0x00000000), 0x00000000);
    }

    #[test]
    fn test_bswap_64() {
        assert_eq!(bswap_64(0x1234567890ABCDEF), 0xEFCDAB9078563412);
        assert_eq!(bswap_64(0xABCDEF0123456789), 0x8967452301EFCDAB);
        assert_eq!(bswap_64(0xFFFFFFFFFFFFFFFF), 0xFFFFFFFFFFFFFFFF);
        assert_eq!(bswap_64(0x0000000000000000), 0x0000000000000000);
    }
} 