//! Startup gameplay tables, transcribed from the intact executable's Watcom loaders.
//! Call these over the original initialized PE data, before mission loading.
use crate::mission_ini::MissionIni;
use crate::{MemoriaJogo, Result};

pub(crate) fn string(m: &MemoriaJogo, address: u32) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for i in 0..256 {
        let c = m.u8(address + i)?;
        if c == 0 {
            return Ok(out);
        }
        out.push(c);
    }
    Err(crate::EngineError::IncompleteField { address })
}
fn profile(m: &MemoriaJogo, ini: &MissionIni, section: u32, key: u32, default: u32) -> Result<u32> {
    Ok(ini.integer(&string(m, section)?, &string(m, key)?, default))
}
fn name(m: &mut MemoriaJogo, ini: &MissionIni, address: u32) -> Result<()> {
    let old = string(m, address)?;
    let mut value = ini.string(&old, b"NAME", &old, 30);
    value.push(0);
    m.write(address, &value)
}
fn damage(
    m: &mut MemoriaJogo,
    ini: &MissionIni,
    section: &[u8],
    dest: u32,
    key: &[u8],
    default: &[u8],
) -> Result<()> {
    let text = ini.string(section, key, default, 250);
    let mut parts = text.split(|c| *c == b',');
    for j in 0..27 {
        let Some(part) = parts.next() else {
            return Err(crate::EngineError::UnportedCallee {
                name: "startup truncated DEGAT list",
                address: 0x48c41c,
            });
        };
        let part = part
            .iter()
            .copied()
            .skip_while(u8::is_ascii_whitespace)
            .collect::<Vec<_>>();
        let text = std::str::from_utf8(&part)
            .map_err(|_| crate::EngineError::IncompleteField { address: dest + j })?;
        let value = text
            .trim()
            .parse::<i32>()
            .map_err(|_| crate::EngineError::IncompleteField { address: dest + j })?;
        m.set_u8(dest + j, value as u8)?;
    }
    Ok(())
}
/// EnginLoadCaract 0x44188b, including all 27 EnginLoadDegat rows.
pub fn unit_types(m: &mut MemoriaJogo, ini: &MissionIni) -> Result<()> {
    m.set_u16(0x4965ac, profile(m, ini, 0x494819, 0x49480a, 2)? as u16)?;
    let ptrs = m.u32(0x4e199c)?;
    damage(m, ini, b"MINE", m.u32(ptrs + 26 * 4)?, b"DEGAT", b"")?;
    for i in 0..26 {
        let r = 0x4965ae + i * 50;
        let section = string(m, r + 12)?;
        damage(m, ini, &section, m.u32(ptrs + 4 * i)?, b"DEGAT", b"")?;
        for (key, default, offset) in [
            (b"BLINDAGE".as_slice(), 5, 11),
            (b"RAYON", 5, 42),
            (b"PORTEE", 1, 44),
            (b"FATIGUE", 3000, 46),
            (b"RECHARGE", 100, 48),
            (b"TMPSRECRUTE", 120, 0),
            (b"COUTPATATE", 1, 2),
            (b"COUTROCHE", 1, 6),
            (b"COUTPIECE", 1, 4),
        ] {
            let value = ini.integer(&section, key, default);
            if offset == 11 {
                m.set_u8(r + offset, value as u8)?;
            } else {
                m.set_u16(
                    r + offset,
                    if offset == 44 {
                        value.wrapping_mul(value)
                    } else {
                        value
                    } as u16,
                )?;
            }
        }
        name(m, ini, r + 12)?;
    }
    Ok(())
}
/// BatimentLoadCaract 0x418c70; its two damage rows are global 27-byte arrays.
pub fn structure_types(m: &mut MemoriaJogo, ini: &MissionIni) -> Result<()> {
    for (i, dest) in [(11, 0x4dfb64), (10, 0x4dfb80)] {
        damage(
            m,
            ini,
            &string(m, 0x495d8c + i * 52 + 19)?,
            dest,
            b"DEGAT",
            b"",
        )?;
    }
    for i in 0..22 {
        let r = 0x495d8c + i * 52;
        let section = string(m, r + 19)?;
        for (key, default, offset) in [
            (b"TMPSCONSTRUCTION".as_slice(), 300, 4),
            (b"COUTPATATE", 5, 12),
            (b"COUTROCHE", 5, 14),
        ] {
            m.set_u16(r + offset, ini.integer(&section, key, default) as u16)?;
        }
        name(m, ini, r + 19)?;
    }
    // Original section/key pointers, defaults and destinations in call/store order.
    for (section, key, default, dest) in [
        (0x4942a3, 0x494295, 512, 0x4973ea),
        (0x4942b9, 0x4942aa, 512, 0x495bf4),
        (0x4942ce, 0x4942c0, 512, 0x495bf6),
        (0x4942e4, 0x4942d5, 512, 0x495bf0),
        (0x4942f5, 0x4942eb, 512, 0x495bf2),
        (0x49430d, 0x4942fc, 100, 0x496f90),
        (0x494325, 0x494314, 100, 0x496f92),
        (0x49433f, 0x49432c, 30, 0x495bfa),
        (0x49435d, 0x494346, 30, 0x495bfc),
        (0x494377, 0x494364, 7, 0x495c00),
        (0x494394, 0x49437e, 7, 0x495bfe),
    ] {
        m.set_u16(dest, profile(m, ini, section, key, default)? as u16)?;
    }
    Ok(())
}
/// SortLoadCaract 0x474785. Defaults come from the original data table.
pub fn spell_types(m: &mut MemoriaJogo, ini: &MissionIni) -> Result<()> {
    for (s, k, d, a) in [
        (0x494cd6, 0x494cc4, 50, 0x49716c),
        (0x494cef, 0x494cdd, 90, 0x49716e),
    ] {
        m.set_u16(a, profile(m, ini, s, k, d)? as u16)?;
    }
    for i in 0..5 {
        let r = 0x497170 + i * 46;
        let section = string(m, r + 16)?;
        for (key, offset, byte_default, square) in [
            (0x494cf6, 0, false, false),
            (0x494cfc, 2, false, false),
            (0x494d02, 14, false, true),
            (0x494d09, 4, false, false),
            (0x494d17, 12, true, false),
            (0x494d20, 8, false, false),
            (0x494d2b, 6, false, false),
            (0x494d35, 10, false, false),
        ] {
            let default = if byte_default {
                u32::from(m.u8(r + offset)?)
            } else {
                u32::from(m.u16(r + offset)?)
            };
            let value = ini.integer(&section, &string(m, key)?, default);
            m.set_u16(
                r + offset,
                if square {
                    value.wrapping_mul(value)
                } else {
                    value
                } as u16,
            )?;
        }
        name(m, ini, r + 16)?;
    }
    Ok(())
}
/// IaAlloc's deterministic table initializer portion, after the binary rules load succeeds.
/// iaRandom(0) takes the explicit clock value and consumes no gameplay Random draw.
pub fn ai_tables(m: &mut MemoriaJogo, clock_ms: u32) -> Result<()> {
    m.set_u32(0x498db4, clock_ms % 0x7fffffff)?;
    for (a, v) in [
        (0x498c38, 100),
        (0x498c3c, 800),
        (0x498c40, 100),
        (0x498c44, 1000),
        (0x498c4c, 20),
        (0x498c50, 5),
        (0x498c54, 1),
        (0x498c58, 1),
        (0x498c60, 0),
        (0x498c70, 30),
        (0x498c80, 30),
        (0x498c90, 20),
        (0x51bcdc, 100),
        (0x51bce0, 100),
        (0x51bce4, 100),
        (0x51bce8, 5),
        (0x51bcec, 80),
        (0x51bcf0, 5),
        (0x51bcf4, 3),
        (0x51bcf8, 2),
        (0x498c10, 2000),
        (0x498c14, 2500),
        (0x498c18, 1000),
        (0x498c1c, 1000),
        (0x498cb0, 10500),
        (0x498cb4, 3500),
        (0x498cb8, 1500),
        (0x498cbc, 1000),
        (0x498cc4, 5),
        (0x498cc8, 3),
        (0x498ccc, 15),
        (0x498cd0, 10),
        (0x521350, 10),
        (0x521354, 10),
        (0x521358, 40),
        (0x52135c, 10),
        (0x521360, 10),
        (0x521364, 6000),
    ] {
        m.set_u32(a, v)?;
    }
    Ok(())
}

