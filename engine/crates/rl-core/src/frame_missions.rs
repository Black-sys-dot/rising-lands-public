//! TestFinDeMission and influence queries over explicit live state.
use crate::{frame::stop, frame_core, frame_economy, MemoriaJogo, Result};
fn zone(m: &MemoriaJogo, cell: u16) -> Result<u16> {
    let x = cell & 255;
    let y = cell >> 8;
    for z in 0..400 {
        if z == 255 {
            continue;
        }
        let r = 0x50c614 + 10 * z;
        if m.u16(r)? <= x && x <= m.u16(r + 4)? && m.u16(r + 2)? <= y && y <= m.u16(r + 6)? {
            return Ok(z as u16);
        }
    }
    Ok(255)
}
fn zone_matches(m: &MemoriaJogo, r: u32, z: u16) -> Result<bool> {
    for o in [4, 6, 8] {
        let v = m.u16(r + o)?;
        if v == 255 || v == z {
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn test(m: &mut MemoriaJogo, player: u8) -> Result<u32> {
    let mut result = 1u8;
    let mut last = None;
    for i in 0..10 {
        let r = 0x4e1d20 + 10 * i;
        let mark = 0x4e1d88 + i;
        match m.u8(r)? {
            t @ 0..=2 => {
                let mut args = [u32::from(player), 0, 0, 0];
                args[usize::from(t) + 1] = u32::from(m.u16(r + 2)?);
                if frame_economy::call(m, "TestRessource", &args)? != 0 {
                    if m.u8(mark)? == 0 {
                        m.set_u8(mark, 1)?;
                    }
                } else {
                    result = 0;
                    if m.u8(mark)? != 0 {
                        m.set_u8(mark, 0)?;
                    }
                }
            }
            3 => {
                let p = u32::from(m.u16(r + 2)?);
                if frame_economy::call(m, "RechercheGetNbInvention", &[u32::from(player)])? as u8
                    as u32
                    >= p
                {
                    if m.u8(mark)? == 0 {
                        m.set_u8(mark, 1)?;
                    }
                } else {
                    result = 0;
                    if m.u8(mark)? != 0 {
                        m.set_u8(mark, 0)?;
                    }
                }
            }
            4 => {
                for j in 0..1000 {
                    let b = 0x49a0c8 + 276 * j;
                    if m.u8(b + 5)? == 0
                        || m.u8(b + 4)? != player
                        || u16::from(m.u8(b + 8)?) != m.u16(r + 2)?
                    {
                        continue;
                    }
                    let c = frame_core::call(m, "BatimentGetCase", &[j])? as u16;
                    let z = zone(m, c)?;
                    if zone_matches(m, r, z)? {
                        last = Some(i);
                        if m.u8(mark)? == 0 {
                            m.set_u8(mark, 1)?;
                        }
                    } else if m.u8(mark)? != 0 {
                        m.set_u8(mark, 0)?;
                    }
                }
                if m.u8(mark)? == 0 {
                    result = 0;
                }
            }
            5 => {
                m.set_u8(mark, 1)?;
                for c in 0..4 {
                    if c == u32::from(player) {
                        continue;
                    }
                    if frame_economy::call(m, "PlayerEstAllie", &[u32::from(player), c])? as u8 != 0
                    {
                        continue;
                    }
                    if m.u16(0x4e1928 + 2 * c)? != 0 || m.u32(0x4dfaa8 + 4 * c)? != 0 {
                        m.set_u8(mark, 0)?;
                    }
                }
                result &= m.u8(mark)?;
            }
            255 if i == 0 => {
                if m.u8(mark)? != 0 {
                    m.set_u8(mark, 0)?;
                }
                result = 0;
            }
            _ => {} // Original switch has no action for other condition bytes.
        }
    }
    let mission = m.u8(0x496fe4)?;
    if mission == 6 {
        let r = 0x50c614;
        let corner = u32::from(m.u16(r)?).wrapping_add(u32::from(m.u16(r + 2)?) << 8) as u16;
        result = frame_core::call(
            m,
            "CaseEstIlluminee",
            &[u32::from(corner), u32::from(player)],
        )? as u8;
    }
    if result != 0 {
        m.set_u8(0x4e1d84, 1)?;
    }
    if m.u16(0x4e1a3a)? == 0 {
        result = 1;
        m.set_u8(0x4e1d84, 0)?;
    }
    if result == 0 || m.u8(0x4e1d84)? == 0 || ![8, 13].contains(&mission) {
        return Ok(u32::from(result));
    }
    let Some(i) = last else {
        return stop("TestFinDeMission uninitialized record", 0x4589dc);
    };
    let r = 0x4e1d20 + 10 * i;
    let mut owners = [false; 4];
    for j in 0..1000 {
        let b = 0x49a0c8 + 276 * j;
        if m.u8(b + 5)? == 0 {
            continue;
        }
        let owner = m.u8(b + 4)?;
        if owner == player || owner == 200 || u16::from(m.u8(b + 8)?) != m.u16(r + 2)? {
            continue;
        }
        let c = frame_core::call(m, "BatimentGetCase", &[j])? as u16;
        if !zone_matches(m, r, zone(m, c)?)? {
            continue;
        }
        if owner >= 4 {
            return stop("TestFinDeMission owner array", 0x458aa8);
        }
        owners[usize::from(owner)] = true;
    }
    m.set_u8(0x4e1d85, owners.iter().filter(|x| **x).count() as u8)?;
    Ok(u32::from(result))
}
