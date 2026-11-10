//! DemonteBatiment41effd. Effects API; original scratch registers are unspecified.
//! The normal frame hook is supplied separately for the active runtime owner.
use crate::{FrameRegisters, MemoriaJogo, Result};

#[cfg(feature = "validation-trace")]
std::thread_local! {
    static ENTRIES: std::cell::RefCell<Vec<(u16, MemoriaJogo)>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(feature = "validation-trace")]
pub fn take_validation_entries() -> Vec<(u16, MemoriaJogo)> {
    ENTRIES.with(|entries| std::mem::take(&mut *entries.borrow_mut()))
}

fn record(index: u16) -> u32 {
    0x49a0c8 + 276 * u32::from(index)
}
fn bag(m: &mut MemoriaJogo, name: &str, address: u32) -> Result<u32> {
    Ok(crate::chain_bags::body(
        m,
        name,
        FrameRegisters {
            eax: address,
            edx: 1,
            ebx: 9,
            ..Default::default()
        },
    )?
    .eax)
}

/// ResetBatimentTerrainModify41b888. Mode0 restores the native patterned soil;
/// mode1 removes building collision only at the original collision footprint.
pub fn reset_terrain(m: &mut MemoriaJogo, index: u16, mode: u8) -> Result<()> {
    let r = record(index);
    let kind = u32::from(m.u8(r + 8)?);
    let width = u32::from(m.u16(0x495d8c + 52 * kind)?);
    let height = u32::from(m.u16(0x495d8e + 52 * kind)?);
    let origin = u32::from(m.u16(r + 2)?);
    let mut cell = origin;
    // ESI advances at every cell and is deliberately not reset between rows.
    let mut x = origin & 255;
    for row in 0..height {
        for col in 0..width {
            if mode == 0 {
                let tile = 0x15d + (x as u16 % 8) + 8 * ((origin / 256 + row) as u16 % 8);
                m.set_u16(
                    m.u32(0x507d70)?.wrapping_add(2 * u32::from(cell as u16)),
                    tile,
                )?;
            } else if crate::frame_core::call(
                m,
                "BatimentCollision",
                &[kind, (row * width + col) & 65535],
            )? as u8
                != 0
            {
                let p = m.u32(0x507da8)?.wrapping_add(u32::from(cell as u16));
                m.set_u8(p, m.u8(p)? & !1)?;
            }
            cell = cell.wrapping_add(1);
            x = x.wrapping_add(1);
        }
        cell = cell.wrapping_add(256).wrapping_sub(width);
    }
    Ok(())
}

/// CreerExplosion475710, only the original empty-cell object/phase stores.
fn explosion(m: &mut MemoriaJogo, cell: u16, kind: u8) -> Result<()> {
    let c = u32::from(cell);
    let p = m.u32(0x507d74)?.wrapping_add(c);
    if m.u8(p)? == 0 {
        m.set_u8(p, kind)?;
        m.set_u8(m.u32(0x507dac)?.wrapping_add(c), 0)?;
    }
    Ok(())
}

