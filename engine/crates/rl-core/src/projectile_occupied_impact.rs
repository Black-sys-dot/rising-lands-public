//! Intact occupied InfligeDegat mode4 and EnginInfligeDegat directions29/30.
//! Selected effects API; building impact and unknown initialized locals stop exactly.
use crate::{frame::stop, player::unit, FrameRegisters, MemoriaJogo, Result};

#[cfg(feature = "validation-trace")]
std::thread_local! { static CALLS: std::cell::RefCell<Vec<[u32;5]>> = const { std::cell::RefCell::new(Vec::new()) }; }
#[cfg(feature = "validation-trace")]
pub fn take_validation_calls() -> Vec<[u32; 5]> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}
fn entered(address: u32, args: [u32; 4]) {
    #[cfg(feature = "validation-trace")]
    CALLS.with(|c| {
        c.borrow_mut()
            .push([address, args[0], args[1], args[2], args[3]])
    });
    #[cfg(not(feature = "validation-trace"))]
    let _ = (address, args);
}
fn move_to(m: &mut MemoriaJogo, victim: u16, cell: u16) -> Result<()> {
    entered(0x4457d3, [u32::from(victim), u32::from(cell), 0, 0]);
    crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(victim), u32::from(cell)])?;
    Ok(())
}

/// PlayerResetAllie46d8cb: alliance store before the original queue producer.
fn reset_allie(m: &mut MemoriaJogo, owner: u8, other: u8) -> Result<()> {
    if owner == other || owner > 4 || other > 4 {
        return Ok(());
    }
    let a = m.u32(0x5080a8)?.wrapping_add(20 * u32::from(owner) + 6);
    m.set_u8(a, m.u8(a)? & !(1 << other))?;
    if owner == m.u8(0x508041)? {
        entered(0x46994b, [10, u32::from(other) + 402, 0, 0]);
        return crate::messagebox_state::enqueue(m, 10, u16::from(other) + 402);
    }
    Ok(())
}

/// CalculDirCase4770ee: equality guards, not a general in-bounds test.
/// Width/height zero and already-outside cells retain the native word wrap.
pub fn neighbor(m: &MemoriaJogo, cell: u16, direction: u8) -> Result<u16> {
    entered(0x4770ee, [u32::from(cell), u32::from(direction), 0, 0]);
    let (x, y) = (i32::from(cell & 255), i32::from(cell >> 8));
    let blocked = match direction {
        1 => x == 0,
        3 => x == 0 || y == 0,
        5 => y == 0,
        7 => y == 0 || x == i32::from(m.u16(0x497168)?) - 1,
        9 => x == i32::from(m.u16(0x497168)?) - 1,
        11 => x == i32::from(m.u16(0x497168)?) - 1 || y == i32::from(m.u16(0x49716a)?) - 1,
        13 => y == i32::from(m.u16(0x49716a)?) - 1,
        15 => x == 0 || y == i32::from(m.u16(0x49716a)?) - 1,
        _ => false,
    };
    if blocked {
        return Ok(65535);
    }
    let offset = match direction {
        1 => -1,
        3 => -257,
        5 => -256,
        7 => -255,
        9 => 1,
        11 => 257,
        13 => 256,
        15 => 255,
        _ => 0,
    };
    Ok(cell.wrapping_add(offset as u16))
}

