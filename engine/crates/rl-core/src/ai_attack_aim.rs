//! CalculAttaqueEngin mode3: intact aim4519bd and ordered firing-animation prefix.
//! The movement provider is reused unchanged. Damage/projectiles remain explicit.
use crate::{frame::stop, player::unit, EngineError, FrameRegisters, MemoriaJogo, Result};

fn target(m: &mut MemoriaJogo, r: u32) -> Result<u16> {
    let building = m.u16(r + 0xb8)?;
    if m.u8(r + 0xb0)? <= 1 {
        let cell = crate::ai_building_attack::adjacent_cell(m, building, m.u16(r + 0x7c)?)?;
        if cell != 65535 {
            return Ok(cell);
        }
    }
    Ok(crate::frame_core::building_cell(m, u32::from(building))? as u16)
}

fn double(m: &MemoriaJogo, p: u32) -> Result<f64> {
    Ok(f64::from_bits(
        u64::from(m.u32(p)?) | (u64::from(m.u32(p + 4)?) << 32),
    ))
}

/// AngleCalcul4796cb with the original constants and final truncation to a word.
/// The attack caller supplies sectors8/32 and byte coordinates from cell words.
pub(crate) fn angle(m: &MemoriaJogo, from: u16, to: u16, sectors: u8) -> Result<u8> {
    let (x, y, tx, ty) = (from & 255, from >> 8, to & 255, to >> 8);
    let dx = f64::from(x.abs_diff(tx));
    let dy = f64::from(y.abs_diff(ty));
    let full = double(m, 0x494e0c)?;
    let half = double(m, 0x494e14)?;
    let step = full / f64::from(sectors);
    let mut degrees = if dy == 0.0 {
        0.0
    } else if dx == 0.0 {
        90.0
    } else {
        (dx / (dx * dx + dy * dy).sqrt()).acos() * half / f64::from_bits(0x400921fb54411744)
    };
    if x > tx {
        degrees = if y >= ty {
            degrees + half
        } else {
            half - degrees
        };
    } else if y > ty {
        degrees = full - degrees;
    }
    degrees += step * double(m, 0x494e1c)?;
    if degrees >= full {
        degrees += double(m, 0x494e24)?;
    }
    let sector = (degrees / step) as u16;
    Ok(
        ((u32::from(sector) % u32::from(sectors) + u32::from(sectors) - u32::from(sectors / 2))
            % u32::from(sectors)
            + 1) as u8,
    )
}

/// RotateEngin448599/RotateTourEngin4484a7 byte arithmetic, including zero and
/// wrapped counters. These indispensable aim leaves do not clamp directions.
fn rotate(m: &mut MemoriaJogo, r: u32, turret: bool) -> Result<()> {
    let (desired, current, counter, count) = if turret {
        (0xda, 0x94, 0x96, 32)
    } else {
        (0x90, 0x91, 0x92, 16)
    };
    if turret && !matches!(m.u8(r + 1)?, 12 | 15) {
        return Ok(());
    }
    if !turret && matches!(m.u8(r + 1)?, 12 | 15) {
        m.set_u8(
            r + 0xda,
            m.u8(r + desired)?
                .wrapping_sub(1)
                .wrapping_mul(2)
                .wrapping_add(1),
        )?;
    }
    let d = m.u8(r + desired)?;
    let c = m.u8(r + current)?;
    if d != c {
        let next = m.u8(r + counter)?.wrapping_add(1);
        m.set_u8(r + counter, next)?;
        if next == m.u8(r + 0x93)? {
            let increase = (d > c && d - c < count / 2) || (c > d && c - d > count / 2);
            let orientation = if increase {
                c % count + 1
            } else if c == 1 {
                count
            } else {
                c.wrapping_sub(1)
            };
            m.set_u8(r + current, orientation)?;
            m.set_u8(r + counter, 0)?;
        }
    }
    Ok(())
}

