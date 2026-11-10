//! Original menu pictures: LoadPie0x4799c4 RLE and LZSS0x47947f.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};
pub const RING: u32 = 0x4973ee;

/// The native ring keeps its last18 bytes on entry. Callers provide its complete
/// original memory; only the first4078 bytes are initialized to spaces.
pub fn lzss(m: &mut MemoriaJogo, source: u32, destination: u32, length: u32) -> Result<u32> {
    m.write(RING, &[32; 4078])?;
    let (mut input, mut output, mut cursor, mut flags) = (0u32, 0u32, 4078u32, 0u32);
    loop {
        flags >>= 1;
        if flags & 0x100 == 0 {
            if input == length {
                break;
            }
            flags = 0xff00 | u32::from(m.u8(source.wrapping_add(input))?);
            input += 1;
        }
        if input == length {
            break;
        }
        if flags & 1 != 0 {
            let byte = m.u8(source.wrapping_add(input))?;
            input += 1;
            m.set_u8(RING + cursor, byte)?;
            m.set_u8(destination.wrapping_add(output), byte)?;
            cursor = (cursor + 1) & 4095;
            output = output.wrapping_add(1);
        } else {
            if length - input < 2 {
                return stop("LZSS truncated native match", 0x4794ff);
            }
            let token = m.u16(source.wrapping_add(input))?;
            input += 2;
            let mut at = u32::from(token & 255) | (u32::from((token >> 12) & 15) * 256);
            for _ in 0..u32::from((token >> 8) & 15) + 3 {
                let byte = m.u8(RING + at)?;
                m.set_u8(RING + cursor, byte)?;
                m.set_u8(destination.wrapping_add(output), byte)?;
                at = (at + 1) & 4095;
                cursor = (cursor + 1) & 4095;
                output = output.wrapping_add(1);
            }
        }
    }
    Ok(output)
}
pub fn lzss_registers(
    m: &mut MemoriaJogo,
    mut r: FrameRegisters,
    source: u32,
    destination: u32,
    length: u32,
) -> Result<FrameRegisters> {
    r.eax = lzss(m, source, destination, length)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Original PIE RLE, full rows at the caller's stride. The shell uses800x600.
/// Palette data is skipped by the original LoadPie; it does not replace it.
pub fn pie(file: &[u8], surface: &mut [u8], stride: usize) -> Result<u32> {
    if file.get(..3) != Some(b"Pie") {
        return stop("LoadPie invalid signature", 0x479a44);
    }
    let word = |p: usize| -> Result<usize> {
        let b = file
            .get(p..p + 2)
            .ok_or(crate::EngineError::UndeclaredAccess {
                address: p as u32,
                size: 2,
            })?;
        Ok(usize::from(u16::from_le_bytes(b.try_into().unwrap())))
    };
    let width = word(3)?;
    let height = word(5)?;
    if width != stride || stride > 1024 || height.checked_mul(stride) != Some(surface.len()) {
        return stop("LoadPie geometry outside complete native rows", 0x479adb);
    }
    let (mut input, mut x, mut row, mut last_edx) = (777usize, 0usize, 0usize, 0u32);
    let mut scanline = vec![0; stride];
    while row < height {
        let token = *file
            .get(input)
            .ok_or(crate::EngineError::UndeclaredAccess {
                address: input as u32,
                size: 1,
            })?;
        input += 1;
        let count = if token >= 128 {
            usize::from(1u8.wrapping_sub(token))
        } else {
            usize::from(token) + 1
        };
        if x + count > stride {
            return stop("LoadPie token crosses native scanline", 0x479b7f);
        }
        if token >= 128 {
            let value = *file
                .get(input)
                .ok_or(crate::EngineError::UndeclaredAccess {
                    address: input as u32,
                    size: 1,
                })?;
            input += 1;
            scanline[x..x + count].fill(value);
            last_edx = u32::from(value) * 0x01010101;
        } else {
            let bytes =
                file.get(input..input + count)
                    .ok_or(crate::EngineError::UndeclaredAccess {
                        address: input as u32,
                        size: count,
                    })?;
            scanline[x..x + count].copy_from_slice(bytes);
            input += count;
            last_edx = count as u32;
        }
        x += count;
        if x == stride {
            surface[row * stride..(row + 1) * stride].copy_from_slice(&scanline);
            row += 1;
            x = 0;
        }
    }
    Ok(last_edx)
}

/// A shell background file of the CD (`TITRE.PIE`, `PAGEMIS.PIE`, `PAGEMIS2.PIE`,
/// `CREDITS.PIE` or `ECRANFIN.LZS`) decoded to 800x600 indexed pixels, as the
/// prepared-shell consumer shows it. `.LZS`: the LZSS envelope (`LZSS`, `FORM`
/// at 780, size at 784, payload at 788) through `lzss`; the original ring's
/// retained 18-byte tail does not change these pictures (checked with the original,
/// all-zero and all-0xff tails), so a zero ring stands in. `.PIE`: `pie` at stride 800.
pub fn decode_shell_file(file: &[u8]) -> Result<Vec<u8>> {
    let mut pixels = vec![0u8; 480000];
    if file.get(..4) == Some(b"LZSS") {
        let invalid = crate::EngineError::UndeclaredAccess {
            address: 784,
            size: 4,
        };
        if file.get(780..784) != Some(b"FORM") {
            return stop("LZSS shell envelope", 0x45a659);
        }
        let size = u32::from_le_bytes(file.get(784..788).ok_or(invalid)?.try_into().unwrap());
        let payload = file
            .get(788..788usize.saturating_add(size as usize))
            .ok_or(crate::EngineError::UndeclaredAccess {
                address: 788,
                size: size as usize,
            })?
            .to_vec();
        let mut m = MemoriaJogo::from_blocks([
            (0x1000000, payload),
            (RING, vec![0u8; 4096]),
            (0x4000000, pixels),
        ])?;
        lzss(&mut m, 0x1000000, 0x4000000, size)?;
        return Ok(m.read(0x4000000, 480000)?.to_vec());
    }
    pie(file, &mut pixels, 800)?;
    Ok(pixels)
}