/// EnginInfligeDegat4502f0, selected fully initialized mode4 ABI.
pub fn consume(m: &mut MemoriaJogo, victim: u16, attacker: u16, direction: u8) -> Result<()> {
    entered(
        0x4502f0,
        [
            u32::from(victim),
            4,
            u32::from(attacker),
            u32::from(direction),
        ],
    );
    if !matches!(direction, 29 | 30) {
        return stop("Mode4 uninitialized damage direction", 0x4504c6);
    }
    let t = unit(m, u32::from(victim))?;
    let s = unit(m, u32::from(attacker))?;
    let cell = m.u16(t.wrapping_add(0x7c))?;
    let kind = m.u8(t.wrapping_add(1))?;
    if kind > 25 {
        return stop("Mode4 uninitialized type class", 0x450526);
    }
    let mut damage = m.u8(if direction == 29 { 0x49716c } else { 0x49716e })?;
    // The jump table sets both local words to2 for these large footprints.
    if matches!(kind, 3 | 11..=16 | 25) {
        damage = ((damage as i8 as i32) / 4) as u8;
    }
    let source_owner = m.u8(s)?;
    damage = damage.wrapping_sub(m.u8(t.wrapping_add(0xf6))?);
    damage = damage.wrapping_sub(crate::frame_economy::bonus(
        m,
        u32::from(source_owner),
        u32::from(m.u8(s.wrapping_add(1))?),
        6,
        0,
    )? as u8);
    if (damage as i8) < 0 {
        damage = 1;
    }
    if crate::frame_core::call(
        m,
        "TerrainEstDecouvert",
        &[
            u32::from(m.u16(t.wrapping_add(8))?),
            u32::from(m.u8(0x508041)?),
        ],
    )? as u8
        != 0
    {
        let volume = crate::player_orders::unit_volume(m, u32::from(victim))?;
        m.set_u8(t.wrapping_add(0xf9), 0)?;
        if m.u32(t.wrapping_add(0x104))? == u32::MAX {
            crate::player::sound(
                m,
                0x45063f,
                true,
                [
                    0x50,
                    (u32::from(kind) << 16) | 0x100,
                    t.wrapping_add(0x104),
                    0,
                ],
                Some(volume),
            )?;
        }
    }
    if m.u8(t.wrapping_add(0xdc))? & 8 != 0 {
        damage = damage.wrapping_sub(1);
    }
    let hp = i32::from(m.u16(t.wrapping_add(0x9a))?);
    let signed = i32::from(damage as i8);
    if hp > signed {
        return m.set_u16(t.wrapping_add(0x9a), (hp - signed) as u16);
    }
    // Native mode4 bypasses the mode1 experience block at4506b1.
    m.set_u16(t.wrapping_add(0x9a), 0)?;
    if m.u8(t.wrapping_add(1))? == 1 {
        spawn(m, [6, u32::from(cell), u32::from(cell), 0, 0, 0, 0, 0, 0])?;
    }
    crate::chain_death::set(m, victim)?;
    let kind = m.u8(t.wrapping_add(1))?;
    crate::frame_economy::call(
        m,
        "MAJSympathie",
        &[
            u32::from(m.u8(t)?),
            u32::from(source_owner),
            u32::from(kind == 3 || (11..=17).contains(&kind)),
        ],
    )?;
    Ok(())
}

fn react(m: &mut MemoriaJogo, victim: u16) -> Result<()> {
    let t = unit(m, u32::from(victim))?;
    let owner = m.u8(t)?;
    if owner == 200 {
        return Ok(());
    }
    if crate::frame_economy::call(m, "PlayerGetType", &[u32::from(owner)])? as u8 == 1
        && matches!(m.u8(t.wrapping_add(2))?, 5 | 18)
    {
        let building = crate::chain_s1::entry(
            m,
            "ChercheBatimentProximiteCaveOuSanctuaire",
            FrameRegisters {
                eax: u32::from(m.u16(t.wrapping_add(8))?),
                edx: u32::from(owner),
                ..Default::default()
            },
        )?
        .eax as u16;
        if building != 65535 {
            let cell = crate::frame_core::building_cell(m, u32::from(building))?;
            move_to(m, victim, cell as u16)?;
        }
    } else {
        // For mode4,450d27 explicitly stores source cell zero.
        let cell = m.u16(t.wrapping_add(0x7c))?;
        let direction = crate::frame_path::direction(0, cell, false);
        let next = neighbor(m, cell, direction)?;
        if next != 65535 {
            move_to(m, victim, next)?;
        }
    }
    Ok(())
}
/// Full InfligeDegat450874 selected mode4, including native target selection.
pub fn impact(
    m: &mut MemoriaJogo,
    cell: u16,
    attacker: u16,
    direction: u8,
    skip_building: bool,
) -> Result<()> {
    entered(
        0x450874,
        [
            u32::from(cell),
            4,
            u32::from(attacker),
            u32::from(direction),
        ],
    );
    if !skip_building && m.u16(m.u32(0x4e0f38)?.wrapping_add(2 * u32::from(cell)))? & 0x8000 != 0 {
        return crate::projectile_building_impact::impact(m, cell, 4, attacker, direction);
    }
    entered(
        0x4465df,
        [u32::from(cell), 65535, u32::from(skip_building), 0],
    );
    let mut victim = None;
    for u in 0..800 {
        let r = unit(m, u)?;
        if !matches!(m.u8(r.wrapping_add(2))?, 0 | 4)
            && m.u8(r.wrapping_add(0x72))? != 0
            && crate::frame_units::occupies(m, u as u16, cell, 1 + u8::from(skip_building))? != 0
        {
            victim = Some(u as u16);
            if m.u8(r.wrapping_add(0xab))? != 0 {
                break;
            }
        }
    }
    let Some(victim) = victim else {
        return Ok(());
    };
    let t = unit(m, u32::from(victim))?;
    let kind = m.u8(t.wrapping_add(1))?;
    react(m, victim)?;
    consume(m, victim, attacker, direction)?;
    if matches!(kind,0..=2|4..=10|18..=24) {
        spawn(m, [14, u32::from(cell), u32::from(cell), 0, 0, 0, 0, 0, 0])?;
    } else if (11..=17).contains(&kind) {
        entered(0x4704dc, [0x22, 0, 0, 0]);
        let random = crate::frame_core::call(m, "Random", &[0x22])?;
        spawn(
            m,
            [
                15 + (random & 1),
                u32::from(cell),
                u32::from(cell),
                0,
                0,
                0,
                0,
                0,
                u32::from(kind == 16),
            ],
        )?;
    }
    let s = crate::player::unit(m, u32::from(attacker))?;
    let (owner, target_owner) = (m.u8(s)?, m.u8(t)?);
    if crate::frame_economy::call(
        m,
        "PlayerEstAllie",
        &[u32::from(owner), u32::from(target_owner)],
    )? as u8
        != 0
    {
        crate::frame_economy::call(
            m,
            "MAJSympathie",
            &[u32::from(target_owner), u32::from(owner), 10],
        )?;
        crate::frame_economy::call(
            m,
            "MAJSympathie",
            &[u32::from(target_owner), u32::from(owner), 6],
        )?;
        reset_allie(m, owner, target_owner)?;
        reset_allie(m, target_owner, owner)?;
    }
    if owner < 4 && target_owner < 4 {
        let a = 0x508044 + 20 * u32::from(owner) + 4 * u32::from(target_owner);
        m.set_u32(a, m.u32(a)?.wrapping_add(1))?;
    }
    if owner != target_owner && target_owner == m.u8(0x508041)? {
        let now = m.u32(0x508f7c)?;
        let previous = m.u32(0x4e19c0)?;
        if previous > now || now.wrapping_sub(previous) >= 200 {
            m.set_u32(0x4e19c0, now)?;
            m.set_u16(0x496acc, m.u16(t.wrapping_add(0x7c))?)?;
            crate::player::sound(
                m,
                0x4510b1,
                true,
                [
                    0x50,
                    (u32::from(m.u8(t.wrapping_add(1))?) << 16) | 0x700,
                    0x496ac8,
                    0,
                ],
                Some(256),
            )?;
        }
    }
    Ok(())
}