/// BatimentGenereFumee41e260: every type body of the jump table 0x41e208
/// (types 0..=21; 9, 12 and > 21 return at 0x41b78a). Each body places
/// CreerExplosion phases 0x3b..0x41 at fixed footprint offsets of the origin
/// cell (16-bit wrap, as the original AX) chosen by the hit points +0x84;
/// types 3 and 4 first clear some object bytes (32-bit offsets, unwrapped).
pub fn smoke(m: &mut MemoriaJogo, index: u16) -> Result<()> {
    let r = record(index);
    let kind = m.u8(r + 8)?;
    let c = m.u16(r + 2)?;
    let hp = u32::from(m.u16(r + 0x84)?);
    let objects = m.u32(0x507d74)?;
    let e =
        |m: &mut MemoriaJogo, offset: u16, effect: u8| explosion(m, c.wrapping_add(offset), effect);
    let z = |m: &mut MemoriaJogo, offset: u32| {
        m.set_u8(objects.wrapping_add(u32::from(c)).wrapping_add(offset), 0)
    };
    match kind {
        0 => {
            e(m, 0, 0x3d)?;
            e(m, 1, 0x3b)?;
            if hp < 0x82 {
                e(m, 0x100, 0x40)
            } else if hp >= 0xa0 {
                Ok(())
            } else {
                e(m, 0x101, 0x41)
            }
        }
        1 => {
            for (offset, effect) in [
                (0x101, 0x3d),
                (0x102, 0x3b),
                (0x203, 0x3b),
                (0x202, 0x3c),
                (0x201, 0x3b),
            ] {
                e(m, offset, effect)?;
            }
            if hp < 0x50 {
                e(m, 0x200, 0x40)
            } else if hp < 0x6e {
                e(m, 0x303, 0x41)
            } else if hp < 0x8c {
                e(m, 0x302, 0x3f)
            } else if hp >= 0xa0 {
                Ok(())
            } else {
                e(m, 0x300, 0x3f)?;
                e(m, 0x301, 0x40)
            }
        }
        2 | 6 | 8 => {
            for (offset, effect) in [(0x201, 0x3c), (0x203, 0x3d), (0x302, 0x3c), (0x203, 0x3b)] {
                e(m, offset, effect)?;
            }
            // Shared tails 0x41e534 / 0x41e54a(0x41e556) / 0x41e4af / 0x41e597.
            let (low, mid, high, top) = if kind == 8 {
                (0x32, 0x78, 0xaa, 0xbe)
            } else {
                (0x28, 0x50, 0x82, 0xa0)
            };
            if hp < low {
                e(m, 0x104, 0x3f)
            } else if hp < mid {
                e(m, if kind == 6 { 0x302 } else { 0x202 }, 0x40)?;
                e(m, 0x305, 0x3f)
            } else if hp < high {
                e(m, 0x300, 0x3f)?;
                e(m, 0x301, 0x40)
            } else if hp >= top {
                Ok(())
            } else {
                e(m, 0x303, 0x41)?;
                e(m, 0x304, 0x40)
            }
        }
        3 => {
            for offset in [0x101, 0x103, 0x102, 0x203] {
                z(m, offset)?;
            }
            for (offset, effect) in [(0x101, 0x3d), (0x103, 0x3b), (0x102, 0x3b), (0x203, 0x3c)] {
                e(m, offset, effect)?;
            }
            if hp < 0x50 {
                z(m, 0x104)?;
                e(m, 0x104, 0x3f)
            } else if hp < 0x82 {
                z(m, 0x100)?;
                e(m, 0x100, 0x41)
            } else if hp < 0xb4 {
                z(m, 0x200)?;
                z(m, 0x202)?;
                e(m, 0x200, 0x40)?;
                e(m, 0x202, 0x41)
            } else {
                Ok(())
            }
        }
        4 => {
            z(m, 0)?;
            z(m, 1)?;
            e(m, 0, 0x3b)?;
            e(m, 1, 0x3d)?;
            if hp < 0x5a {
                z(m, 0x102)?;
                e(m, 0x102, 0x3f)
            } else if hp < 0x82 {
                z(m, 0x100)?;
                e(m, 0x100, 0x40)
            } else if hp >= 0xbe {
                Ok(())
            } else {
                z(m, 0x201)?;
                e(m, 0x201, 0x41)
            }
        }
        5 => {
            for (offset, effect) in [(0x300, 0x3f), (3, 0x3c), (0x202, 0x3c), (0x102, 0x3d)] {
                e(m, offset, effect)?;
            }
            if hp < 0x32 {
                e(m, 0x203, 0x40)?;
            }
            if hp < 0x50 {
                e(m, 0x101, 0x3f)?;
            }
            if hp < 0x6e {
                e(m, 0x301, 0x40)?;
                e(m, 0x200, 0x3f)
            } else if hp < 0x8c {
                e(m, 0x302, 0x41)
            } else if hp >= 0xa0 {
                Ok(())
            } else {
                e(m, 0x304, 0x40)
            }
        }
        7 => {
            for (offset, effect) in [
                (0x103, 0x3b),
                (0x102, 0x3c),
                (0x101, 0x3c),
                (0x200, 0x3d),
                (0x201, 0x3b),
            ] {
                e(m, offset, effect)?;
            }
            if hp < 0x46 {
                e(m, 0x203, 0x3f)
            } else if hp < 0x82 {
                e(m, 0x202, 0x41)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x201, 0x41)
            }
        }
        10 | 11 => {
            e(m, 1, 0x3b)?;
            if hp < 0x3c {
                e(m, 0x102, 0x3f)
            } else if hp < 0x78 {
                e(m, 0x100, 0x40)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x101, 0x3f)
            }
        }
        13 => {
            for (offset, effect) in [(0x101, 0x3d), (0x103, 0x3b), (0x102, 0x3c)] {
                e(m, offset, effect)?;
            }
            if hp < 0x78 {
                e(m, 0x201, 0x3f)
            } else if hp < 0x96 {
                e(m, 0x202, 0x41)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x200, 0x40)
            }
        }
        14 => {
            for (offset, effect) in [
                (0x103, 0x3b),
                (0x102, 0x3c),
                (0x101, 0x3c),
                (0x200, 0x3d),
                (0x201, 0x3b),
            ] {
                e(m, offset, effect)?;
            }
            if hp < 0x5a {
                e(m, 0x203, 0x3f)
            } else if hp < 0x8c {
                e(m, 0x302, 0x41)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x200, 0x40)?;
                e(m, 0x301, 0x41)
            }
        }
        15 | 20 => {
            e(m, 0x202, 0x3b)?;
            e(m, 0x101, 0x3c)?;
            e(m, 0x202, 0x3d)?;
            if hp < 0x3c {
                e(m, 0x303, 0x3f)
            } else if hp < 0x78 {
                e(m, 0x302, 0x41)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x301, 0x40)?;
                e(m, 0x303, 0x40)
            }
        }
        16 => {
            for (offset, effect) in [(0x102, 0x3b), (0x101, 0x3c), (0x200, 0x3c), (0x202, 0x3d)] {
                e(m, offset, effect)?;
            }
            if hp < 0x5a {
                e(m, 0x302, 0x3f)
            } else if hp < 0x78 {
                e(m, 0x303, 0x41)?;
                e(m, 0x300, 0x40)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x301, 0x41)?;
                e(m, 0x304, 0x40)
            }
        }
        17 => {
            e(m, 1, 0x3b)?;
            e(m, 0x101, 0x3c)?;
            if hp < 0x3c {
                Ok(())
            } else if hp < 0x78 {
                e(m, 0, 0x3f)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 1, 0x41)
            }
        }
        18 => {
            e(m, 0x202, 0x3b)?;
            e(m, 0x101, 0x3c)?;
            e(m, 0x202, 0x3d)?;
            if hp < 0x3c {
                e(m, 0x103, 0x40)
            } else if hp < 0x78 {
                e(m, 0x201, 0x40)?;
                e(m, 0x202, 0x41)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x201, 0x40)?;
                e(m, 0x203, 0x41)
            }
        }
        19 | 21 => {
            e(m, 0x100, 0x3b)?;
            e(m, 0x101, 0x3c)?;
            if hp < 0x3c {
                e(m, 0x202, 0x3f)
            } else if hp < 0x78 {
                e(m, 0x200, 0x40)
            } else if hp >= 0xb4 {
                Ok(())
            } else {
                e(m, 0x201, 0x41)
            }
        }
        // 9, 12 and every kind above 0x15 leave through 0x41b78a.
        _ => Ok(()),
    }
}

