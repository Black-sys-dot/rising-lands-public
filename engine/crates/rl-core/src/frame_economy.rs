//! Economy frame effects, mechanically ported from the accepted Python providers.
use crate::{BcdDigits, EngineError, MemoriaJogo, Result};
fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}
fn f64_read(m: &MemoriaJogo, a: u32) -> Result<f64> {
    Ok(f64::from_le_bytes(
        m.read(a, 8)?.try_into().expect("f64 width"),
    ))
}
fn f64_write(m: &mut MemoriaJogo, a: u32, v: f64) -> Result<()> {
    m.write(a, &v.to_le_bytes())
}
pub fn bonus(m: &MemoriaJogo, owner: u32, kind: u32, selector: u32, ecx: u32) -> Result<u32> {
    let p = owner & 255;
    let t = kind & 255;
    let s = selector & 255;
    if p == 4 || p == 200 {
        return Ok(0);
    }
    if s == 8 || s > 12 {
        return Ok(ecx);
    }
    let offsets = [
        (0, 2),
        (11, 1),
        (2, 1),
        (4, 2),
        (6, 1),
        (8, 2),
        (10, 1),
        (12, 1),
        (0, 0),
        (14, 1),
        (17, 1),
        (15, 1),
        (16, 1),
    ];
    let (o, width) = offsets[s as usize];
    let a = 0x4dfba0u32.wrapping_add(p * 468 + t * 18 + o);
    if width == 1 {
        Ok(u32::from(m.u8(a)?))
    } else {
        Ok(u32::from(m.u16(a)?))
    }
}
fn material(m: &mut MemoriaJogo) -> Result<()> {
    let local = u32::from(m.u8(0x508041)?);
    for r in 0..4 {
        let a = 0x4e2ea8 + ((local * 4 + r) << 3);
        let mut shown = BcdDigits::encode(m.u16(a + 4)?).0;
        let target = BcdDigits::encode(m.u16(a + 2)?).0;
        for c in 0..4 {
            let d = shown[c];
            let t = target[c];
            let mask = 1u8 << c;
            let on = m.u8(a + 6)? & mask != 0;
            if d < t {
                m.set_u8(0x4e05a2, 1)?;
                if on {
                    shown[c] = d.wrapping_add(1);
                    m.set_u8(a + 6, m.u8(a + 6)? & !mask)?;
                } else {
                    m.set_u8(a + 6, m.u8(a + 6)? | mask)?;
                }
            } else if d > t {
                m.set_u8(0x4e05a2, 1)?;
                if on {
                    m.set_u8(a + 6, m.u8(a + 6)? & !mask)?;
                } else {
                    shown[c] = d.wrapping_sub(1);
                    m.set_u8(a + 6, m.u8(a + 6)? | mask)?;
                }
            } else if on {
                m.set_u8(a + 6, m.u8(a + 6)? & !mask)?;
                m.set_u8(0x4e05a2, 1)?;
            }
        }
        let mut val = 0u16;
        for (d, k) in shown.into_iter().zip([1u16, 10, 100, 1000]) {
            val = val.wrapping_add(u16::from(d).wrapping_mul(k));
        }
        m.set_u16(a + 4, val)?;
    }
    Ok(())
}
fn research(m: &mut MemoriaJogo) -> Result<()> {
    let mission = m.u8(0x496fe4)?;
    for p in 0..4 {
        if m.u8(0x50871c + p)? >= 2 {
            continue;
        }
        let base = 0x5080f4 + 16 * p;
        let branch = u32::from(m.u8(base + 4)?);
        let item = u32::from(m.u8(base + branch)?);
        let reg = 0x50817c + branch * 0x168 + item * 12;
        let available = m.u8(reg + 6)? != 255 && m.u8(reg + 5)? <= mission;
        if available {
            let scientists = m.u16(0x508174 + 2 * p)?;
            let increment = if scientists == 0 {
                0.0
            } else {
                f64_read(m, 0x494cbb)? - 1.0 / f64::from(scientists)
            };
            f64_write(m, base + 8, f64_read(m, base + 8)? + increment)?;
            m.set_u16(0x508174 + 2 * p, 0)?;
            let branch = u32::from(m.u8(base + 4)?);
            let item = u32::from(m.u8(base + branch)?);
            let reg = 0x50817c + branch * 0x168 + item * 12;
            if f64::from(m.u32(reg)?) > f64_read(m, base + 8)? {
                continue;
            }
            m.set_u8(0x50871c + p, m.u8(0x50871c + p)?.wrapping_add(1))?;
            crate::chain_technology::apply_invention(m, p, item, branch)?;
        } else if m.u8(reg + 6)? == 255 {
            m.set_u8(base + 4, ((branch + 1) % 4) as u8)?;
            continue;
        } else if m.u8(reg + 4)? == 100 {
            continue;
        }
        let branch = u32::from(m.u8(base + 4)?);
        m.set_u8(base + branch, m.u8(base + branch)?.wrapping_add(1))?;
        m.set_u32(base + 8, 0)?;
        m.set_u32(base + 12, 0)?;
    }
    Ok(())
}
pub fn sympathy(m: &mut MemoriaJogo, a: u32, b: u32, event: u32) -> Result<()> {
    if a == b || a >= 4 || b >= 4 {
        return Ok(());
    }
    if m.u8(0x51a990 + 16 * a)? != 0 || m.u8(0x51a990 + 16 * b)? != 0 {
        return Ok(());
    }
    let reg = 0x497288u32.wrapping_add(event.wrapping_shl(4));
    let counter = m.u32(reg + 8)?.wrapping_add(1);
    m.set_u32(reg + 8, counter)?;
    if counter < m.u32(reg + 12)? {
        return Ok(());
    }
    let mode = m.u8(reg + 4)?;
    m.set_u32(reg + 8, 0)?;
    let pairs = if mode == 1 {
        (0..4)
            .flat_map(|i| (0..4).filter(move |j| *j != i).map(move |j| (i, j)))
            .collect::<Vec<_>>()
    } else {
        vec![(a, b)]
    };
    for (i, j) in pairs {
        let addr = m.u32(0x5080a8)?.wrapping_add(20 * i + 12 + 2 * j);
        let current = i32::from(m.u16(addr)? as i16);
        let sum = (m.u32(reg)? as i32).wrapping_add(current);
        m.set_u16(addr, sum.clamp(-50, 50) as u16)?;
    }
    Ok(())
}
fn alliances(m: &mut MemoriaJogo) -> Result<()> {
    for p in 0..4 {
        for q in 0..4 {
            let a = 0x5080b0 + 16 * p + 4 * q;
            let v = m.u32(a)?;
            if v == 0 {
                continue;
            }
            m.set_u32(a, v.wrapping_sub(1))?;
            if v == 1 && p == u32::from(m.u8(0x508041)?) {
                // 0x46dab0: the local player's unanswered proposal expires:
                // InitGameZone(1), then MAJSympathie(proposer q, p, event 9).
                crate::frame_core::call(m, "InitGameZone", &[1, 0])?;
                call(m, "MAJSympathie", &[q, p, 9])?;
            }
        }
    }
    Ok(())
}
fn attack_sympathy(m: &mut MemoriaJogo) -> Result<()> {
    if m.u32(0x508f7c)? % 2000 != 0 {
        return Ok(());
    }
    for p in 0..4 {
        for q in 0..4 {
            let a = 0x508044 + 20 * p + 4 * q;
            if m.u32(a)? == 0 && p != q {
                sympathy(m, p, q, 0x13)?;
            }
            m.set_u32(a, 0)?;
        }
    }
    Ok(())
}
fn new_plant(m: &mut MemoriaJogo, c: u16) -> Result<bool> {
    let a = m.u32(0x507d74)?.wrapping_add(u32::from(c));
    if m.u8(a)? != 0 || m.u8(m.u32(0x507da8)?.wrapping_add(u32::from(c)))? & 1 != 0 {
        return Ok(false);
    }
    let t = if crate::frame_core::random(m)? & 1 != 0 {
        0x29
    } else {
        0x1d
    };
    m.set_u8(a, t)?;
    Ok(true)
}
fn spread(m: &mut MemoriaJogo, c: u16) -> Result<()> {
    for d in [u16::MAX, 1, 0xff00, 256] {
        let n = c.wrapping_add(d);
        let t = m.u16(m.u32(0x507d70)?.wrapping_add(u32::from(n) * 2))?;
        if ((0x84..0x87).contains(&t) || (0x9d..0xdd).contains(&t)) && new_plant(m, n)? {
            return Ok(());
        }
    }
    Ok(())
}
fn schedule(m: &mut MemoriaJogo) -> Result<()> {
    let period = u32::from(m.u16(0x4973ea)?);
    if period == 0 {
        return Err(stop("SetUpdateChamp division by zero", 0x475328));
    }
    if m.u32(0x508f7c)? % period == 0 {
        m.set_u8(0x509952, 1)?;
    }
    Ok(())
}
fn update_fields(m: &mut MemoriaJogo) -> Result<()> {
    let row = m.u16(0x4973e8)?;
    if m.u8(0x509952)? == 0 && row <= 1 {
        return Ok(());
    }
    if row > 255 {
        return Err(stop("UpdateChamp row outside captured grid", 0x478806));
    }
    for x in 1..255 {
        let c = row * 256 + x;
        let a = m.u32(0x507d74)?.wrapping_add(u32::from(c));
        if m.u8(a)? == 0x1c && crate::frame_core::random(m)? % 5 == 0 {
            spread(m, c)?;
        }
        let t = m.u8(a)?;
        if !(0x1d..=0x34).contains(&t) || crate::frame_core::random(m)? % 10 != 0 {
            continue;
        }
        let t = m.u8(a)?;
        if !((0x1d..0x28).contains(&t) || (0x29..0x34).contains(&t)) {
            m.set_u8(a, 0)?;
            spread(m, c)?;
        } else if (0x24..=0x27).contains(&t) || (0x30..=0x33).contains(&t) {
            if crate::frame_core::random(m)? % 5 == 0 {
                m.set_u8(a, t.wrapping_add(1))?;
            } else {
                spread(m, c)?;
            }
        } else {
            m.set_u8(a, t.wrapping_add(1))?;
        }
    }
    let next = row.wrapping_add(1);
    m.set_u16(0x4973e8, next)?;
    if next > 255 {
        m.set_u16(0x4973e8, 1)?;
        m.set_u8(0x509952, 0)?;
    }
    Ok(())
}
fn state7(m: &mut MemoriaJogo, i: u32) -> Result<()> {
    let a = crate::frame_core::unit(m, i)?;
    let t = m.u8(a + 1)?;
    if t == 17 {
        // CalculEnginStatic 0x4530a6: the balloon only drifts, then returns.
        return crate::frame_movement::balloon_animation(m, i);
    }
    if (11..=16).contains(&t) || t >= 26 {
        return Ok(());
    }
    if crate::frame_core::random(m)? % 200 != 0 {
        return Ok(());
    }
    let dir = m.u8(a + 0x91)? % 16 + 1;
    m.set_u8(a + 0x90, dir)?;
    m.set_u8(a + 0x91, dir)?;
    Ok(())
}
fn count_type(m: &MemoriaJogo, index: u32, kind: u8) -> Result<u32> {
    let a = 0x49a0c8 + (index & 0xffff) * 276;
    let mut count = 0;
    for slot in 0..u32::from(m.u16(a + 14)?) {
        if m.u8(a + 0x4e + slot)? != 1 {
            continue;
        }
        let unit = u32::from(m.u16(a + 0x12 + slot * 2)?);
        if m.u8(m.u32(0x4e19a0)?.wrapping_add(unit * 276 + 1))? == kind {
            count += 1;
        }
    }
    Ok(count)
}
fn troop_in_bag(m: &MemoriaJogo, index: u32) -> Result<bool> {
    let a = 0x49a0c8 + (index & 0xffff) * 276;
    for slot in 0..u32::from(m.u16(a + 14)?) {
        if m.u8(a + 0x4e + slot)? != 1 {
            continue;
        }
        let unit = u32::from(m.u16(a + 0x12 + slot * 2)?);
        let t = m.u8(m.u32(0x4e19a0)?.wrapping_add(unit * 276 + 1))?;
        if !matches!(t, 3 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 25) {
            return Ok(true);
        }
    }
    Ok(false)
}
/// BatimentNextEtape habitation predicate (0x426933..0x426978): troops for
/// types 2/21, otherwise colonists (tag 1 value 9) in the structure Bag.
pub fn inhabited(m: &MemoriaJogo, index: u32) -> Result<bool> {
    let a = 0x49a0c8 + (index & 0xffff) * 276;
    let kind = m.u8(a + 8)?;
    Ok(if (kind == 2 || kind == 21) && troop_in_bag(m, index)? {
        true
    } else if kind == 21 {
        false
    } else {
        count_type(m, index, 9)? != 0
    })
}
pub fn production(
    m: &mut MemoriaJogo,
    index: u32,
    require_inhabited: bool,
    declared_chain: bool,
) -> Result<()> {
    let a = 0x49a0c8 + (index & 0xffff) * 276;
    let kind = m.u8(a + 8)?;
    if require_inhabited && !inhabited(m, index)? {
        return Ok(());
    }
    let progress = m.u32(a + 0x8c)?;
    if progress == 0 || m.u8(a + 0x74)? == 0 {
        return Ok(());
    }
    let request = u32::from(m.u8(a + 0x7b)?);
    let time = u32::from(m.u16(0x4965ae + 50 * request)?) * 100;
    if progress < time {
        let qty = count_type(m, index, 9)?;
        let mut divisor = m.u8(0x495d8c + 52 * u32::from(kind) + 18)?;
        if kind == 14 {
            divisor = divisor.wrapping_sub(1);
        } else if kind == 19 {
            divisor = divisor.wrapping_sub(m.u8(0x495be8)?);
        }
        let coefficient = f64_read(m, if kind == 14 { 0x494432 } else { 0x49443a })?;
        let converted = if divisor == 0 {
            0x80000000
        } else {
            ((f64::from(qty) / f64::from(divisor)) * coefficient).trunc() as u32
        };
        m.set_u32(a + 0x8c, progress.wrapping_add(100 + (converted & 0xffff)))?;
    }
    if m.u32(a + 0x8c)? < time {
        return Ok(());
    }
    if kind == 18 {
        return crate::building_support::complete_type18(m, index as u16);
    }
    if declared_chain {
        crate::chain_recruit::sortie(m, index, 4)
    } else {
        Err(stop("SortieTroupe completion", 0x420e4c))
    }
}
pub fn call(m: &mut MemoriaJogo, name: &str, a: &[u32]) -> Result<u32> {
    match name {
        "CalculMatiere" => {
            material(m)?;
            Ok(0)
        }
        "RechercheCalcul" => {
            research(m)?;
            Ok(0)
        }
        "PlayerInitAllianceCompteur" => {
            alliances(m)?;
            Ok(0)
        }
        "PlayerAttaqueCalculSympa" => {
            attack_sympathy(m)?;
            Ok(0)
        }
        "MAJSympathie" => {
            sympathy(m, a[0], a[1], a[2])?;
            Ok(0)
        }
        "SetUpdateChamp" => {
            schedule(m)?;
            Ok(0)
        }
        "UpdateChamp" => {
            update_fields(m)?;
            Ok(0)
        }
        "ResetTableauHangar" => {
            for p in 0..4 {
                for i in 0..10 {
                    m.set_u16(0x4e296c + 20 * p + 2 * i, 0xffff)?;
                }
            }
            Ok(0)
        }
        "GetMeteo" => {
            let count = m.u32(0x508f7c)?;
            if count % 900 == 0 {
                m.set_u8(0x496d2c, m.u8(0x496d2c)?.wrapping_add(1) % 3)?;
            }
            m.set_u8(0x496d2c, 0)?;
            if count % 100 == 0 {
                let d = if crate::frame_core::random(m)? % 5 < 3 {
                    1
                } else {
                    u16::MAX
                };
                let v = m.u16(0x496d2e)?.wrapping_add(d);
                m.set_u16(
                    0x496d2e,
                    if v == 0xfffb {
                        0xfffc
                    } else if v == 5 {
                        4
                    } else {
                        v
                    },
                )?;
            }
            Ok(0)
        }
        "CalculCompteurMarche" => {
            crate::market::counter_market(m, a[0])?;
            Ok(0)
        }
        "CalculCompteurProposition" => {
            crate::market::counter_proposal(m, a[0])?;
            Ok(0)
        }
        "BonusGetValEngin" => bonus(m, a[0], a[1], a[2], a[3]),
        "EnginNextEtape:state7" => {
            state7(m, a[0])?;
            Ok(0)
        }
        "BatimentNextEtape:production" | "CalculTroupe" => {
            production(m, a[0], name != "CalculTroupe", m.player.is_some())?;
            Ok(0)
        }
        "PlayerGetType" => {
            let p = a[0] & 255;
            Ok(if p < 4 {
                u32::from(m.u8(m.u32(0x5080a8)?.wrapping_add(20 * p + 1))?)
            } else {
                2
            })
        }
        "PlayerEstAllie" => {
            let p = a[0] & 255;
            let q = a[1] & 255;
            if p == q {
                return Ok(1);
            }
            if p > 4 || q > 4 {
                return Ok(0);
            }
            let v = m.u32(m.u32(0x5080a8)?.wrapping_add(20 * p + 3))?;
            Ok(u32::from((v >> 24) & (1 << q) != 0))
        }
        "RechercheGetNbInvention" => Ok(u32::from(m.u8(0x50871c + (a[0] & 255))?)),
        "BatimentGetType" => Ok(u32::from(m.u8(0x49a0c8 + (a[0] & 0xffff) * 276 + 8)?)),
        "TestRessource" => {
            let base = 0x4e2ea8 + 32 * (a[0] & 255);
            for (cost, slot) in [(a[1], 1), (a[2], 2), (a[3], 0)] {
                if (cost as u16) > m.u16(base + 8 * slot + 2)? {
                    return Ok(0);
                }
            }
            Ok(1)
        }
        "BagContientTroupe" => Ok(u32::from(troop_in_bag(
            m,
            (a[0].wrapping_sub(0x49a0d4)) / 276,
        )?)),
        "BagVide" => {
            let c = m.u16(a[0])?;
            Ok(u32::from(c & 0xff00) | u32::from(c == 0))
        }
        "PlayerGetColor" => Ok(if a[0] & 255 == 200 {
            30
        } else {
            u32::from(m.u8(0x4e24dc + (a[0] & 255))?)
        }),
        _ => Err(stop("Economy frame provider", 0x42c868)),
    }
}

