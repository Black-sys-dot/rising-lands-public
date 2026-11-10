//! EnginNextEtape attack dispatch (0x456886) and ground-attack approach.
//! Translated directly from intact asm. Unported damage consumers stop explicitly.
use crate::{frame::stop, player::unit, FrameRegisters, MemoriaJogo, Result};

pub fn state1(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    if !validate_target(m, u, r)? || m.u8(r + 2)? != 1 {
        return Ok(());
    }
    // 4567ae/4567c9 return after invalid-target handling. Only the valid
    // 45689e arm calls CalculAttaqueEngin in this step.
    if m.u8(r + 0xb7)? == 3 {
        crate::ai_building_fire_chain::approach(m, u)
    } else {
        approach(m, u)
    }
}

/// Same target validation at 0x456886 and hunger saved-state entry 0x455393.
pub(crate) fn validate_target(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<bool> {
    let mode = m.u8(r + 0xb7)?;
    let target = m.u16(r + 0xb8)?;
    let invalid = match mode {
        1 => {
            if target == 65535 {
                true
            } else {
                let t = unit(m, u32::from(target))?;
                matches!(m.u8(t + 2)?, 0 | 4) || m.u8(t)? == 200 || m.u8(t + 0x72)? == 0
            }
        }
        3 => {
            if target == 65535 {
                true
            } else {
                let t = crate::player::structure(u32::from(target));
                m.u8(t + 5)? == 3 || m.u8(t + 4)? == 200
            }
        }
        _ => false,
    };
    if invalid {
        let owner = m.u8(r)?;
        if crate::frame_economy::call(m, "PlayerGetType", &[u32::from(owner)])? as u8 == 2 {
            crate::frame_ai::call(m, "iagUnitSendMessage", &[u32::from(u), 0x4a, 0])?;
        } else {
            crate::frame_spawn::call(
                m,
                "SetDeplaceEngin",
                &[u32::from(u), u32::from(m.u16(r + 0x7e)?)],
            )?;
            m.set_u16(r + 0xb8, 65535)?;
        }
        return Ok(false);
    }
    Ok(true)
}

fn release_destination(m: &mut MemoriaJogo, u: u16, r: u32, cell: u16) -> Result<()> {
    m.set_u16(r + 0x7e, cell)?;
    crate::frame_spawn::release(m, u32::from(u))?;
    m.set_u8(r + 0x88, 0)
}
fn distance(a: u16, b: u16) -> u32 {
    let dx = i32::from(a & 255) - i32::from(b & 255);
    let dy = i32::from(a >> 8) - i32::from(b >> 8);
    (dx * dx + dy * dy) as u32
}
fn ground_range(m: &mut MemoriaJogo, u: u16, r: u32, target: u16) -> Result<bool> {
    let cell = m.u16(r + 8)?;
    let kind = m.u8(r + 1)?;
    if kind == 16 || m.u8(r + 2)? == 24 {
        return crate::spell_range::in_range(m, u, 5, target);
    }
    if m.u8(r + 0x78)? == 0 || matches!(kind, 1 | 4 | 21) {
        // EnginAdjacentCase 0x446b0c: the whole expanded rectangle, interior included.
        if crate::spell_range::adjacent_cell(m, u, target)? {
            return Ok(true);
        }
    }
    let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(r)?), u32::from(kind), 4, 0)?;
    Ok(distance(cell, target) <= (u32::from(m.u8(r + 0xb0)?).wrapping_add(bonus) & 65535))
}
fn line_of_fire(m: &MemoriaJogo, r: u32, target: u16) -> Result<bool> {
    if matches!(m.u8(r + 1)?, 4 | 8 | 12 | 22 | 23) {
        return Ok(true);
    }
    let mut cell = m.u16(r + 0x7c)?;
    for _ in 0..256 {
        let x = i32::from(cell & 255);
        let y = i32::from(cell >> 8);
        let tx = i32::from(target & 255);
        let ty = i32::from(target >> 8);
        cell = (x + (tx - x).signum() + ((y + (ty - y).signum()) << 8)) as u16;
        let terrain = m.u16(m.u32(0x507d70)? + 2 * u32::from(cell))?;
        if (0x1dd..0x260).contains(&terrain) || cell == target {
            return Ok(cell == target);
        }
    }
    stop("CalculAttaqueEngin line bounds", 0x451836)
}

