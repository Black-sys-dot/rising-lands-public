//! Owned, immutable original CMP banks. No filesystem or Python in rendering.
use super::render::{Error, Result};

#[derive(Clone)]
pub struct Sprite {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub mask: Vec<u8>,
    pub raw: Vec<u8>,
}
pub struct Assets {
    pub sprites: Vec<Sprite>,
    pub palette: [u8; 768],
    pub shadow: [u8; 256],
    pub light: [u8; 256],
}
fn word(b: &[u8], p: usize) -> Result<usize> {
    Ok(u16::from_le_bytes(
        b.get(p..p + 2)
            .ok_or(Error::Invalid("truncated CMP"))?
            .try_into()
            .unwrap(),
    ) as usize)
}
fn dword(b: &[u8], p: usize) -> Result<usize> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .ok_or(Error::Invalid("truncated CMP"))?
            .try_into()
            .unwrap(),
    ) as usize)
}
impl Assets {
    pub fn load(blobs: &[&[u8]]) -> Result<Self> {
        if blobs.len() != 9 {
            return Err(Error::Invalid(
                "expected six CMP banks, PIE, shadow and lighting tables",
            ));
        }
        for (b, digest) in blobs.iter().zip(HASHES) {
            if sha256(b) != digest {
                return Err(Error::Invalid("original asset SHA-256 mismatch"));
            }
        }
        let mut sprites = Vec::new();
        for (&b, count) in blobs[..6].iter().zip([1345, 2848, 915, 1920, 611, 4517]) {
            if word(b, 0)? != count {
                return Err(Error::Invalid("CMP count"));
            }
            let mut p = 2;
            for _ in 0..count {
                let w = word(b, p)?;
                let h = word(b, p + 2)?;
                let n = dword(b, p + 4)?;
                p += 8;
                if w == 0 || h == 0 || w * h > 16_777_216 {
                    return Err(Error::Invalid("CMP dimensions"));
                }
                let raw = b
                    .get(p..p + n)
                    .ok_or(Error::Invalid("CMP payload"))?
                    .to_vec();
                p += n;
                let mut pixels = vec![0; w * h];
                let mut mask = vec![0; w * h];
                let mut q = 0;
                for y in 0..h {
                    let mut x = 0;
                    while x < w {
                        let c = *raw.get(q).ok_or(Error::Invalid("CMP command"))?;
                        q += 1;
                        if c == 128 {
                            return Err(Error::Invalid("CMP 0x80"));
                        }
                        let n = if c > 128 {
                            256 - c as usize
                        } else {
                            c as usize
                        };
                        if x + n > w {
                            return Err(Error::Invalid("CMP row overflow"));
                        }
                        if c > 128 {
                            pixels[y * w + x..y * w + x + n].copy_from_slice(
                                raw.get(q..q + n).ok_or(Error::Invalid("CMP literal"))?,
                            );
                            mask[y * w + x..y * w + x + n].fill(255);
                            q += n;
                        }
                        x += n;
                    }
                }
                if raw[q..].iter().any(|&x| x != 0) {
                    return Err(Error::Invalid("CMP trailing data"));
                }
                sprites.push(Sprite {
                    width: w,
                    height: h,
                    pixels,
                    mask,
                    raw,
                });
            }
            if p != b.len() {
                return Err(Error::Invalid("CMP trailing bank data"));
            }
        }
        let pie = blobs[6];
        if pie.len() < 777 || &pie[..3] != b"Pie" || word(pie, 7)? != 256 {
            return Err(Error::Invalid("PIE palette"));
        }
        let mut palette = [0; 768];
        for (d, s) in palette[3..].iter_mut().zip(&pie[12..777]) {
            *d = s.wrapping_shl(2);
        }
        Ok(Self {
            sprites,
            palette,
            shadow: blobs[7]
                .try_into()
                .map_err(|_| Error::Invalid("shadow table"))?,
            light: blobs[8]
                .try_into()
                .map_err(|_| Error::Invalid("lighting table"))?,
        })
    }
    pub fn sprite(&self, id: u32) -> Result<&Sprite> {
        self.sprites
            .get(id.wrapping_sub(1) as usize)
            .ok_or(Error::Invalid("sprite ID outside original catalog"))
    }
}