/// AX is the building index. No refund is invented: the original has none here.
pub fn dismantle(m: &mut MemoriaJogo, index: u16) -> Result<()> {
    #[cfg(feature = "validation-trace")]
    ENTRIES.with(|entries| entries.borrow_mut().push((index, m.clone())));
    let r = record(index);
    if matches!(m.u8(r + 8)?, 9 | 12) {
        m.set_u16(r + 0x84, 0)?;
    }
    if m.u8(r + 4)? != 200 {
        m.set_u8(0x4dfab8, m.u8(r + 4)?)?;
    }
    if m.u8(r + 0x90)? != 0 {
        reset_terrain(m, index, 1)?;
        m.set_u8(r + 0x90, 0)?;
        if m.u8(0x508041)? == m.u8(r + 4)? {
            let volume = crate::projectile_building_impact::volume(m, index)?;
            if m.u32(r + 0x100)? == u32::MAX {
                crate::player::sound(
                    m,
                    0x41f0b9,
                    true,
                    [0x42, (u32::from(m.u8(r + 8)?) << 16) | 0x500, r + 0x100, 0],
                    Some(volume),
                )?;
            }
        }
    }
    let kind = m.u8(r + 8)?;
    let occupied = if kind == 21 {
        bag(m, "BagContientTroupe", r + 12)? as u8 != 0
    } else {
        (kind == 2 && bag(m, "BagContientTroupe", r + 12)? as u8 != 0)
            || bag(m, "BagContient", r + 12)? as u16 != 0
    };
    if occupied {
        let unit = crate::chain_recruit::retire_colonist(m, u32::from(index))? as u16;
        if unit != 65535 {
            let u = crate::frame_core::unit(m, u32::from(unit))?;
            let life = m.u16(u + 0x9a)?;
            m.set_u16(u + 0x9a, life - life / 2)?;
        }
        return Ok(());
    }
    let hp = m.u16(r + 0x84)?;
    if hp != 0 {
        let loss = (crate::frame_core::random(m)? % 25 + 8) as u16;
        if loss >= hp {
            m.set_u16(r + 0x84, 0)?;
        } else if crate::frame_core::random(m)? & 1 == 0 {
            m.set_u16(r + 0x84, hp.wrapping_sub(loss))?;
            smoke(m, index)?;
        }
        return Ok(());
    }
    let kind = u32::from(m.u8(r + 8)?);
    let width = m.u16(0x495d8c + 52 * kind)?;
    let height = m.u16(0x495d8e + 52 * kind)?;
    let origin = m.u16(r + 2)?;
    reset_terrain(m, index, 0)?;
    crate::player_build::reset_terrain(m, u32::from(index))?;
    crate::chain_vision::structure_minimap(m, index, 0)?;
    m.set_u8(r + 5, 0)?;
    m.set_u16(r + 10, 0)?;
    m.set_u8(r, 255)?;
    for offset in [0x74, 0x75, 0x76] {
        m.set_u8(r + offset, 0)?;
    }
    m.set_u8(r + 0x7b, 255)?;
    m.set_u16(r + 0x78, 65535)?;
    for offset in [0x7c, 0x7e, 0x80, 0x82] {
        m.set_u16(r + offset, 0)?;
    }
    m.set_u32(r + 0x8c, 0)?;
    let selected = 0x495be0 + 2 * u32::from(m.u8(0x4dfab8)?);
    if m.u16(selected)? == index {
        m.set_u16(selected, 65535)?;
    }
    for player in 0..4 {
        if m.u8(r + 0x110 + player)? != 0 {
            crate::frame_core::call(m, "BatimentIllumine", &[u32::from(index), player, 0])?;
            m.set_u8(r + 0x110 + player, 0)?;
        }
    }
    for row in 0..height {
        for col in 0..width {
            let draw = crate::frame_core::random(m)?;
            let cell = u32::from(origin) + 256 * u32::from(row) + u32::from(col);
            let object = m.u32(0x507d74)?.wrapping_add(cell);
            if draw % 3 == 0 && m.u8(object)? == 0 {
                let value = (crate::frame_core::random(m)? % 11 + 12) as u8;
                m.set_u8(object, value)?;
            } else if m.u8(object)? == 0 {
                let value = (crate::frame_core::random(m)? & 7) as u16 + 0x19d;
                m.set_u16(m.u32(0x507d70)?.wrapping_add(2 * cell), value)?;
            }
        }
    }
    Ok(())
}