fn spawn(m: &mut MemoriaJogo, a: [u32; 9]) -> Result<()> {
    let [kind, from, target, mode, attacker, direction, dx, dy, flag] = a;
    let (kind, from, target) = (kind as u8, from as u16, target as u16);
    let mut slot = 0u32;
    while slot < 100 {
        if m.u8(m.u32(0x5080f0)?.wrapping_add(slot * 56 + 2))? == 0 {
            break;
        }
        slot += 1;
    }
    if slot == 100 {
        return Ok(());
    }
    let q = m.u32(0x5080f0)?.wrapping_add(slot * 56);
    m.set_u8(q, kind)?;
    m.set_u16(q.wrapping_add(0x14), from)?;
    m.set_u16(q.wrapping_add(0x16), target)?;
    m.set_u8(q.wrapping_add(0x23), flag as u8)?;
    m.set_u16(
        q.wrapping_add(12),
        (from & 255).wrapping_mul(32).wrapping_add(dx as u16),
    )?;
    m.set_u8(q.wrapping_add(1), 1)?;
    m.set_u32(q.wrapping_add(0x24), 0)?;
    m.set_u16(q.wrapping_add(0x28), 0)?;
    m.set_u8(q.wrapping_add(2), 1)?;
    m.set_u8(q.wrapping_add(0x18), 0)?;
    m.set_u8(q.wrapping_add(3), 0)?;
    m.set_u16(q.wrapping_add(0x1a), 12)?;
    m.set_u16(
        q.wrapping_add(14),
        (from >> 8).wrapping_mul(32).wrapping_add(dy as u16),
    )?;
    m.set_u16(q.wrapping_add(0x10), dx as u16)?;
    m.set_u16(q.wrapping_add(0x12), dy as u16)?;
    m.set_u8(q.wrapping_add(0x1f), mode as u8)?;
    m.set_u16(q.wrapping_add(0x20), attacker as u16)?;
    m.set_u8(q.wrapping_add(0x22), direction as u8)?;
    m.set_u8(q.wrapping_add(0x19), 0)?;
    m.set_u16(q.wrapping_add(0x1c), 0)?;
    if matches!(kind, 15 | 16) && flag != 0 {
        m.set_u32(q.wrapping_add(0x24), 12800)?;
    }
    Ok(())
}