/// Complete building-target aim block. Called precisely after the predecessor's
/// real in-range and clear-line checks; no prior movement effect is repeated.
pub fn aim(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    m.set_u16(r + 0x7e, m.u16(r + 0x7c)?)?;
    crate::frame_spawn::release(m, u32::from(u))?;
    m.set_u8(r + 0x88, 0)?;
    let cell = target(m, r)?;
    let kind = m.u8(r + 1)?;
    let turret = matches!(kind, 12 | 15);
    let direction = if turret {
        // Native turret path resolves the building centre independently, after
        // the ordinary adjacency lookup; both calls and their order matter.
        let centre = crate::frame_core::building_cell(m, u32::from(m.u16(r + 0xb8)?))? as u16;
        angle(m, m.u16(r + 8)?, centre, 32)?
    } else {
        let mut d = angle(m, m.u16(r + 8)?, cell, 8)?
            .wrapping_mul(2)
            .wrapping_sub(1);
        if kind == 3 {
            // ADD EDX,11 occurs after zero-extension, not wrapping in a byte.
            d = ((u32::from(d) + 11) % 16 + 1) as u8;
        }
        d
    };
    let (desired, current) = if turret { (0xda, 0x94) } else { (0x90, 0x91) };
    m.set_u8(r + desired, direction)?;
    if m.u8(r + current)? != direction {
        rotate(m, r, turret)?;
        return Ok(());
    }
    let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(r)?), u32::from(kind), 3, 0)? & 65535;
    let reload = i32::from(m.u16(0x4965de + 50 * u32::from(kind))?).wrapping_sub(bonus as i32);
    if i32::from(m.u16(r + 0x102)?) < reload {
        return Ok(());
    }
    m.set_u16(r + 0x102, 0)?;
    m.set_u8(r + 0x8a, 1)?;
    if kind != 16 {
        m.set_u8(r + 0xa4, 0)?;
        m.set_u8(r + 0xa9, 0)?;
    }
    if matches!(kind, 14 | 16) && m.u8(r + 0xda)? == 0 {
        crate::frame_spawn::call(
            m,
            "SetDeplaceEngin",
            &[u32::from(u), u32::from(m.u16(r + 0x7c)?)],
        )?;
    }
    Ok(())
}

/// Only the next ordered firing-animation leaf/gates. A shot remains a named
/// stop BEFORE target resolution, sound, projectile, collision or damage.
fn firing_prefix(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    let kind = m.u8(r + 1)?;
    if matches!(kind, 0..=4 | 8 | 11 | 14 | 15 | 18..=25) {
        let count = m.u8(r + 0xa9)?.wrapping_add(1);
        m.set_u8(r + 0xa9, count)?;
        if count == m.u8(r + 0xa8)? {
            let frames = u32::from(m.u8(r + 0xa6)?);
            if frames == 0 {
                return stop("AnimAttaqueEngin native division", 0x447a4d);
            }
            let next = (u32::from(m.u8(r + 0xa4)?) + 1) % frames;
            m.set_u8(r + 0xa9, 0)?;
            m.set_u8(r + 0xa4, next as u8)?;
        }
    }
    if m.u8(r + 0xa9)? == 0 && m.u8(r + 0xa4)? == m.u8(r + 0x7a)? {
        return stop("CalculAttaqueEngin building shot target", 0x452503);
    }
    if m.u8(r + 0xa9)? == 0 && m.u8(r + 0xa4)? == 0 {
        let remaining = m.u8(r + 0x8b)?;
        if remaining == 0 {
            m.set_u8(r + 0x8a, 0)?;
            if kind != 16 {
                m.set_u8(r + 0xa4, 0)?;
            }
            m.set_u16(r + 0x102, 0)?;
        } else {
            m.set_u8(r + 0x8b, remaining.wrapping_sub(1))?;
            if kind == 3 {
                m.set_u8(r + 0xa4, 5)?;
            }
            if kind == 11 {
                m.set_u8(r + 0xa4, 0)?;
            }
        }
    }
    Ok(())
}

pub fn approach(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    if m.u8(r + 0xb7)? == 3 && m.u8(r + 0x8a)? == 1 {
        return firing_prefix(m, r);
    }
    match crate::ai_building_attack::approach(m, u) {
        Err(EngineError::UnportedCallee {
            name: "CalculAttaqueEngin building aim",
            address: 0x4519bd,
        }) => aim(m, u),
        result => result,
    }
}
pub fn state1(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    if crate::chain_attack::validate_target(m, u, r)? && m.u8(r + 2)? == 1 {
        approach(m, u)?;
    }
    Ok(())
}
/// Explicit void effects transport, paired with a native PUSH EAX/CALL/POP shim.
pub fn entry(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    approach(m, r.eax as u16)?;
    Ok(FrameRegisters {
        esp: r.esp.wrapping_add(4),
        ..r
    })
}
