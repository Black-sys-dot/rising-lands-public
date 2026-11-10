//! SetDyingEngin0x444938 and state4 (flying group transports included), translated from intact instructions.
use crate::{
    frame::stop,
    player::{unit, Memory},
    MemoriaJogo, Result,
};
fn clear_hover(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    if m.w(0x496304)? == u32::from(u) {
        m.sb(0x4e0f36, 0)?;
        m.sd(0x4e0f3c, 0)?;
        m.sb(0x4e0f37, 0)?;
        m.sw(0x496304, 65535)?;
    }
    Ok(())
}
fn deselect(m: &mut MemoriaJogo, u: u16, site: u32) -> Result<()> {
    crate::player_orders::call(m, "DeselectionEngin", [u32::from(u), 0, 0, 0], site)?;
    Ok(())
}
fn reset_collision(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    let cell = m.w(r + 8)?;
    crate::frame_movement::call(m, "ResetEnginCollision", &[u32::from(u), cell])?;
    if cell != m.w(r + 0x7c)? {
        crate::frame_movement::call(m, "ResetEnginCollision", &[u32::from(u), m.w(r + 0x7c)?])?;
    }
    Ok(())
}
pub fn set(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    if m.b(r + 0xdc)? & 4 != 0 {
        crate::player::sound(m, 0x444c84, false, [r + 0x104, 0, 0, 0], None)?;
        let on_map = m.b(r + 0x72)?;
        deselect(m, u, 0x444c96)?;
        reset_collision(m, u, r)?;
        if !matches!(m.b(r + 2)?, 0 | 4) {
            m.sb(r + 2, 0)?;
            if m.w(0x496304)? == u32::from(u) {
                m.sb(0x4e0f36, 0)?;
                m.sb(0x4e0f37, 0)?;
                m.sd(0x4e0f3c, 0)?;
                m.sw(0x496304, 65535)?;
            }
        }
        if on_map != 0 {
            let cell = m.w(r + 8)?;
            let owner = m.b(r)?;
            crate::player_sight::minimap(m, owner as u8, cell as u16, 5)?;
            if !matches!(owner, 4 | 200) {
                crate::player_sight::area(
                    m,
                    cell as u16,
                    m.b(r + 0xbb)? + m.b(r + 0xf5)?,
                    owner as u8,
                    2,
                )?;
            }
            crate::frame_combat::call(m, "ProjectileInit", &[31, cell, cell, 0, 0, 0, 0, 0, 0])?;
        }
        return Ok(());
    }
    if matches!(m.b(r + 2)?, 0 | 4) {
        return Ok(());
    }
    crate::frame_ai::call(m, "iagUnitSendMessage", &[u32::from(u), 0x44, 0])?;
    crate::player::sound(m, 0x4449c3, false, [r + 0x104, 0, 0, 0], None)?;
    if crate::frame_core::call(m, "TerrainEstDecouvert", &[m.w(r + 8)?, m.b(0x508041)?])? & 255 != 0
    {
        let volume = crate::player_orders::unit_volume(m, u32::from(u))?;
        m.sb(r + 0xf9, 0)?;
        if m.d(r + 0x104)? == u32::MAX {
            crate::player::sound(
                m,
                0x444a1b,
                true,
                [0x50, (m.b(r + 1)? << 16) | 0x500, r + 0x104, 0],
                Some(volume),
            )?;
        }
    }
    if !matches!(m.b(r + 2)?, 0 | 4) {
        m.sb(r + 2, 4)?;
        clear_hover(m, u)?;
    }
    crate::chain_cargo::drop_all(m, u)?;
    // 0x444ad6: the second state setter. The cargo release can detach a
    // group transport (ResetOrdre writes state 2), which goes back to 4.
    if !matches!(m.b(r + 2)?, 0 | 4) {
        m.sb(r + 2, 4)?;
        clear_hover(m, u)?;
    }
    m.sb(r + 0xa4, 0)?;
    m.sw(r + 0xc2, 0)?;
    deselect(m, u, 0x444b71)?;
    reset_collision(m, u, r)?;
    // 0x444bbd: a dying group transport (types 11..=17) leaves a wreck
    // effect at its cell: type 16 effect 0x22 lifted by its altitude +0xab,
    // the others effect 0x12 or 0x13 (Random bit) offset by (-32, -32).
    let kind = m.b(r + 1)?;
    if (11..=17).contains(&kind) {
        let cell = m.w(r + 8)?;
        let args = if kind == 16 {
            let lift = (m.b(r + 0xab)? as i32).wrapping_neg() as u32;
            [0x22, cell, cell, 0, 0, 0, 0, lift, 0]
        } else {
            let effect = if crate::frame_core::random(m)? & 1 == 0 {
                0x12
            } else {
                0x13
            };
            [effect, cell, cell, 0, 0, 0, 0xffff_ffe0, 0xffff_ffe0, 0]
        };
        crate::frame_combat::call(m, "ProjectileInit", &args)?;
    }
    Ok(())
}
pub fn state4(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    let kind = m.b(r + 1)?;
    if (11..=17).contains(&kind) {
        // 0x456b2f: a dead group transport is removed at once. Unless +0xdc
        // bit 2 is set it leaves sacks of resource 0 at its cell, as many as
        // the word 0x4965b2 of its type; then its cargo is released.
        if crate::frame_core::call(m, "GetBit", &[m.b(r + 0xdc)?, 2])? as u8 == 0 {
            let count = m.w(0x4965b2 + 50 * kind)?;
            for _ in 0..count {
                crate::chain_cargo::deposit(m, m.w(r + 8)? as u16, 0)?;
            }
        }
        deselect(m, u, 0x456bd9)?;
        crate::chain_cargo::drop_all(m, u)?;
        crate::chain_entry::retire(m, u)?;
        m.sw(r + 0x9a, 0)?;
        m.sb(r + 2, 0)?;
        return Ok(());
    }
    let phase = m.b(r + 0xa4)?;
    if phase == 0 {
        deselect(m, u, 0x456a0f)?;
        reset_collision(m, u, r)?;
    }
    let count = m.b(r + 0xdb)?;
    if phase != count.wrapping_sub(1) {
        if count == 0 {
            return stop("EnginNextEtape death animation division", 0x456a71);
        }
        m.sb(r + 0xa4, (phase + 1) % count)?;
    }
    let timer = (m.w(r + 0xc2)? + 1) & 65535;
    m.sw(r + 0xc2, timer)?;
    if timer >= 330 {
        deselect(m, u, 0x456ab3)?;
        crate::chain_cargo::drop_all(m, u)?;
        crate::chain_entry::retire(m, u)?;
        m.sw(r + 0x9a, 0)?;
        m.sb(r + 2, 0)?;
    }
    Ok(())
}
/// EnginSetLevelExperience0x44fb20. The packed level accepts only DL0..4.
pub fn level(m: &mut MemoriaJogo, u: u16, level: u8) -> Result<()> {
    if level >= 5 {
        return Ok(());
    }
    let r = unit(m, u32::from(u))?;
    m.sb(r + 0xaf, (m.b(r + 0xaf)? & 31) | (u32::from(level) << 5))?;
    let kind = m.b(r + 1)?;
    let combat = matches!(kind,0..=3|8|11|12|15|16|18..=25);
    match level {
        0 => {}
        1 | 4 => {
            if level == 1 && combat {
                m.sb(r + 0xf7, 1)?;
            } else {
                // Level4's combat branch jumps to the same magic-percentage
                // body as level1's noncombat branch, not to the damage setter.
                let gain = if level == 4 && !combat { 2 } else { 1 };
                m.sb(r + 0xf8, (m.w(0x4965dc + 50 * kind)? * gain / 100) & 255)?;
            }
        }
        2 => {
            m.sb(r + 0xf6, 1)?;
        }
        3 => {
            if m.b(r + 0x72)? == 0 {
                m.sb(r + 0xf5, 1)?;
                return Ok(());
            }
            crate::chain_entry::retire(m, u)?;
            m.sb(r + 0xf5, 1)?;
            let cell = m.w(r + 8)?;
            crate::frame_core::call(m, "SetEnginCollision", &[u32::from(u), cell])?;
            m.sb(r + 0x72, 1)?;
            let owner = m.b(r)?;
            if !matches!(owner, 4 | 200) {
                let radius = m.b(r + 0xbb)? + m.b(r + 0xf5)?;
                crate::player_sight::area(
                    m,
                    cell as u16,
                    (radius + 2) as u16 as u32,
                    owner as u8,
                    0,
                )?;
                crate::player_sight::area(m, cell as u16, radius as u16 as u32, owner as u8, 1)?;
            }
            crate::player_sight::minimap(m, owner as u8, cell as u16, 4)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

/// EnginInfligeDegat0x4506a1 lethal mode1 continuation.
pub fn lethal(
    m: &mut MemoriaJogo,
    victim: u16,
    attacker: u16,
    source_owner: u8,
    impact_cell: u16,
    unit_attacker: bool,
) -> Result<()> {
    let source = unit(m, u32::from(attacker))?;
    let target = unit(m, u32::from(victim))?;
    // 0x4506a1: only mode 1 (a unit attacker) gains experience.
    if unit_attacker && matches!(m.b(source + 1)?, 0..=10 | 25) {
        let packed = m.b(source + 0xaf)? as u8;
        // Native adds ten to the low five bits, so every kill reaches its
        // threshold. Preserve the packed-byte clear before the level call.
        m.sb(source + 0xaf, u32::from(packed & 0xe0))?;
        level(m, attacker, (packed >> 5).wrapping_add(1))?;
    }
    m.sw(target + 0x9a, 0)?;
    if m.b(target + 1)? == 1 {
        let cell = u32::from(impact_cell);
        crate::frame_combat::call(m, "ProjectileInit", &[6, cell, cell, 0, 0, 0, 0, 0, 0])?;
    }
    set(m, victim)?;
    let kind = m.b(target + 1)?;
    crate::frame_economy::call(
        m,
        "MAJSympathie",
        &[
            m.b(target)?,
            u32::from(source_owner),
            u32::from(kind == 3 || (11..=17).contains(&kind)),
        ],
    )?;
    Ok(())
}
