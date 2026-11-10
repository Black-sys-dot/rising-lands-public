//! Selected intact ProjectileAvance46eaf8 bodies for VISÃO28 and NOVA29.
//! No pool filtering, phase clock, renderer admission or replacement callbacks.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

#[cfg(feature = "validation-trace")]
std::thread_local! {
    static CALLS: std::cell::RefCell<Vec<[u32; 5]>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(feature = "validation-trace")]
pub fn take_validation_calls() -> Vec<[u32; 5]> {
    CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}
fn entered(address: u32, args: [u32; 4]) {
    #[cfg(feature = "validation-trace")]
    CALLS.with(|calls| {
        calls
            .borrow_mut()
            .push([address, args[0], args[1], args[2], args[3]])
    });
    #[cfg(not(feature = "validation-trace"))]
    let _ = (address, args);
}
fn query(m: &mut MemoriaJogo, address: u32) -> Result<u16> {
    entered(address, [0; 4]);
    Ok(crate::spell_queries::entry(m, address, FrameRegisters::default())?.eax as u16)
}
/// Exact selected ProjectileSet publication order for the indispensable
/// stationary explosion kinds6..8. Pool bytes not written by x86 stay intact.
fn explosion(m: &mut MemoriaJogo, kind: u8, from: u16, p: u32) -> Result<()> {
    let target = m.u16(p.wrapping_add(0x16))?;
    let mode = m.u8(p.wrapping_add(0x1f))?;
    let attacker = m.u16(p.wrapping_add(0x20))?;
    let direction = m.u8(p.wrapping_add(0x22))?;
    let flag = m.u8(p.wrapping_add(0x23))?;
    spawn(
        m,
        [
            u32::from(kind),
            u32::from(from),
            u32::from(target),
            u32::from(mode),
            u32::from(attacker),
            u32::from(direction),
            0,
            0,
            u32::from(flag),
        ],
    )
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
    entered(
        0x46e208,
        [slot, u32::from(kind), u32::from(from), u32::from(target)],
    );
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
    } else if kind == 30 {
        m.set_u8(q.wrapping_add(0x19), 0)?;
        crate::frame_combat::call(m, "ProjectileLineaire", &[slot])?;
        m.set_u16(q.wrapping_add(0x1a), 20)?;
    }
    Ok(())
}
/// ProjectileCalculLineaire46e4ca, pointer ABI; all word stores wrap.
pub fn linear(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let remaining = m.u16(p.wrapping_add(0x34))?;
    if remaining != 0 {
        let (step, correction) = if (m.u16(p.wrapping_add(0x2e))? as i16) < 0 {
            (0x2c, 0x32)
        } else {
            (0x2a, 0x30)
        };
        let position = m
            .u32(p.wrapping_add(8))?
            .wrapping_add(m.u16(p.wrapping_add(step))? as i16 as u32);
        m.set_u32(p.wrapping_add(8), position)?;
        m.set_u16(
            p.wrapping_add(0x2e),
            m.u16(p.wrapping_add(0x2e))?
                .wrapping_add(m.u16(p.wrapping_add(correction))?),
        )?;
        m.set_u16(p.wrapping_add(12), (position & 8191) as u16)?;
        m.set_u16(p.wrapping_add(0x34), remaining.wrapping_sub(1))?;
        m.set_u16(p.wrapping_add(14), (position >> 13) as u16)?;
    }
    Ok(())
}
fn player_type(m: &mut MemoriaJogo, p: u8) -> Result<u8> {
    Ok(crate::frame_economy::call(m, "PlayerGetType", &[u32::from(p)])? as u8)
}
fn move_to(m: &mut MemoriaJogo, u: u16, c: u16) -> Result<()> {
    crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(u), u32::from(c)])?;
    Ok(())
}
fn react(m: &mut MemoriaJogo, victim: u16, attacker: u16) -> Result<()> {
    let t = crate::player::unit(m, u32::from(victim))?;
    let s = crate::player::unit(m, u32::from(attacker))?;
    let owner = m.u8(t)?;
    if owner == 200 {
        return Ok(());
    }
    let kind = m.u8(t.wrapping_add(1))?;
    if player_type(m, owner)? == 1 && matches!(m.u8(t.wrapping_add(2))?, 5 | 18) {
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
            let cell = crate::frame_core::building_cell(m, u32::from(building))? as u16;
            move_to(m, victim, cell)?;
        }
        return Ok(());
    }
    let source_kind = m.u8(s.wrapping_add(1))?;
    let eligible = matches!(kind,0..=3|8|11|12|15|16|18..=25)
        && m.u8(t.wrapping_add(0xdc))? & 2 == 0
        && !matches!(source_kind, 10 | 17)
        && (source_kind != 16 || matches!(kind, 2 | 8))
        && m.u8(s)? != owner;
    if eligible {
        let pt = player_type(m, owner)?;
        let attack = pt == 1
            && (m.u8(t.wrapping_add(2))? == 7
                || m.u32(0x508f7c)?.wrapping_sub(m.u32(t.wrapping_add(0x110))?) > 250)
            || pt == 2 && m.u8(t.wrapping_add(2))? == 7;
        if attack {
            crate::player_orders::call(
                m,
                "SetAttaqueEngin",
                [u32::from(victim), 1, u32::from(attacker), 0],
                0x450cab,
            )?;
        }
    } else {
        let cell = m.u16(t.wrapping_add(0x7c))?;
        let direction = crate::frame_path::direction(m.u16(s.wrapping_add(0x7c))?, cell, false);
        let next = crate::frame_path::neighbor(m, cell, direction)?;
        if next != 65535 {
            move_to(m, victim, next)?;
        }
    }
    Ok(())
}

