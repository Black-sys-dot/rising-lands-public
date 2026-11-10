//! Intact InfligeDegat4508af mode1/3/4 building branch and BatimentInfligeDegat41a2c1.
//! No destruction tick, device, profile/UI or AI-city fallback.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

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
fn record(index: u16) -> u32 {
    0x49a0c8 + 276 * u32::from(index)
}
/// BatimentGetVolume41a0ab uses the stored origin, not BatimentGetCase.
pub fn volume(m: &MemoriaJogo, index: u16) -> Result<u32> {
    entered(0x41a0ab, [u32::from(index), 0, 0, 0]);
    let cell = m.u16(record(index) + 2)?;
    let x = (m.u16(0x508724)? / 2).wrapping_add(m.u16(0x508720)?);
    let y = (m.u16(0x508726)? / 2).wrapping_add(m.u16(0x508722)?);
    let dx = i32::from(x) - i32::from(cell & 255);
    let dy = i32::from(y) - i32::from(cell >> 8);
    let distance = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
    Ok(if distance <= 145 {
        256
    } else if distance <= 300 {
        180
    } else if distance < 600 {
        100
    } else {
        0
    })
}
/// Full native VoixOffAsk sound selection, using the existing exact unopened
/// sound provider. Occupied handles and device images remain addressed stops.
pub fn voice(m: &mut MemoriaJogo, index: u16, request: u16, flag: u8) -> Result<()> {
    entered(
        0x41a1ac,
        [u32::from(index), u32::from(request), u32::from(flag), 0],
    );
    let handle = record(index) + 0x100;
    crate::player::sound(
        m,
        0x41a1d3,
        false,
        [handle, u32::from(request), 0, u32::from(flag)],
        None,
    )?;
    let (bank, sound) = match request {
        0 => (0, 0),
        1 => (1, 3),
        2 => (2, 0),
        3 => (1, 0),
        4 => (1, 1),
        5 => (2, 1),
        6 => (2, 2),
        _ => (0, u32::from(request)),
    };
    crate::player::sound(
        m,
        0x41a221,
        true,
        [0x41, (bank << 16) | (sound << 8), handle, u32::from(flag)],
        Some(256),
    )?;
    Ok(())
}
fn city(m: &mut MemoriaJogo, index: u16, message: u8, source: u16) -> Result<()> {
    entered(
        0x484808,
        [u32::from(index), u32::from(message), u32::from(source), 0],
    );
    crate::chain_queries::entry(
        m,
        "iagVilleSendMessage",
        FrameRegisters {
            eax: u32::from(index),
            edx: u32::from(message),
            ebx: u32::from(source),
            ..Default::default()
        },
    )?;
    Ok(())
}
/// BatimentInfligeDegat41a2c1: AX=index, DX=damage, BL=source player.
/// This changes destruction state; it does not execute subsequent building ticks.
pub fn consume(m: &mut MemoriaJogo, index: u16, damage: u16, source: u8) -> Result<()> {
    entered(
        0x41a2c1,
        [u32::from(index), u32::from(damage), u32::from(source), 0],
    );
    let a = record(index);
    m.set_u16(a + 6, m.u16(a + 6)?.wrapping_add(damage))?;
    if m.u8(a + 4)? == m.u8(0x508041)? {
        let v = volume(m, index)?;
        if m.u32(a + 0x100)? == u32::MAX {
            crate::player::sound(
                m,
                0x41a331,
                true,
                [0x42, (u32::from(m.u8(a + 8)?) << 16) | 0x100, a + 0x100, 0],
                Some(v),
            )?;
        }
    }
    if m.u8(a + 4)? == m.u8(0x508041)? && m.u8(0x4dfb9b)? == 0 {
        // The original explicitly requests building slot1, not the victim.
        voice(m, 1, 3, 0)?;
        m.set_u8(0x4dfb9b, 1)?;
    }
    let base = u32::from(m.u16(0x495d90 + 52 * u32::from(m.u8(a + 8)?))?);
    if base == 0 {
        return stop("BatimentInfligeDegat native zero divisor", 0x41a3ce);
    }
    let damage_band = ((u32::from(m.u16(a + 6)?) * 200) / base) as u16;
    let construction_band = ((u32::from(m.u16(a + 10)?) * 200) / base) as u16;
    if construction_band >= damage_band {
        return Ok(());
    }
    city(m, index, 2, 0)?;
    m.set_u8(a + 0x90, 1)?;
    let kind = m.u8(a + 8)?;
    m.set_u8(a + 5, 3)?;
    if !matches!(kind, 9 | 12) {
        m.set_u8(0x4dfb44, 20)?;
    }
    if kind == 20 {
        m.set_u16(a + 0x84, 0x113)?;
    }
    crate::frame_economy::call(
        m,
        "MAJSympathie",
        &[u32::from(source), u32::from(m.u8(a + 4)?), 8],
    )?;
    let local = m.u8(0x508041)?;
    if local == m.u8(a + 4)? {
        voice(m, 1, 1, 0)?;
    } else if m.u8(a + 0x110 + u32::from(local))? != 0 {
        voice(m, 1, 4, 0)?;
    }
    Ok(())
}
/// TypeBatimentGetDegat 0x418b8a: AL = structure type, DL = mode, BL = the
/// target's unit kind. Only turret types 10 (table 0x4dfb80) and 11 (table
/// 0x4dfb64) deal damage: mode 1 reads the kind's byte, mode 3 the building
/// column (+26); every other type or mode returns 0.
pub fn structure_damage(m: &MemoriaJogo, kind: u8, mode: u8, target: u8) -> Result<u8> {
    let table = match kind {
        10 => 0x4dfb80,
        11 => 0x4dfb64,
        _ => return Ok(0),
    };
    match mode {
        1 => m.u8(table + u32::from(target)),
        3 => m.u8(table + 26),
        _ => Ok(0),
    }
}
fn reset_allie(m: &mut MemoriaJogo, owner: u8, other: u8) -> Result<()> {
    entered(0x46d8cb, [u32::from(owner), u32::from(other), 0, 0]);
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
fn attacked(m: &mut MemoriaJogo, owner: u8, other: u8) -> Result<()> {
    entered(0x46d600, [u32::from(owner), u32::from(other), 0, 0]);
    if owner < 4 && other < 4 {
        let a = 0x508044 + 20 * u32::from(owner) + 4 * u32::from(other);
        m.set_u32(a, m.u32(a)?.wrapping_add(1))?;
    }
    Ok(())
}
/// Selected internal4508af continuation. Caller already selected this actual
/// tagged building. Mode1 uses the native source attack/research row, mode3 the
/// shooter structure's TypeBatimentGetDegat column; mode4 accepts directions
/// 29/30. Other building damage modes are outside this API.
pub fn impact(
    m: &mut MemoriaJogo,
    cell: u16,
    mode: u8,
    attacker: u16,
    direction: u8,
) -> Result<()> {
    let index = (m.u16(m.u32(0x4e0f38)?.wrapping_add(2 * u32::from(cell)))? & 0x7fe0) >> 5;
    entered(
        0x4508af,
        [
            u32::from(cell),
            u32::from(index),
            u32::from(attacker),
            u32::from(direction),
        ],
    );
    let unit = crate::player::unit(m, u32::from(attacker))?;
    let mut source = m.u8(unit)?;
    let damage = match mode {
        3 => {
            // 0x45092a: a structure shooter (turret types 10/11). The damage
            // is TypeBatimentGetDegat(shooter type, 3, target type) and the
            // source player is the shooter's owner; no city message.
            let target_type = m.u8(record(index) + 8)?;
            let shooter_type = m.u8(record(attacker) + 8)?;
            source = m.u8(record(attacker) + 4)?;
            u16::from(structure_damage(m, shooter_type, 3, target_type)?)
        }
        1 => {
            let kind = u32::from(m.u8(unit + 1)?);
            let row = m.u32(m.u32(0x4e199c)?.wrapping_add(4 * kind))?;
            let base = u32::from(m.u8(row.wrapping_add(26))?);
            let bonus = crate::frame_economy::bonus(m, u32::from(source), kind, 5, base)?;
            let damage = base.wrapping_add(bonus) as u16;
            city(m, index, 4, attacker)?;
            damage
        }
        4 => match direction {
            29 => m.u16(0x49716c)? / 2,
            30 => m.u16(0x49716e)? / 2,
            _ => return stop("Building impact outside selected direction", 0x4509b2),
        },
        _ => return stop("Building impact outside selected mode", 0x4508af),
    };
    consume(m, index, damage, source)?;
    entered(0x4704dc, [0x21, 0, 0, 0]);
    let random = crate::frame_core::call(m, "Random", &[0x21])?;
    spawn(m, cell, 15 + (random & 1) as u8)?;
    let owner = m.u8(record(index) + 4)?;
    if crate::frame_economy::call(m, "PlayerEstAllie", &[u32::from(source), u32::from(owner)])?
        as u8
        != 0
    {
        crate::frame_economy::call(
            m,
            "MAJSympathie",
            &[u32::from(owner), u32::from(source), 10],
        )?;
        crate::frame_economy::call(m, "MAJSympathie", &[u32::from(owner), u32::from(source), 6])?;
        reset_allie(m, source, owner)?;
        reset_allie(m, owner, source)?;
    }
    attacked(m, source, owner)
}
/// Native ProjectileInit/Set publication order for ground impact15/16.
fn spawn(m: &mut MemoriaJogo, cell: u16, kind: u8) -> Result<()> {
    entered(
        0x46e44c,
        [u32::from(kind), u32::from(cell), u32::from(cell), 0],
    );
    let pool = m.u32(0x5080f0)?;
    let mut slot = 0;
    while slot < 100 && m.u8(pool.wrapping_add(slot * 56 + 2))? != 0 {
        slot += 1;
    }
    if slot == 100 {
        return Ok(());
    }
    let q = pool.wrapping_add(slot * 56);
    m.set_u8(q, kind)?;
    m.set_u16(q + 0x14, cell)?;
    m.set_u16(q + 0x16, cell)?;
    m.set_u8(q + 0x23, 0)?;
    m.set_u16(q + 12, (cell & 255).wrapping_mul(32))?;
    m.set_u8(q + 1, 1)?;
    m.set_u32(q + 0x24, 0)?;
    m.set_u16(q + 0x28, 0)?;
    m.set_u8(q + 2, 1)?;
    m.set_u8(q + 0x18, 0)?;
    m.set_u8(q + 3, 0)?;
    m.set_u16(q + 0x1a, 12)?;
    m.set_u16(q + 14, (cell >> 8).wrapping_mul(32))?;
    m.set_u16(q + 0x10, 0)?;
    m.set_u16(q + 0x12, 0)?;
    m.set_u8(q + 0x1f, 0)?;
    m.set_u16(q + 0x20, 0)?;
    m.set_u8(q + 0x22, 0)?;
    m.set_u8(q + 0x19, 0)?;
    m.set_u16(q + 0x1c, 0)
}