/// InventionLoadCaract 0x470560: all four research branches, 30 records each.
pub fn invention_types(m: &mut MemoriaJogo, ini: &MissionIni) -> Result<()> {
    for (branch, prefix_address, digits) in [
        (0, 0x494bbc, 11),
        (1, 0x494bfb, 9),
        (2, 0x494c38, 10),
        (3, 0x494c76, 9),
    ] {
        let mut prefix = string(m, prefix_address)?;
        prefix.truncate(digits);
        for item in 0..30 {
            let mut section = prefix.clone();
            section.extend(item.to_string().bytes());
            let a = 0x50817c + 360 * branch + 12 * item;
            for (key, default, offset) in [
                (b"COUT".as_slice(), 65000, 0),
                (b"MISSION".as_slice(), 100, 5),
                (b"EFFET".as_slice(), 255, 6),
                (b"PARAM1".as_slice(), if branch == 3 { 2 } else { 1 }, 7),
                (b"PARAM2".as_slice(), 0, 8),
                (b"PARAM3".as_slice(), 0, 9),
                (b"APPARITION".as_slice(), 100, 4),
            ] {
                let v = ini.integer(&section, key, default);
                if offset == 0 {
                    m.set_u32(a, v)?;
                } else {
                    m.set_u8(a + offset, v as u8)?;
                }
            }
        }
    }
    Ok(())
}