fn ordinary_impact(
    m: &mut MemoriaJogo,
    cell: u16,
    attacker: u16,
    skip_building: bool,
) -> Result<()> {
    let grid = m.u32(0x4e0f38)?;
    if !skip_building && m.u16(grid.wrapping_add(2 * u32::from(cell)))? & 0x8000 != 0 {
        return crate::projectile_building_impact::impact(m, cell, 1, attacker, 0);
    }
    let mut victim = None;
    for u in 0..800 {
        let r = crate::player::unit(m, u)?;
        if !matches!(m.u8(r.wrapping_add(2))?, 0 | 4)
            && m.u8(r.wrapping_add(0x72))? != 0
            && crate::frame_units::occupies(m, u as u16, cell, 1)? != 0
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
    let t = crate::player::unit(m, u32::from(victim))?;
    let kind = m.u8(t.wrapping_add(1))?;
    react(m, victim, attacker)?;
    crate::chain_damage::consume(m, victim, attacker)?;
    if matches!(kind,0..=2|4..=10|18..=24) {
        spawn(m, [14, u32::from(cell), u32::from(cell), 0, 0, 0, 0, 0, 0])?;
    } else if (11..=17).contains(&kind) {
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
        crate::message_box::reset_allie(m, owner, target_owner)?;
        crate::message_box::reset_allie(m, target_owner, owner)?;
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

fn damage(
    m: &mut MemoriaJogo,
    cell: u16,
    mode: u8,
    attacker: u16,
    kind: u8,
    flag: bool,
) -> Result<()> {
    entered(
        0x450874,
        [
            u32::from(cell),
            u32::from(mode),
            u32::from(attacker),
            u32::from(kind),
        ],
    );
    if mode == 1 {
        return ordinary_impact(m, cell, attacker, flag);
    }
    // The native mode4 selector is shared with mode1, but its occupied-cell
    // consumer is a different body. Never substitute ordinary attack damage.
    if mode != 4 {
        return stop("Projectile impact unported damage mode", 0x450874);
    }
    crate::projectile_occupied_impact::impact(m, cell, attacker, kind, flag)
}
/// Selected ProjectileInfligeDegat46e5fe: central stored mode, then the
/// original ordered mode4 neighbours. Bounds are the original256-square grid.
pub fn impact(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    entered(0x46e5fe, [p, 0, 0, 0]);
    damage(
        m,
        m.u16(p.wrapping_add(0x16))?,
        m.u8(p.wrapping_add(0x1f))?,
        m.u16(p.wrapping_add(0x20))?,
        m.u8(p.wrapping_add(0x22))?,
        m.u8(p.wrapping_add(0x23))? != 0,
    )?;
    if m.u8(p)? != 29 {
        return stop("ProjectileInfligeDegat selected NOVA body", 0x46e625);
    }
    for (dx, dy) in [
        (-1, 0),
        (1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        // x86 reloads target/attacker for every call; callbacks can alias them.
        let cell = m.u16(p.wrapping_add(0x16))?;
        let (x, y) = (i32::from(cell & 255), i32::from(cell >> 8));
        if (0..256).contains(&(x + dx)) && (0..256).contains(&(y + dy)) {
            damage(
                m,
                (i32::from(cell) + dx + 256 * dy) as u16,
                4,
                m.u16(p.wrapping_add(0x20))?,
                m.u8(p)?,
                false,
            )?;
        }
    }
    Ok(())
}
fn extinguish(m: &mut MemoriaJogo, center: u16, radius: u16, owner: u8) -> Result<()> {
    entered(
        0x476e82,
        [u32::from(center), u32::from(radius), u32::from(owner), 0],
    );
    if owner >= 4 {
        return stop(
            "EteintTerrain owner outside declared player domain",
            0x476e82,
        );
    }
    crate::player_sight::area(m, center, u32::from(radius), owner, 2)?;
    // 476f8c..477040 repeats the clipped circle and reads the light grid.
    // It makes no stores/calls, but its declared reads/faults remain real.
    let (x, y, rr) = (
        i32::from(center & 255),
        i32::from(center >> 8),
        i32::from(radius),
    );
    let limit = (u32::from(radius) * u32::from(radius) + u32::from(radius >= 2)) as u16;
    for dy in -rr..=rr {
        if y + dy < 0 || y + dy >= i32::from(m.u16(0x49716a)?) {
            continue;
        }
        for dx in -rr..=rr {
            if x + dx >= 0
                && x + dx < i32::from(m.u16(0x497168)?)
                && ((dx * dx + dy * dy) as u16) <= limit
            {
                let cell = (i32::from(center) + dx + 256 * dy) as u16;
                let _ = m.u8(m
                    .u32(0x507d88 + 4 * u32::from(owner))?
                    .wrapping_add(u32::from(cell)))?;
            }
        }
    }
    Ok(())
}
/// EAX points at a projectile, no active/newborn gate. Clocking belongs to
/// ProjectileCalcul46fc0f. Errors retain the actual native mutation prefix.
pub fn advance(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    match m.u8(p)? {
        28 => {
            let phase = u16::from(m.u8(p.wrapping_add(0x18))?);
            if phase == query(m, crate::spell_queries::GET_TEMPS_FIN)? {
                let owner = m.u8(p.wrapping_add(0x20))?;
                let radius = query(m, crate::spell_queries::GET_RAYON_EFFET)?;
                extinguish(m, m.u16(p.wrapping_add(0x16))?, radius, owner)?;
                m.set_u8(p.wrapping_add(2), 0)?;
            }
        }
        29 | 30 => {
            // 0x46ef58: kinds 29 and 30 share the phase gate, the follow-up
            // projectile of kind + 1 and the linear motion; arrival differs.
            if m.u8(p.wrapping_add(0x18))? < 12 {
                return Ok(());
            }
            let count = m.u8(p.wrapping_add(3))?.wrapping_add(1);
            m.set_u8(p.wrapping_add(3), count)?;
            if m.u8(p.wrapping_add(0x19))? != 0 && count == m.u8(p.wrapping_add(4))? {
                spawn(
                    m,
                    [
                        u32::from(m.u8(p)?.wrapping_add(1)),
                        u32::from(m.u16(p.wrapping_add(0x14))?),
                        u32::from(m.u16(p.wrapping_add(0x16))?),
                        u32::from(m.u8(p.wrapping_add(0x1f))?),
                        u32::from(m.u16(p.wrapping_add(0x20))?),
                        u32::from(m.u8(p.wrapping_add(0x22))?),
                        m.u16(p.wrapping_add(0x10))? as i16 as u32,
                        m.u16(p.wrapping_add(0x12))? as i16 as u32,
                        u32::from(m.u8(p.wrapping_add(0x23))?),
                    ],
                )?;
                m.set_u8(p.wrapping_add(0x19), 0)?;
            }
            if m.u16(p.wrapping_add(0x34))? != 0 {
                entered(0x46e4ca, [p, 0, 0, 0]);
                linear(m, p)?;
                if m.u8(p.wrapping_add(0x23))? != 0 {
                    m.set_u32(
                        p.wrapping_add(0x24),
                        m.u32(p.wrapping_add(0x24))?
                            .wrapping_add(u32::from(m.u16(p.wrapping_add(0x28))?)),
                    )?;
                }
            } else if m.u8(p)? == 30 {
                nova_sweep(m, p)?;
                m.set_u8(p.wrapping_add(2), 0)?;
            } else {
                impact(m, p)?;
                let cell = m.u16(p.wrapping_add(0x16))?;
                let (x, y) = (cell & 255, cell >> 8);
                let width = if x == 0 || x == 255 { 2 } else { 3 };
                let height = if y == 0 || y == 255 { 2 } else { 3 };
                let mut from = cell
                    .wrapping_sub(u16::from(x != 0))
                    .wrapping_sub(if y != 0 { 256 } else { 0 });
                for _ in 0..height {
                    for _ in 0..width {
                        entered(0x4704dc, [0x2f, 0, 0, 0]);
                        let kind = 6 + (crate::frame_core::random(m)? % 3) as u8;
                        explosion(m, kind, from, p)?;
                        from = from.wrapping_add(1);
                    }
                    // Native uses256-height (not256-width); on clipped side
                    // edges this deliberately drifts between rows.
                    from = from.wrapping_add(256 - height);
                }
                m.set_u8(p.wrapping_add(2), 0)?;
            }
        }
        _ => return stop("ProjectileAvance selected spell type", 0x46eaf8),
    }
    Ok(())
}

/// Kind 30 arrival, 0x46f088 (BatimentLanceNova's projectile). Sets 0x4dfb44
/// to 15, then walks the 15 x 15 window around the target, clipped at the map
/// edge (the row stride is 0x100 minus the column count; rows and columns at
/// or past 0x100 are skipped without moving the row start back). Every cell
/// with squared distance (0x479555, unsigned word differences) up to 49 takes
/// InfligeDegat mode 4 from the projectile's attacker with type 30 and a zero
/// stack flag, then Random(0x2f) % 3 picks a debris projectile of kind 6, 7
/// or 8 from that cell to the target in the first free slot.
fn nova_sweep(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    m.set_u8(0x4dfb44, 15)?;
    let target = u32::from(m.u16(p.wrapping_add(0x16))?);
    let (x, y) = (target % 256, target / 256);
    let (x0, cols) = if x > 7 {
        (x - 7, 15)
    } else {
        (0, 15 - (7 - x))
    };
    let (y0, rows) = if y > 7 {
        (y - 7, 15)
    } else {
        (0, 15 - (7 - y))
    };
    let stride = 0x100u32.wrapping_sub(cols);
    let mut cell = (y0 << 8).wrapping_add(x0);
    let mut row = y0;
    loop {
        if row & 65535 >= (y0 + rows) & 65535 {
            break;
        }
        if row & 65535 < 0x100 {
            let mut col = x0;
            while col & 65535 < (x0 + cols) & 65535 {
                if col & 65535 < 0x100 {
                    let dx = (x as i32 - (col & 65535) as i32).unsigned_abs();
                    let dy = (y as i32 - (row & 65535) as i32).unsigned_abs();
                    if (dx * dx + dy * dy) & 65535 <= 0x31 {
                        let attacker = m.u16(p.wrapping_add(0x20))?;
                        let kind = m.u8(p)?;
                        damage(m, cell as u16, 4, attacker, kind, false)?;
                        entered(0x4704dc, [0x2f, 0, 0, 0]);
                        let debris = 6 + (crate::frame_core::random(m)? % 3) as u8;
                        explosion(m, debris, cell as u16, p)?;
                    }
                }
                col = col.wrapping_add(1);
                cell = cell.wrapping_add(1);
            }
        }
        row = row.wrapping_add(1);
        cell = cell.wrapping_add(stride);
    }
    Ok(())
}