pub fn approach(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    if !matches!(m.u8(r + 0xb7)?, 1 | 5) {
        return stop("CalculAttaqueEngin building/other target", 0x451914);
    }
    match m.u8(r + 0x8a)? {
        1 => return firing(m, u, r),
        2 => return recovery(m, u, r),
        3 => {
            if !matches!(m.u8(r + 2)?, 0 | 4) {
                m.set_u8(r + 2, 7)?;
                crate::frame_spawn::release(m, u32::from(u))?;
            }
            m.set_u16(r + 0xb8, 65535)?;
            m.set_u8(r + 0xa4, 0)?;
            return m.set_u8(r + 0xa9, 0);
        }
        0 => {}
        _ => return Ok(()), // Native dispatch exits for auxiliary states >3.
    }
    if m.u8(r + 0x86)? != 0 || m.u8(r + 0x87)? != 0 {
        crate::frame_movement::displace(m, u32::from(u))?;
        return Ok(());
    }
    let target = attack_target(m, r)?;
    if !attack_range(m, u, r, target)? || !line_of_fire(m, r, target)? {
        if !attack_range(m, u, r, target)? {
            if target != m.u16(r + 0x7e)? {
                release_destination(m, u, r, target)?;
            }
            crate::frame_movement::displace(m, u32::from(u))?;
            return Ok(());
        }
        // 452010 follows the second EnginAPortee call, after the failed
        // firing-line test. It chooses a movement goal, not an attack result.
        let origin = m.u16(r + 8)?;
        let close_target16 = m.u8(r + 0xb7)? == 1
            && m.u8(unit(m, u32::from(m.u16(r + 0xb8)?))? + 1)? == 16
            && distance(origin, target) <= 4;
        let mut destination = target;
        if close_target16 {
            // 4521c4..452202: both neighbours are measured from the same
            // origin, with the second heading derived from the first result.
            if target != origin {
                let direction = crate::frame_path::direction(target, origin, false);
                let first = crate::projectile_occupied_impact::neighbor(m, origin, direction)?;
                let direction = crate::frame_path::direction(first, origin, false);
                destination = crate::projectile_occupied_impact::neighbor(m, origin, direction)?;
            }
        } else if matches!(m.u8(r + 1)?, 4 | 8 | 12 | 22 | 23) {
            // Native 4522a2 returns without movement for these bypass types.
            return Ok(());
        }
        if destination != m.u16(r + 0x7e)? {
            m.set_u16(r + 0x7e, destination)?;
            if m.u16(0x496304)? == u {
                m.set_u8(0x4e0f36, 0)?;
                // The two inline release arms have distinct store order.
                if close_target16 {
                    m.set_u8(0x4e0f37, 0)?;
                    m.set_u16(0x496304, 65535)?;
                    m.set_u32(0x4e0f3c, 0)?;
                } else {
                    m.set_u32(0x4e0f3c, 0)?;
                    m.set_u8(0x4e0f37, 0)?;
                    m.set_u16(0x496304, 65535)?;
                }
            }
            m.set_u8(r + 0x88, 0)?;
        }
        crate::frame_movement::displace(m, u32::from(u))?;
        return Ok(());
    }
    release_destination(m, u, r, m.u16(r + 0x7c)?)?;
    let kind = m.u8(r + 1)?;
    let turret = matches!(kind, 12 | 15);
    let mut direction = angle(m.u16(r + 8)?, target, if turret { 32 } else { 8 });
    if !turret {
        direction = direction.wrapping_mul(2).wrapping_sub(1);
        if kind == 3 {
            direction = direction.wrapping_add(11) % 16 + 1;
        }
    }
    let (desired, current) = if turret { (0xda, 0x94) } else { (0x90, 0x91) };
    m.set_u8(r + desired, direction)?;
    if direction != m.u8(r + current)? {
        if turret {
            rotate_turret(m, r)?;
        } else {
            crate::frame_movement::call(m, "RotateEngin", &[u32::from(u)])?;
        }
        return Ok(());
    }
    let reload = i32::from(m.u16(0x4965de + 50 * u32::from(kind))?)
        - crate::frame_economy::bonus(m, u32::from(m.u8(r)?), u32::from(kind), 3, 0)? as i32;
    if i32::from(m.u16(r + 0x102)?) < reload {
        return Ok(());
    }
    m.set_u16(r + 0x102, 0)?;
    m.set_u8(r + 0x8a, 1)?;
    reset_animation(m, r)?;
    if matches!(kind, 14 | 16) && m.u8(r + 0xda)? == 0 {
        crate::frame_spawn::call(
            m,
            "SetDeplaceEngin",
            &[u32::from(u), u32::from(m.u16(r + 0x7c)?)],
        )?;
    }
    Ok(())
}