/// Stable isolated-provider ids for capture and reference comparisons.
pub fn isolated(id: u32) -> Option<(&'static str, usize)> {
    Some(match id {
        250 => ("CalculMatiere", 0),
        251 => ("RechercheCalcul", 0),
        252 => ("PlayerInitAllianceCompteur", 0),
        253 => ("PlayerAttaqueCalculSympa", 0),
        254 => ("MAJSympathie", 3),
        255 => ("SetUpdateChamp", 0),
        256 => ("UpdateChamp", 0),
        257 => ("ResetTableauHangar", 0),
        258 => ("GetMeteo", 0),
        259 => ("CalculCompteurMarche", 1),
        260 => ("CalculCompteurProposition", 2),
        261 => ("BonusGetValEngin", 4),
        262 => ("EnginNextEtape:state7", 1),
        263 => ("BatimentNextEtape:production", 1),
        264 => ("CalculTroupe", 1),
        265 => ("PlayerGetType", 1),
        266 => ("PlayerEstAllie", 2),
        267 => ("RechercheGetNbInvention", 1),
        268 => ("BatimentGetType", 1),
        269 => ("BagVide", 1),
        270 => ("PlayerGetColor", 1),
        271 => ("BagContientTroupe", 1),
        272 => ("TestRessource", 4),
        _ => return None,
    })
}
