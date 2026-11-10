//! Projectile creation and linear preparation from Combat's proved pool model.
//! Unknown world callbacks stop at their original address before publication.
use crate::{frame::stop, MemoriaJogo, Result};
fn record(m: &MemoriaJogo, index: u32) -> Result<u32> {
    if index >= 100 {
        return stop("ProjectileSet slot outside pool", 0x46e208);
    }
    Ok(m.u32(0x5080f0)?.wrapping_add(index.wrapping_mul(56)))
}
fn line(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let x = m.u16(p + 12)?;
    let y = m.u16(p + 14)?;
    let target = m.u16(p + 0x16)?;
    let dx = (target & 255)
        .wrapping_mul(32)
        .wrapping_add(16)
        .wrapping_sub(x) as i16;
    let dy = (target >> 8)
        .wrapping_mul(32)
        .wrapping_add(16)
        .wrapping_sub(y) as i16;
    let sx = if dx < 0 { -1i16 } else { 1 };
    let sy = if dy < 0 { -8192i16 } else { 8192 };
    let ax = dx.wrapping_abs() as u16;
    let ay = dy.wrapping_abs() as u16;
    let (major, minor, axial) = if (ax as i16) > (ay as i16) {
        (ax, ay, sx)
    } else {
        (ay, ax, sy)
    };
    m.set_u16(p + 0x34, major)?;
    m.set_u16(p + 0x2c, axial as u16)?;
    m.set_u16(p + 0x32, minor.wrapping_mul(2))?;
    m.set_u16(p + 0x2a, sx.wrapping_add(sy) as u16)?;
    m.set_u16(
        p + 0x30,
        minor.wrapping_mul(2).wrapping_sub(major.wrapping_mul(2)),
    )?;
    m.set_u32(
        p + 8,
        (i32::from(y as i16).wrapping_shl(13)).wrapping_add(i32::from(x as i16)) as u32,
    )?;
    m.set_u16(p + 0x2e, minor.wrapping_mul(2).wrapping_sub(major))?;
    if m.u8(p + 0x23)? != 0 {
        let divisor = i32::from(major as i16);
        let step = if divisor == 0 { 12800 } else { 12800 / divisor };
        m.set_u16(p + 0x28, step as u16)?;
        m.set_u32(p + 0x24, 0)?;
    }
    Ok(())
}
fn set(m: &mut MemoriaJogo, a: &[u32]) -> Result<u32> {
    let p = record(m, a[0])?;
    let kind = a[1] as u8;
    let from = a[2] as u16;
    let to = a[3] as u16;
    let mode = a[4] as u8;
    let attacker = a[5] as u16;
    let direction = a[6] as u8;
    let dx = a[7] as u16;
    let dy = a[8] as u16;
    let flag = a[9] as u8;
    // ProjectileSet46e22a..46e2cb: retain the native publication order.
    m.set_u8(p, kind)?;
    m.set_u16(p + 0x14, from)?;
    m.set_u16(p + 0x16, to)?;
    m.set_u8(p + 0x23, flag)?;
    m.set_u16(p + 12, (from & 255).wrapping_mul(32).wrapping_add(dx))?;
    m.set_u8(p + 1, 1)?;
    m.set_u32(p + 0x24, 0)?;
    m.set_u16(p + 0x28, 0)?;
    m.set_u8(p + 2, 1)?;
    m.set_u8(p + 0x18, 0)?;
    m.set_u8(p + 3, 0)?;
    m.set_u16(p + 0x1a, 12)?;
    m.set_u16(p + 14, (from >> 8).wrapping_mul(32).wrapping_add(dy))?;
    m.set_u16(p + 0x10, dx)?;
    m.set_u16(p + 0x12, dy)?;
    m.set_u8(p + 0x1f, mode)?;
    m.set_u16(p + 0x20, attacker)?;
    m.set_u8(p + 0x22, direction)?;
    m.set_u8(p + 0x19, 0)?;
    m.set_u16(p + 0x1c, 0)?;
    match kind {
        1..=5 | 9..=13 | 17 | 20..=23 | 29 | 30 => {
            if matches!(kind, 1 | 9 | 17 | 20..=23) {
                m.set_u8(p + 1, 0)?;
            }
            let count = match kind {
                1 | 17 | 20..=23 => Some(15),
                2 => Some(10),
                3 => Some(8),
                4 => Some(4),
                9..=13 => Some(2),
                _ => None,
            };
            if let Some(count) = count {
                m.set_u8(p + 4, count)?;
            }
            if (9..=13).contains(&kind) {
                m.set_u16(p + 0x1a, 5)?;
            }
            m.set_u8(p + 0x19, u8::from(matches!(kind,1..=4|9..=12)))?;
            line(m, p)?;
            if kind == 9 {
                m.set_u16(p + 0x1c, 5)?;
            } else if matches!(kind, 1 | 17 | 20..=23) {
                m.set_u16(p + 0x1c, 10)?;
            }
            if matches!(kind, 20..=23 | 29 | 30) {
                m.set_u16(p + 0x1a, 20)?;
            }
        }
        15 | 16 if flag != 0 => m.set_u32(p + 0x24, 12800)?,
        // 0x46e3fb: kind 26 strikes at once, InfligeDegat on the target cell.
        26 => hit(m, p, m.u16(p + 0x16)?, false)?,
        28 => return stop("ProjectileSet SortilegeGetRayonEffet", 0x474734),
        _ => {} // The original leaves every other byte untouched for these types.
    }
    Ok(a[0]) // Semantic slot, not a claim about ProjectileSet's full EAX.
}
/// ProjectileInfligeDegat0x46e5fe central impact for the non-area types:
/// InfligeDegat(target, mode, attacker, direction, flag) with modes 1 and 3.
fn impact(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    hit(m, p, m.u16(p + 0x16)?, m.u8(p + 0x23)? != 0)
}
fn hit(m: &mut MemoriaJogo, p: u32, cell: u16, flag: bool) -> Result<()> {
    let mode = m.u8(p + 0x1f)?;
    if !matches!(mode, 1 | 3) {
        return stop("ProjectileInfligeDegat damage mode", 0x450874);
    }
    crate::chain_damage::impact_mode(m, cell, mode, m.u16(p + 0x20)?, flag)
}
/// Arrival of the area projectiles 20/22 (0x46f4de). ProjectileInfligeDegat
/// 0x46e5fe hits the target cell with the stored flag and, for these kinds
/// (0x46e897), its eight neighbours inside the map with flag 0, in the order
/// W, E, NW, N, NE, SW, S, SE. Then 0x46f4e5 walks the box around the target
/// (clamped at the map border) and, per cell, draws Random % 3 to publish a
/// debris projectile of kind 6, 7 or 8 from that cell to the target in the
/// first free slot (none when the pool is full). The native row stride is
/// 256 minus the ROW count, not the column count: on the left/right border
/// rows (3 rows, 2 columns) the walk shifts by one cell per row, as the
/// original does.
fn area_arrival(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    impact(m, p)?;
    let target = m.u16(p + 0x16)?;
    let (x, y) = (target & 255, target >> 8);
    let neighbours = [
        (x != 0, target.wrapping_sub(1)),
        (x != 255, target.wrapping_add(1)),
        (y != 0 && x != 0, target.wrapping_sub(0x101)),
        (y != 0, target.wrapping_sub(0x100)),
        (y != 0 && x != 255, target.wrapping_sub(0xff)),
        (y != 255 && x != 0, target.wrapping_add(0xff)),
        (y != 255, target.wrapping_add(0x100)),
        (y != 255 && x != 255, target.wrapping_add(0x101)),
    ];
    for (inside, cell) in neighbours {
        if inside {
            hit(m, p, cell, false)?;
        }
    }
    let target = m.u16(p + 0x16)?;
    let (x, y) = (u32::from(target & 255), u32::from(target >> 8));
    let mut columns = 3u32;
    let mut rows = 3u32;
    let x0 = if x != 0 {
        x - 1
    } else {
        columns = 2;
        0
    };
    let y0 = if y != 0 {
        y - 1
    } else {
        rows -= 1;
        0
    };
    if x == 255 {
        columns -= 1;
    }
    if y == 255 {
        rows -= 1;
    }
    let mut cell = x0 + (y0 << 8);
    let stride = 256 - rows;
    for _ in 0..rows {
        for _ in 0..columns {
            let kind = 6 + crate::frame_core::random(m)? % 3;
            let pool = m.u32(0x5080f0)?;
            let mut slot = 0;
            while slot < 100 && m.u8(pool.wrapping_add(slot * 56 + 2))? != 0 {
                slot += 1;
            }
            if slot != 100 {
                set(
                    m,
                    &[
                        slot,
                        kind,
                        cell & 0xffff,
                        u32::from(m.u16(p + 0x16)?),
                        u32::from(m.u8(p + 0x1f)?),
                        u32::from(m.u16(p + 0x20)?),
                        u32::from(m.u8(p + 0x22)?),
                        0,
                        0,
                        u32::from(m.u8(p + 0x23)?),
                    ],
                )?;
            }
            cell = cell.wrapping_add(1);
        }
        cell = cell.wrapping_add(stride);
    }
    Ok(())
}
/// ProjectileAvance0x46eaf8. No activity/newborn filtering here: the caller
/// performs that before each of the original thirty live-pool scans.
pub fn advance(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let kind = m.u8(p)?;
    let phase = m.u8(p + 0x18)?;
    match kind {
        0 | 35..=255 => Ok(()),
        // 0x46fb9e routes kinds 18/19 and 0x46eb3c kind 34 to the same
        // 0x46fbb6 stationary-explosion body as kinds 6..=8.
        6..=8 | 18 | 19 | 34 => {
            if phase == 32 {
                // 46fbc0 clears the projectile before drawing Random and
                // calling CreerExplosion, unlike type25's impact branch.
                m.set_u8(p + 2, 0)?;
                let effect = 56 + crate::frame_core::random(m)? % 6;
                let cell = u32::from(m.u16(p + 0x16)?);
                let objects = m.u32(0x507d74)?;
                if m.u8(objects + cell)? == 0 {
                    m.set_u8(objects + cell, effect as u8)?;
                    m.set_u8(m.u32(0x507dac)? + cell, 0)?;
                }
            }
            Ok(())
        }
        14..=16 | 26 | 27 | 31 | 32 => {
            let limit = if kind <= 16 {
                16
            } else if kind <= 27 {
                8
            } else {
                32
            };
            if phase == limit {
                m.set_u8(p + 2, 0)?;
            }
            Ok(())
        }
        28..=30 => crate::spell_projectile_lifecycle::advance(m, p),
        24 => {
            // 46fb83 ->46eee7 ->46eea3: damage once at phase32, then
            // 46ef45 clears activity. Unlike type25, no explosion/RNG follows.
            // ProjectileInfligeDegat24 returns directly at46ead6 after its
            // ordinary stored-mode hit; it has no area-neighbour traversal.
            if phase == 32 {
                impact(m, p)?;
                m.set_u8(p + 2, 0)?;
            }
            Ok(())
        }
        25 => {
            if phase == 32 {
                impact(m, p)?;
                let effect = 56 + crate::frame_core::random(m)? % 6;
                // CreerExplosion0x475710 only occupies an empty object cell.
                let cell = u32::from(m.u16(p + 0x16)?);
                let objects = m.u32(0x507d74)?;
                if m.u8(objects + cell)? == 0 {
                    m.set_u8(objects + cell, effect as u8)?;
                    m.set_u8(m.u32(0x507dac)? + cell, 0)?;
                }
                m.set_u8(p + 2, 0)?;
            }
            Ok(())
        }
        // Kind 33 (0x46eb47): at phase 20 the shot scatters. Random(0x29)
        // odd keeps the target cell; even draws a row (Random(0x2a) % rows)
        // and a column (Random(0x2b) % columns) of the 3x3 box around the
        // target clamped to the map. Random(0x2c) % 3 then publishes a debris
        // projectile of kind 6, 7 or 8 from that cell to the target in the
        // first free slot (none when the pool is full), the shot is
        // retargeted to the cell (0x46ee9f), hits it centrally and clears.
        33 => {
            if phase != 20 {
                return Ok(());
            }
            let target = m.u16(p + 0x16)?;
            let mut cell = target;
            if crate::frame_core::random(m)? & 1 == 0 {
                let (x, y) = (u32::from(target & 255), u32::from(target >> 8));
                let mut columns = 3u32;
                let mut rows = 3u32;
                let x0 = if x != 0 {
                    x - 1
                } else {
                    columns = 2;
                    0
                };
                let y0 = if y != 0 {
                    y - 1
                } else {
                    rows -= 1;
                    0
                };
                if x == 255 {
                    columns -= 1;
                }
                if y == 255 {
                    rows -= 1;
                }
                let row = crate::frame_core::random(m)? % rows;
                let column = crate::frame_core::random(m)? % columns;
                cell = (((y0 + row) << 8) + x0 + column) as u16;
            }
            let kind = 6 + crate::frame_core::random(m)? % 3;
            call(
                m,
                "ProjectileInit",
                &[
                    kind,
                    u32::from(cell),
                    u32::from(target),
                    u32::from(m.u8(p + 0x1f)?),
                    u32::from(m.u16(p + 0x20)?),
                    u32::from(m.u8(p + 0x22)?),
                    0,
                    0,
                    u32::from(m.u8(p + 0x23)?),
                ],
            )?;
            m.set_u16(p + 0x16, cell)?;
            impact(m, p)?;
            m.set_u8(p + 2, 0)
        }
        // Native23 uses the same46ef66 counter/child/linear path, then
        // central impact46f07c (no area tail) before clearing activity.
        1..=5 | 9..=13 | 17 | 20..=23 => {
            // 0x46ef66 (kinds 1..=5 the unit shots and their trail children,
            // 9..=13, 17 and 20..=23 the turret shots) shares the trail
            // spawn, the linear step and the arrival dispatch 0x46fb1a:
            // kind 1 (0x46fad8) inflicts damage then publishes one debris
            // projectile of kind 6 + Random(0x30) % 3 on the target cell,
            // kinds 9, 17, 21 and 23 inflict damage, 20 and 22 take the
            // 0x46f4de area path, the others only free the slot.
            let count = m.u8(p + 3)?.wrapping_add(1);
            m.set_u8(p + 3, count)?;
            if m.u8(p + 0x19)? != 0 && count == m.u8(p + 4)? {
                call(
                    m,
                    "ProjectileInit",
                    &[
                        u32::from(kind + 1),
                        u32::from(m.u16(p + 0x14)?),
                        u32::from(m.u16(p + 0x16)?),
                        u32::from(m.u8(p + 0x1f)?),
                        u32::from(m.u16(p + 0x20)?),
                        u32::from(m.u8(p + 0x22)?),
                        m.u16(p + 0x10)? as i16 as u32,
                        m.u16(p + 0x12)? as i16 as u32,
                        u32::from(m.u8(p + 0x23)?),
                    ],
                )?;
                // Also cleared when no free slot exists.
                m.set_u8(p + 0x19, 0)?;
            }
            let remaining = m.u16(p + 0x34)?;
            if remaining != 0 {
                let (step, correction) = if (m.u16(p + 0x2e)? as i16) < 0 {
                    (0x2c, 0x32)
                } else {
                    (0x2a, 0x30)
                };
                let position = m.u32(p + 8)?.wrapping_add(m.u16(p + step)? as i16 as u32);
                m.set_u32(p + 8, position)?;
                m.set_u16(
                    p + 0x2e,
                    m.u16(p + 0x2e)?.wrapping_add(m.u16(p + correction)?),
                )?;
                m.set_u16(p + 12, (position & 8191) as u16)?;
                m.set_u16(p + 0x34, remaining - 1)?;
                m.set_u16(p + 14, (position >> 13) as u16)?;
                if m.u8(p + 0x23)? != 0 {
                    m.set_u32(
                        p + 0x24,
                        m.u32(p + 0x24)?.wrapping_add(u32::from(m.u16(p + 0x28)?)),
                    )?;
                }
            } else {
                match m.u8(p)? {
                    1 => {
                        impact(m, p)?;
                        let kind = 6 + crate::frame_core::random(m)? % 3;
                        let target = u32::from(m.u16(p + 0x16)?);
                        call(
                            m,
                            "ProjectileInit",
                            &[
                                kind,
                                target,
                                target,
                                u32::from(m.u8(p + 0x1f)?),
                                u32::from(m.u16(p + 0x20)?),
                                u32::from(m.u8(p + 0x22)?),
                                0,
                                0,
                                u32::from(m.u8(p + 0x23)?),
                            ],
                        )?;
                    }
                    9 | 17 | 21 | 23 => impact(m, p)?,
                    20 | 22 => area_arrival(m, p)?,
                    _ => {}
                }
                m.set_u8(p + 2, 0)?;
            }
            Ok(())
        }
    }
}
pub fn call(m: &mut MemoriaJogo, name: &str, a: &[u32]) -> Result<u32> {
    match name {
        "ProjectileInit" => {
            let pool = m.u32(0x5080f0)?;
            let mut slot = 0;
            while slot < 100 && m.u8(pool.wrapping_add(slot * 56 + 2))? != 0 {
                slot += 1;
            }
            if slot == 100 {
                return Ok(slot);
            }
            let mut args = [0u32; 10];
            args[0] = slot;
            args[1..].copy_from_slice(a);
            set(m, &args)
        }
        "ProjectileSet" => set(m, a),
        "ProjectileLineaire" => {
            let p = record(m, a[0])?;
            line(m, p)?;
            Ok(0)
        }
        "ProjectileCalcul" => crate::frame::call(m, "ProjectileCalcul", a),
        _ => stop("Unknown Combat frame provider", 0x46e44c),
    }
}
pub fn isolated(id: u32) -> Option<(&'static str, usize)> {
    Some(match id {
        550 => ("ProjectileInit", 9),
        551 => ("ProjectileSet", 10),
        552 => ("ProjectileLineaire", 1),
        _ => return None,
    })
}
