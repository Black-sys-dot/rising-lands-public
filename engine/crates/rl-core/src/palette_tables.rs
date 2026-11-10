//! ProgInit palette remap tables, intact 0x474f04..0x474f2e: CalculEclairage
//! 0x45fafc (DL percent, table 0x4e24b8), CalculNoirBlanc 0x45fca5 (0x4e24bc),
//! CalculNegatif 0x45fe31 (0x4e24c0) and CalculGris 0x45fddf (0x4e24c4).
//! The three searching routines share the nearest-colour selection
//! 0x45fbf9..0x45fca3: squared component distance 0x4795af against every
//! palette entry, candidates 1..255 strictly closer than the current best,
//! skipping indices inside the colour-cycle ranges counted at 0x4e24e1.
//! ProgInit computes these before its SetCyclage calls, so the count it sees
//! is whatever memory holds; the exclusion is evaluated, never assumed.
use crate::{MemoriaJogo, Result};

/// Colour-cycle ranges as the selection reads them: `(first, last)` per range.
#[derive(Clone, Copy, Default)]
pub struct Cycles {
    pub count: u8,
    pub ranges: [(u8, u8); 3],
}

impl Cycles {
    pub fn from_memory(m: &MemoriaJogo) -> Result<Self> {
        let count = m.u8(0x4e24e1)?;
        let mut ranges = [(0u8, 0u8); 3];
        for (i, range) in ranges.iter_mut().enumerate() {
            let base = 0x4e24e4 + 3 * i as u32;
            *range = (m.u8(base)?, m.u8(base + 1)?);
        }
        Ok(Self { count, ranges })
    }
    fn excludes(&self, index: u8) -> bool {
        // Native 0x45fc36..0x45fc5d reads 3*count bytes; counts above three
        // would read past the three SetCyclage records into the next bytes.
        (0..usize::from(self.count)).any(|i| match self.ranges.get(i) {
            Some(&(first, last)) => first <= index && index <= last,
            None => false,
        })
    }
}

/// Native 0x4795af: sum of squared unsigned component differences.
fn distance(a: &[u8], b: [u8; 3]) -> u32 {
    (0..3)
        .map(|k| {
            let d = u32::from(a[k]).abs_diff(u32::from(b[k]));
            d * d
        })
        .sum()
}

/// Native 0x45fbf9..0x45fca3 selection against the 768-byte palette.
fn nearest(palette: &[u8], target: [u8; 3], cycles: &Cycles) -> u8 {
    let dist: Vec<u32> = (0..256)
        .map(|c| distance(&palette[3 * c..3 * c + 3], target))
        .collect();
    let mut best = 0usize;
    for candidate in 1..256 {
        if dist[candidate] < dist[best] && !cycles.excludes(candidate as u8) {
            best = candidate;
        }
    }
    best as u8
}

/// CalculEclairage 0x45fafc: components scaled by 100/percent; zero becomes
/// 30 and values above 255 saturate. A zero percent divides by zero natively.
pub fn eclairage(palette: &[u8], percent: u8, cycles: &Cycles) -> Option<[u8; 256]> {
    if percent == 0 {
        return None;
    }
    let mut table = [0u8; 256];
    for (i, out) in table.iter_mut().enumerate() {
        let mut target = [0u8; 3];
        for k in 0..3 {
            let v = u32::from(palette[3 * i + k]) * 100 / u32::from(percent);
            target[k] = if v == 0 {
                30
            } else if v >= 256 {
                255
            } else {
                v as u8
            };
        }
        *out = nearest(palette, target, cycles);
    }
    Some(table)
}

/// CalculNoirBlanc 0x45fca5: the grey target is the smallest component.
pub fn noir_blanc(palette: &[u8], cycles: &Cycles) -> [u8; 256] {
    let mut table = [0u8; 256];
    for (i, out) in table.iter_mut().enumerate() {
        let p = &palette[3 * i..3 * i + 3];
        let grey = p[0].min(p[1]).min(p[2]);
        *out = nearest(palette, [grey; 3], cycles);
    }
    table
}

/// CalculNegatif 0x45fe31: each component is the low byte of 63 - value.
pub fn negatif(palette: &[u8], cycles: &Cycles) -> [u8; 256] {
    let mut table = [0u8; 256];
    for (i, out) in table.iter_mut().enumerate() {
        let p = &palette[3 * i..3 * i + 3];
        let target = [
            0x3fu8.wrapping_sub(p[0]),
            0x3fu8.wrapping_sub(p[1]),
            0x3fu8.wrapping_sub(p[2]),
        ];
        *out = nearest(palette, target, cycles);
    }
    table
}

/// CalculGris 0x45fddf: 47 - (mean of the three components >> 3), no search.
pub fn gris(palette: &[u8]) -> [u8; 256] {
    let mut table = [0u8; 256];
    for (i, out) in table.iter_mut().enumerate() {
        let p = &palette[3 * i..3 * i + 3];
        let mean = (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3;
        *out = (0x30u32.wrapping_sub(mean >> 3).wrapping_sub(1)) as u8;
    }
    table
}

/// ProgInit 0x474f04..0x474f2e: the four tables from the palette at
/// [0x4e24b0] into the pointers [0x4e24b8], [0x4e24bc], [0x4e24c0], [0x4e24c4].
pub fn prog_init_tables(m: &mut MemoriaJogo, percent: u8) -> Result<()> {
    let palette = m.read(m.u32(0x4e24b0)?, 768)?.to_vec();
    let cycles = Cycles::from_memory(m)?;
    let lighting = eclairage(&palette, percent, &cycles)
        .ok_or(crate::EngineError::IncompleteField { address: 0x45fb46 })?;
    m.write(m.u32(0x4e24b8)?, &lighting)?;
    m.write(m.u32(0x4e24bc)?, &noir_blanc(&palette, &cycles))?;
    m.write(m.u32(0x4e24c0)?, &negatif(&palette, &cycles))?;
    m.write(m.u32(0x4e24c4)?, &gris(&palette))?;
    Ok(())
}