/// AngleCalcul0x4796cb, u8 coordinates and the original double pi constant.
pub(crate) fn angle(from: u16, to: u16, sectors: u8) -> u8 {
    let (x, y, tx, ty) = (from & 255, from >> 8, to & 255, to >> 8);
    let dx = f64::from(x.abs_diff(tx));
    let dy = f64::from(y.abs_diff(ty));
    let mut degrees = if dy == 0.0 {
        0.0
    } else if dx == 0.0 {
        90.0
    } else {
        (dx / (dx * dx + dy * dy).sqrt()).acos() * 180.0 / f64::from_bits(0x400921fb54411744)
    };
    if x > tx {
        degrees = if y >= ty {
            degrees + 180.0
        } else {
            180.0 - degrees
        };
    } else if y > ty {
        degrees = 360.0 - degrees;
    }
    let step = 360.0 / f64::from(sectors);
    degrees += step * 0.5;
    if degrees >= 360.0 {
        degrees -= 360.0;
    }
    ((degrees / step) as u8 + sectors - sectors / 2) % sectors + 1
}
fn rotate_turret(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    let desired = m.u8(r + 0xda)?;
    let current = m.u8(r + 0x94)?;
    if desired != current {
        let count = m.u8(r + 0x96)?.wrapping_add(1);
        m.set_u8(r + 0x96, count)?;
        if count == m.u8(r + 0x93)? {
            let increase = (desired > current && desired - current < 16)
                || (current > desired && current - desired > 16);
            let next = if increase {
                current % 32 + 1
            } else if current == 1 {
                32
            } else {
                current.wrapping_sub(1)
            };
            m.set_u8(r + 0x94, next)?;
            m.set_u8(r + 0x96, 0)?;
        }
    }
    Ok(())
}
fn reset_animation(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    if m.u8(r + 1)? != 16 {
        m.set_u8(r + 0xa4, 0)?;
        m.set_u8(r + 0xa9, 0)?;
    }
    Ok(())
}
fn animate(m: &mut MemoriaJogo, r: u32, attack: bool) -> Result<()> {
    let count = m.u8(r + 0xa9)?.wrapping_add(1);
    m.set_u8(r + 0xa9, count)?;
    if count == m.u8(r + if attack { 0xa8 } else { 0xa7 })? {
        let frames = u32::from(m.u8(r + if attack { 0xa6 } else { 0xa5 })?);
        if frames == 0 {
            return stop("AnimAttaqueEngin division", 0x447a4d);
        }
        let next = ((u32::from(m.u8(r + 0xa4)?) + 1) % frames) as u8;
        // Original447a4f clears the timer before447a56 publishes the frame.
        m.set_u8(r + 0xa9, 0)?;
        m.set_u8(r + 0xa4, next)?;
    }
    Ok(())
}
fn firing(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    let kind = m.u8(r + 1)?;
    // 0x4523de AnimAttaqueEngin 0x4479c4: its kind chain 0x4479e6..0x447ac9 lets only these
    // kinds reach the phase table; 12, 13, 16 and 17 return untouched.
    if matches!(kind, 0..=4 | 8 | 11 | 14 | 15 | 18..=25) {
        animate(m, r, true)?;
    }
    if m.u8(r + 0xa9)? == 0 && m.u8(r + 0xa4)? == m.u8(r + 0x7a)? {
        fire(m, u, r)?;
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
pub(crate) fn recovery(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    let kind = m.u8(r + 1)?;
    if kind == 16 {
        animate(m, r, false)?;
    }
    if kind == 17 {
        // 0x452a5f: AnimBallonStatic, then the shared 0x452ab4 continuation.
        crate::frame_movement::balloon_animation(m, u32::from(u))?;
    }
    if m.u8(r + 0x8b)? == 1 {
        reset_animation(m, r)?;
    }
    let remaining = m.u8(r + 0x8b)?.wrapping_sub(1);
    m.set_u8(r + 0x8b, remaining)?;
    if remaining == 0 {
        m.set_u8(r + 0x8a, 1)?;
        reset_animation(m, r)?;
        if matches!(kind, 14 | 16) {
            if m.u8(r + 0xda)? == 0 {
                crate::frame_spawn::call(
                    m,
                    "SetDeplaceEngin",
                    &[u32::from(u), u32::from(m.u16(r + 0x7c)?)],
                )?;
            }
        } else if m.u8(r + 0xb7)? == 3 {
            // 0x452b3e: EnginAPortee(mode 3), then EnginViseCible 0x451464.
            if !crate::ai_building_attack::in_range(m, u)?
                || !crate::ai_building_attack::faces_target(m, r)?
            {
                m.set_u8(r + 0x8a, 0)?;
            }
        } else {
            let target = attack_target(m, r)?;
            let direction = angle(
                m.u16(r + 8)?,
                target,
                if matches!(kind, 12 | 15) { 32 } else { 8 },
            );
            let direction = if matches!(kind, 12 | 15) {
                direction
            } else if kind == 3 {
                (2 * direction - 1 + 11) % 16 + 1
            } else {
                2 * direction - 1
            };
            if !attack_range(m, u, r, target)?
                || m.u8(r + if matches!(kind, 12 | 15) { 0x94 } else { 0x91 })? != direction
            {
                m.set_u8(r + 0x8a, 0)?;
            }
        }
    }
    Ok(())
}
fn fire(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    let target = attack_target(m, r)?;
    if m.u8(r + 2)? == 24 {
        let registers = FrameRegisters {
            esi: r,
            edi: u32::from(u),
            ..Default::default()
        };
        match m.u8(r + 0xf4)? {
            0 | 4 => {
                crate::spell_projectiles::attack_call(m, registers)?;
            }
            1 | 2 => {
                crate::spell_remaining::attack_call(m, registers)?;
            }
            3 => {
                crate::spell_effects::armor_attack_call(m, registers)?;
            }
            _ => return stop("SortilegeEffet", 0x473510),
        }
        crate::frame_spawn::reset_order(m, u32::from(u))?;
        return Ok(());
    }
    if m.u8(r + 2)? != 1 {
        return Ok(());
    }
    let kind = m.u8(r + 1)?;
    let cell = m.u16(r + 8)?;
    let light = m.u32(0x507d78 + 4 * u32::from(m.u8(0x508041)?))?;
    if m.u8(light + u32::from(cell))? == 0 {
        let volume = crate::player_orders::unit_volume(m, u32::from(u))?;
        m.set_u8(r + 0xf9, 0)?;
        if m.u32(r + 0x104)? == u32::MAX {
            crate::player::sound(
                m,
                0x4525a1,
                true,
                [0x50, (u32::from(kind) << 16) | 0x2300, r + 0x104, 0],
                Some(volume),
            )?;
        }
    }
    if matches!(kind, 14 | 16) && m.u8(r + 0xda)? != 0 {
        m.set_u8(r + 0xda, m.u8(r + 0xda)?.wrapping_sub(1))?;
    }
    if m.u8(r + 0x78)? == 0 {
        let hit = if matches!(kind, 3 | 11..=16 | 25) {
            target
        } else {
            crate::frame_path::neighbor(m, cell, m.u8(r + 0x91)?)?
        };
        if kind == 16 {
            projectile(
                m,
                &[
                    8,
                    u32::from(hit),
                    u32::from(hit),
                    1,
                    u32::from(u),
                    0,
                    0,
                    0,
                    0,
                ],
            )?;
        }
        return crate::chain_damage::impact(m, hit, u);
    }
    let (dx, dy) = if matches!(kind, 1 | 11 | 21) {
        (0, 0)
    } else {
        muzzle(m, r)?
    };
    let direction = if kind == 11 {
        angle(cell, target, 32)
    } else {
        m.u8(r + 0x91)?
    };
    if !matches!(kind, 3 | 25) {
        let target_flag = m.u8(r + 0xb7)? == 1
            && matches!(m.u8(unit(m, u32::from(m.u16(r + 0xb8)?))? + 1)?, 16 | 17);
        projectile(
            m,
            &[
                u32::from(m.u8(r + 0x79)?),
                u32::from(cell),
                u32::from(target),
                1,
                u32::from(u),
                u32::from(direction),
                dx as i16 as i32 as u32,
                dy as i16 as i32 as u32,
                u32::from(target_flag),
            ],
        )?;
    }
    if matches!(kind, 3 | 11 | 25) {
        projectile(
            m,
            &[
                if kind == 25 { 33 } else { 26 },
                u32::from(target),
                u32::from(target),
                1,
                u32::from(u),
                u32::from(direction),
                0,
                0,
                0,
            ],
        )?;
    }
    Ok(())
}
fn projectile(m: &mut MemoriaJogo, args: &[u32]) -> Result<()> {
    crate::frame_combat::call(m, "ProjectileInit", args).map(|_| ())
}
fn muzzle(m: &MemoriaJogo, r: u32) -> Result<(u16, u16)> {
    let kind = m.u8(r + 1)?;
    let direction = m.u8(r + if matches!(kind, 12 | 15) { 0x94 } else { 0x91 })?;
    let table = match kind {
        2 => Some(0x496af0),
        8 => Some(0x496b10),
        18 => Some(0x496b30),
        23 => Some(0x496b50),
        24 => Some(0x496b70),
        21 => Some(0x496b90),
        22 => Some(0x496bb0),
        15 => Some(0x496bd0),
        12 => Some(0x496c50),
        _ => None,
    };
    let (mut x, mut y) = (0u16, 0u16);
    if let Some(table) = table {
        let index = if matches!(kind, 12 | 15) {
            direction.wrapping_sub(1)
        } else {
            direction.wrapping_sub(1) / 2
        };
        if index >= if matches!(kind, 12 | 15) { 32 } else { 8 } {
            return stop("Projectile muzzle direction", 0x4511a5);
        }
        x = m.u16(table + 4 * u32::from(index))?;
        y = m.u16(table + 4 * u32::from(index) + 2)?;
    }
    // AffCorpsEnginDecalage0x410fcc, original type-indexed switch.
    let (bx, by) = match kind {
        0 => (40, 22),
        1 => (23, 9),
        2 => (32, 16),
        3 => (45, 56),
        4 => (40, 32),
        5 => (11, 11),
        6 => (16, 20),
        7 => (26, 26),
        8 => (32, 30),
        9 => (19, 14),
        10 => (24, 12),
        11 => (41, 52),
        12 => (52, 48),
        13 => (55, 56),
        14 => (42, 45),
        15 => (49, 66),
        16 => (78, 54),
        17 => (79, 212),
        18 => (36, 48),
        19 => (16, 16),
        20 => (46, 54),
        21 => (23, 15),
        22 => (54, 54),
        23 => (22, 16),
        24 => (32, 32),
        25 => (45, 56),
        _ => return stop("Projectile body offset type", 0x410fcc),
    };
    Ok((x.wrapping_sub(bx), y.wrapping_sub(by)))
}
/// Typed body transport: scratch GPRs are outside the effects-only contract.
pub fn fixture(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    state1(m, r.edi as u16, r.esi)?;
    Ok(r)
}

/// Native mode1 resolves the live target cell for range, movement, aim and fire.
fn attack_target(m: &MemoriaJogo, r: u32) -> Result<u16> {
    let target = m.u16(r + 0xb8)?;
    match m.u8(r + 0xb7)? {
        1 => m.u16(unit(m, u32::from(target))? + 0x7c),
        5 => Ok(target),
        _ => stop("CalculAttaqueEngin target mode", 0x451914),
    }
}
fn attack_range(m: &mut MemoriaJogo, u: u16, r: u32, target: u16) -> Result<bool> {
    if m.u8(r + 0xb7)? == 1 {
        crate::chain_battle::in_range(m, u, m.u16(r + 0xb8)?)
    } else {
        ground_range(m, u, r, target)
    }
}
