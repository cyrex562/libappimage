use std::io::{Read, Write};
use std::convert::TryInto;

/// Size of MD5 hash in bytes
pub const MD5_HASH_SIZE: usize = 16;

/// MD5 hash type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Md5Hash([u8; MD5_HASH_SIZE]);

impl Md5Hash {
    /// Create a new MD5 hash from bytes
    pub fn new(bytes: [u8; MD5_HASH_SIZE]) -> Self {
        Self(bytes)
    }

    /// Get the hash as bytes
    pub fn as_bytes(&self) -> &[u8; MD5_HASH_SIZE] {
        &self.0
    }

    /// Convert the hash to a hex string
    pub fn to_hex(&self) -> String {
        self.0.iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// MD5 context for incremental hashing
#[derive(Debug)]
pub struct Md5Context {
    lo: u32,
    hi: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    buffer: [u8; 64],
    block: [u32; 16],
}

impl Md5Context {
    /// Create a new MD5 context
    pub fn new() -> Self {
        Self {
            lo: 0,
            hi: 0,
            a: 0x67452301,
            b: 0xefcdab89,
            c: 0x98badcfe,
            d: 0x10325476,
            buffer: [0; 64],
            block: [0; 16],
        }
    }

    /// Update the context with data
    pub fn update(&mut self, data: &[u8]) {
        let mut saved_lo = self.lo;
        self.lo = (saved_lo + data.len() as u32) & 0x1fffffff;
        if self.lo < saved_lo {
            self.hi += 1;
        }
        self.hi += (data.len() as u32) >> 29;

        let mut used = saved_lo & 0x3f;
        let mut remaining = data;

        if used > 0 {
            let free = 64 - used;
            if data.len() < free {
                self.buffer[used..used + data.len()].copy_from_slice(data);
                return;
            }
            self.buffer[used..].copy_from_slice(&data[..free]);
            remaining = &data[free..];
            self.transform(&self.buffer);
        }

        while remaining.len() >= 64 {
            self.transform(remaining);
            remaining = &remaining[64..];
        }

        self.buffer[..remaining.len()].copy_from_slice(remaining);
    }

    /// Finalize the context and get the hash
    pub fn finalize(mut self) -> Md5Hash {
        let mut used = self.lo & 0x3f;
        self.buffer[used] = 0x80;
        used += 1;

        let free = 64 - used;
        if free < 8 {
            self.buffer[used..].fill(0);
            self.transform(&self.buffer);
            used = 0;
        }

        self.buffer[used..56].fill(0);
        self.lo <<= 3;
        self.buffer[56] = self.lo as u8;
        self.buffer[57] = (self.lo >> 8) as u8;
        self.buffer[58] = (self.lo >> 16) as u8;
        self.buffer[59] = (self.lo >> 24) as u8;
        self.buffer[60] = self.hi as u8;
        self.buffer[61] = (self.hi >> 8) as u8;
        self.buffer[62] = (self.hi >> 16) as u8;
        self.buffer[63] = (self.hi >> 24) as u8;

        self.transform(&self.buffer);

        let mut hash = [0u8; MD5_HASH_SIZE];
        hash[0] = self.a as u8;
        hash[1] = (self.a >> 8) as u8;
        hash[2] = (self.a >> 16) as u8;
        hash[3] = (self.a >> 24) as u8;
        hash[4] = self.b as u8;
        hash[5] = (self.b >> 8) as u8;
        hash[6] = (self.b >> 16) as u8;
        hash[7] = (self.b >> 24) as u8;
        hash[8] = self.c as u8;
        hash[9] = (self.c >> 8) as u8;
        hash[10] = (self.c >> 16) as u8;
        hash[11] = (self.c >> 24) as u8;
        hash[12] = self.d as u8;
        hash[13] = (self.d >> 8) as u8;
        hash[14] = (self.d >> 16) as u8;
        hash[15] = (self.d >> 24) as u8;

        Md5Hash(hash)
    }

    /// Transform the buffer
    fn transform(&mut self, data: &[u8]) {
        let mut a = self.a;
        let mut b = self.b;
        let mut c = self.c;
        let mut d = self.d;

        // Load block
        for i in 0..16 {
            self.block[i] = u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap());
        }

        // Round 1
        a = f(a, b, c, d, self.block[0], 0xd76aa478, 7);
        d = f(d, a, b, c, self.block[1], 0xe8c7b756, 12);
        c = f(c, d, a, b, self.block[2], 0x242070db, 17);
        b = f(b, c, d, a, self.block[3], 0xc1bdceee, 22);
        a = f(a, b, c, d, self.block[4], 0xf57c0faf, 7);
        d = f(d, a, b, c, self.block[5], 0x4787c62a, 12);
        c = f(c, d, a, b, self.block[6], 0xa8304613, 17);
        b = f(b, c, d, a, self.block[7], 0xfd469501, 22);
        a = f(a, b, c, d, self.block[8], 0x698098d8, 7);
        d = f(d, a, b, c, self.block[9], 0x8b44f7af, 12);
        c = f(c, d, a, b, self.block[10], 0xffff5bb1, 17);
        b = f(b, c, d, a, self.block[11], 0x895cd7be, 22);
        a = f(a, b, c, d, self.block[12], 0x6b901122, 7);
        d = f(d, a, b, c, self.block[13], 0xfd987193, 12);
        c = f(c, d, a, b, self.block[14], 0xa679438e, 17);
        b = f(b, c, d, a, self.block[15], 0x49b40821, 22);

        // Round 2
        a = g(a, b, c, d, self.block[1], 0xf61e2562, 5);
        d = g(d, a, b, c, self.block[6], 0xc040b340, 9);
        c = g(c, d, a, b, self.block[11], 0x265e5a51, 14);
        b = g(b, c, d, a, self.block[0], 0xe9b6c7aa, 20);
        a = g(a, b, c, d, self.block[5], 0xd62f105d, 5);
        d = g(d, a, b, c, self.block[10], 0x02441453, 9);
        c = g(c, d, a, b, self.block[15], 0xd8a1e681, 14);
        b = g(b, c, d, a, self.block[4], 0xe7d3fbc8, 20);
        a = g(a, b, c, d, self.block[9], 0x21e1cde6, 5);
        d = g(d, a, b, c, self.block[14], 0xc33707d6, 9);
        c = g(c, d, a, b, self.block[3], 0xf4d50d87, 14);
        b = g(b, c, d, a, self.block[8], 0x455a14ed, 20);
        a = g(a, b, c, d, self.block[13], 0xa9e3e905, 5);
        d = g(d, a, b, c, self.block[2], 0xfcefa3f8, 9);
        c = g(c, d, a, b, self.block[7], 0x676f02d9, 14);
        b = g(b, c, d, a, self.block[12], 0x8d2a4c8a, 20);

        // Round 3
        a = h(a, b, c, d, self.block[5], 0xfffa3942, 4);
        d = h(d, a, b, c, self.block[8], 0x8771f681, 11);
        c = h(c, d, a, b, self.block[11], 0x6d9d6122, 16);
        b = h(b, c, d, a, self.block[14], 0xfde5380c, 23);
        a = h(a, b, c, d, self.block[1], 0xa4beea44, 4);
        d = h(d, a, b, c, self.block[4], 0x4bdecfa9, 11);
        c = h(c, d, a, b, self.block[7], 0xf6bb4b60, 16);
        b = h(b, c, d, a, self.block[10], 0xbebfbc70, 23);
        a = h(a, b, c, d, self.block[13], 0x289b7ec6, 4);
        d = h(d, a, b, c, self.block[0], 0xeaa127fa, 11);
        c = h(c, d, a, b, self.block[3], 0xd4ef3085, 16);
        b = h(b, c, d, a, self.block[6], 0x04881d05, 23);
        a = h(a, b, c, d, self.block[9], 0xd9d4d039, 4);
        d = h(d, a, b, c, self.block[12], 0xe6db99e5, 11);
        c = h(c, d, a, b, self.block[15], 0x1fa27cf8, 16);
        b = h(b, c, d, a, self.block[2], 0xc4ac5665, 23);

        // Round 4
        a = i(a, b, c, d, self.block[0], 0xf4292244, 6);
        d = i(d, a, b, c, self.block[7], 0x432aff97, 10);
        c = i(c, d, a, b, self.block[14], 0xab9423a7, 15);
        b = i(b, c, d, a, self.block[5], 0xfc93a039, 21);
        a = i(a, b, c, d, self.block[12], 0x655b59c3, 6);
        d = i(d, a, b, c, self.block[3], 0x8f0ccc92, 10);
        c = i(c, d, a, b, self.block[10], 0xffeff47d, 15);
        b = i(b, c, d, a, self.block[1], 0x85845dd1, 21);
        a = i(a, b, c, d, self.block[8], 0x6fa87e4f, 6);
        d = i(d, a, b, c, self.block[15], 0xfe2ce6e0, 10);
        c = i(c, d, a, b, self.block[6], 0xa3014314, 15);
        b = i(b, c, d, a, self.block[13], 0x4e0811a1, 21);
        a = i(a, b, c, d, self.block[4], 0xf7537e82, 6);
        d = i(d, a, b, c, self.block[11], 0xbd3af235, 10);
        c = i(c, d, a, b, self.block[2], 0x2ad7d2bb, 15);
        b = i(b, c, d, a, self.block[9], 0xeb86d391, 21);

        self.a = self.a.wrapping_add(a);
        self.b = self.b.wrapping_add(b);
        self.c = self.c.wrapping_add(c);
        self.d = self.d.wrapping_add(d);
    }
}

/// Calculate MD5 hash of data
pub fn md5(data: &[u8]) -> Md5Hash {
    let mut ctx = Md5Context::new();
    ctx.update(data);
    ctx.finalize()
}

/// Calculate MD5 hash of a reader
pub fn md5_reader<R: Read>(mut reader: R) -> std::io::Result<Md5Hash> {
    let mut ctx = Md5Context::new();
    let mut buffer = [0; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        ctx.update(&buffer[..n]);
    }
    Ok(ctx.finalize())
}

/// Helper functions for MD5 rounds
fn f(x: u32, y: u32, z: u32, w: u32, m: u32, t: u32, s: u32) -> u32 {
    let x = x.wrapping_add(f_round(x, y, z).wrapping_add(m).wrapping_add(t));
    (x.rotate_left(s)).wrapping_add(y)
}

fn g(x: u32, y: u32, z: u32, w: u32, m: u32, t: u32, s: u32) -> u32 {
    let x = x.wrapping_add(g_round(x, y, z).wrapping_add(m).wrapping_add(t));
    (x.rotate_left(s)).wrapping_add(y)
}

fn h(x: u32, y: u32, z: u32, w: u32, m: u32, t: u32, s: u32) -> u32 {
    let x = x.wrapping_add(h_round(x, y, z).wrapping_add(m).wrapping_add(t));
    (x.rotate_left(s)).wrapping_add(y)
}

fn i(x: u32, y: u32, z: u32, w: u32, m: u32, t: u32, s: u32) -> u32 {
    let x = x.wrapping_add(i_round(x, y, z).wrapping_add(m).wrapping_add(t));
    (x.rotate_left(s)).wrapping_add(y)
}

fn f_round(x: u32, y: u32, z: u32) -> u32 {
    (z) ^ ((x) & ((y) ^ (z)))
}

fn g_round(x: u32, y: u32, z: u32) -> u32 {
    (y) ^ ((z) & ((x) ^ (y)))
}

fn h_round(x: u32, y: u32, z: u32) -> u32 {
    (x) ^ (y) ^ (z)
}

fn i_round(x: u32, y: u32, z: u32) -> u32 {
    (y) ^ ((x) | !(z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5() {
        // Test empty string
        assert_eq!(
            md5(b"").to_hex(),
            "d41d8cd98f00b204e9800998ecf8427e"
        );

        // Test "a"
        assert_eq!(
            md5(b"a").to_hex(),
            "0cc175b9c0f1b6a831c399e269772661"
        );

        // Test "abc"
        assert_eq!(
            md5(b"abc").to_hex(),
            "900150983cd24fb0d6963f7d28e17f72"
        );

        // Test "message digest"
        assert_eq!(
            md5(b"message digest").to_hex(),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );

        // Test "abcdefghijklmnopqrstuvwxyz"
        assert_eq!(
            md5(b"abcdefghijklmnopqrstuvwxyz").to_hex(),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );

        // Test "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        assert_eq!(
            md5(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789").to_hex(),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );

        // Test "12345678901234567890123456789012345678901234567890123456789012345678901234567890"
        assert_eq!(
            md5(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890").to_hex(),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn test_md5_reader() {
        use std::io::Cursor;
        let data = b"Hello, World!";
        let reader = Cursor::new(data);
        let hash = md5_reader(reader).unwrap();
        assert_eq!(hash.to_hex(), "65a8e27d8879283831b664bd8b7f0ad4");
    }
} 