// SHA-256, FIPS 180-4. Used only at creation; rendering retains the verified bytes.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut data = bytes.to_vec();
    data.push(128);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&(bytes.len() as u64 * 8).to_be_bytes());
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in data.as_chunks::<64>().0.iter() {
        let mut w = [0u32; 64];
        for (i, b) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*b);
        }
        for i in 16..64 {
            let a = w[i - 15];
            let b = w[i - 2];
            w[i] = w[i - 16]
                .wrapping_add(a.rotate_right(7) ^ a.rotate_right(18) ^ (a >> 3))
                .wrapping_add(w[i - 7])
                .wrapping_add(b.rotate_right(17) ^ b.rotate_right(19) ^ (b >> 10));
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut z] = h;
        for i in 0..64 {
            let t = z
                .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
                .wrapping_add((e & f) ^ (!e & g))
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let u = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22))
                .wrapping_add((a & b) ^ (a & c) ^ (b & c));
            z = g;
            g = f;
            f = e;
            e = d.wrapping_add(t);
            d = c;
            c = b;
            b = a;
            a = t.wrapping_add(u);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, z]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0; 32];
    for (d, v) in out.as_chunks_mut::<4>().0.iter_mut().zip(h) {
        d.copy_from_slice(&v.to_be_bytes());
    }
    out
}

const HASHES: [[u8; 32]; 9] = [
    [
        49, 128, 105, 241, 114, 134, 33, 217, 138, 68, 9, 181, 168, 128, 158, 164, 16, 215, 180,
        173, 201, 77, 107, 164, 33, 214, 155, 20, 201, 138, 153, 219,
    ],
    [
        185, 1, 46, 11, 198, 17, 158, 31, 71, 69, 255, 110, 37, 191, 139, 169, 18, 66, 62, 156, 54,
        69, 19, 17, 138, 49, 181, 53, 92, 135, 190, 172,
    ],
    [
        247, 238, 76, 224, 236, 149, 122, 246, 9, 240, 174, 154, 184, 69, 172, 27, 192, 140, 43,
        89, 140, 116, 155, 156, 59, 120, 236, 55, 244, 83, 36, 216,
    ],
    [
        74, 29, 183, 210, 169, 47, 66, 25, 112, 211, 95, 138, 100, 2, 235, 100, 137, 150, 146, 87,
        197, 77, 7, 181, 89, 1, 134, 240, 17, 50, 126, 29,
    ],
    [
        175, 79, 148, 234, 153, 106, 101, 211, 101, 146, 187, 108, 239, 196, 7, 220, 98, 139, 61,
        72, 140, 156, 51, 91, 143, 96, 21, 187, 252, 182, 163, 174,
    ],
    [
        2, 130, 250, 75, 197, 36, 31, 182, 144, 90, 13, 170, 12, 0, 244, 62, 99, 254, 99, 192, 240,
        15, 246, 190, 218, 1, 211, 96, 33, 65, 253, 29,
    ],
    [
        218, 106, 7, 201, 58, 106, 2, 124, 216, 28, 229, 14, 77, 252, 117, 104, 72, 34, 59, 217,
        81, 244, 166, 14, 112, 186, 177, 233, 188, 60, 32, 167,
    ],
    [
        82, 246, 213, 20, 182, 190, 210, 255, 62, 98, 194, 115, 171, 246, 204, 215, 144, 130, 229,
        32, 93, 101, 127, 199, 209, 120, 127, 52, 126, 227, 181, 182,
    ],
    [
        227, 174, 11, 19, 28, 216, 221, 101, 25, 47, 181, 59, 131, 252, 172, 11, 12, 225, 148, 204,
        135, 181, 168, 148, 227, 1, 153, 109, 0, 204, 105, 56,
    ],
];

#[cfg(test)]
mod tests {
    use super::sha256;
    #[test]
    fn fips_sha256_vectors() {
        assert_eq!(
            sha256(b""),
            [
                227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39,
                174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85
            ]
        );
        assert_eq!(
            sha256(b"abc"),
            [
                186, 120, 22, 191, 143, 1, 207, 234, 65, 65, 64, 222, 93, 174, 34, 35, 176, 3, 97,
                163, 150, 23, 122, 156, 180, 16, 255, 97, 242, 0, 21, 173
            ]
        );
    }
}
