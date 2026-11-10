//! Intact Watcom AI kernels. Native addresses and validation: research/ai-kernels.md.
use crate::{
    chain_technology::Technology, frame_tasks, EngineError, FrameRegisters as R, MemoriaJogo,
    Result,
};
fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}
fn task(i: u32) -> u32 {
    0x512320u32.wrapping_add(i.wrapping_mul(96))
}
fn city_base(p: u32) -> u32 {
    0x51b84cu32.wrapping_add(p.wrapping_mul(124))
}
fn periodic(m: &MemoriaJogo, period: u32, address: u32) -> Result<bool> {
    let n = m.u32(period)?;
    if n == 0 {
        return Err(stop("AI kernel x86 division fault", address));
    }
    Ok(m.u32(0x508f7c)? % n == 0)
}
/// iaProbSearchIfAlreadyOne 0x47d98b, including its scratch-register results.
pub fn search_registers(m: &MemoriaJogo, mut r: R) -> Result<R> {
    let kind = r.edx;
    let count = r.ebx;
    let first = r.ecx;
    let mut index = m.u32(0x5148b0u32.wrapping_add(r.eax.wrapping_shl(4)))?;
    for _ in 0..10001 {
        if index == u32::MAX {
            r.eax = index;
            r.ebx = index;
            r.ecx = index;
            r.esp = r.esp.wrapping_add(24);
            return Ok(r);
        }
        let t = task(index);
        r.edx = u32::from(m.u16(t.wrapping_add(0x5a))?);
        if r.edx == kind {
            r.edx = 0;
            for i in 0..6u32 {
                if (count as i32) >= (i + 1) as i32 {
                    let value = if i == 0 {
                        first
                    } else {
                        m.u32(r.esp.wrapping_add(4 * i))?
                    };
                    if value == m.u32(t.wrapping_add(0x18 + 4 * i))? {
                        r.edx = r.edx.wrapping_add(1);
                    }
                }
            }
            if r.edx == count {
                r.eax = index;
                r.ebx = index;
                r.ecx = index;
                r.esp = r.esp.wrapping_add(24);
                return Ok(r);
            }
        }
        index = m.u32(t)?;
    }
    Err(stop("iaProbSearchIfAlreadyOne cyclic list", 0x47d99d))
}
pub fn search(m: &MemoriaJogo, p: u32, kind: u32, args: &[u32]) -> Result<u32> {
    let mut index = m.u32(0x5148b0u32.wrapping_add(p.wrapping_shl(4)))?;
    for _ in 0..10001 {
        if index == u32::MAX {
            return Ok(index);
        }
        let t = task(index);
        if u32::from(m.u16(t.wrapping_add(0x5a))?) == kind {
            let mut equal = true;
            for (i, v) in args.iter().enumerate() {
                if m.u32(t.wrapping_add(0x18 + 4 * i as u32))? != *v {
                    equal = false;
                }
            }
            if equal {
                return Ok(index);
            }
        }
        index = m.u32(t)?;
    }
    Err(stop("iaProbSearchIfAlreadyOne cyclic list", 0x47d99d))
}
fn post(m: &mut MemoriaJogo, p: u32, kind: u32, args: &[u32]) -> Result<u32> {
    let mut a = [0; 8];
    a[0] = p;
    a[1] = kind;
    a[2..2 + args.len()].copy_from_slice(args);
    frame_tasks::post(m, &a)
}
fn technology(m: &mut MemoriaJogo, name: &str, p: u32, t: u32) -> Result<u32> {
    Ok(Technology::default()
        .entry(
            m,
            name,
            R {
                eax: p,
                edx: t,
                ..R::default()
            },
        )?
        .registers
        .eax)
}
/// iagVilleNoyau 0x48453c, active scan and construction paths.
pub fn city(m: &mut MemoriaJogo, p: u32) -> Result<u32> {
    let base = city_base(p);
    if !periodic(m, base.wrapping_add(0x58), 0x484560)? {
        return Ok(0);
    }
    for s in 0..1000u32 {
        let a = 0x49a0c8 + 276 * s;
        if m.u8(a + 5)? != 0
            && u32::from(m.u8(a + 4)?) == p
            && m.u16(a + 6)? != 0
            && search(m, p, 2, &[s])? == u32::MAX
        {
            post(m, p, 2, &[s])?;
        }
    }
    if m.u8(0x51a98eu32.wrapping_add(16 * p))? == 0 {
        return Ok(0);
    }
    for level in 1..=10u32 {
        let table = m.u32(0x498c24u32.wrapping_add(m.u32(base + 0x64)?.wrapping_mul(4)))?;
        for i in 0..22u32 {
            let t = u32::from(m.u8(table + 2 * i)?);
            if u32::from(m.u8(table + 2 * i + 1)?) < level {
                continue;
            }
            let a = base.wrapping_add(2 * t);
            if u32::from(m.u16(a)?) + u32::from(m.u16(a + 0x2c)?) >= level {
                continue;
            }
            if technology(m, "TypeBatimentEstRealisable", p & 255, t)? & 255 != 1 {
                continue;
            }
            post(m, p, 1, &[t])?;
            return Ok(0);
        }
    }
    Ok(0)
}
/// Register transport for routine-level native comparisons; no frame-time audit.
pub fn entry(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    match name {
        "iaProbSearchIfAlreadyOne" => return search_registers(m, r),
        "iagUnitNoyau" => r.eax = 0, // intact bytes 31 c0 c3
        "iagVilleNoyau" => r.eax = city(m, r.eax)?,
        "iagRessNoyau" => r.eax = resource(m, r.eax)?,
        "iagWarNoyau" => {
            r.eax = war(m, r.eax)?;
            r.edx = 0;
        }
        "iaSolver" => r.eax = frame_tasks::solve(m, r.eax)?,
        _ => return Err(stop("unknown AI kernel", 0x482936)),
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
fn resource_base(p: u32) -> u32 {
    0x51ba3cu32.wrapping_add(p.wrapping_mul(168))
}
fn war_base(p: u32) -> u32 {
    0x51bcfcu32.wrapping_add(p.wrapping_mul(0x1144))
}
fn built(m: &MemoriaJogo, p: u32, t: u32) -> Result<u32> {
    Ok(u32::from(m.u16(city_base(p).wrapping_add(2 * t))?))
}
/// First three GetRessource outputs, in original food/stone/parts order.
fn resources(m: &MemoriaJogo, p: u32, stock: bool) -> Result<[u16; 3]> {
    let b = 0x4e2ea8 + 32 * (p & 255);
    let mut result = [0; 3];
    for (i, o) in [8, 16, 0].into_iter().enumerate() {
        let value = m.u16(b + o + 2)?;
        result[i] = if stock {
            value.wrapping_sub(m.u16(b + o)?)
        } else {
            value
        };
    }
    Ok(result)
}
fn quit_survival(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    m.set_u8(0x51a98c + 16 * p, 1)?;
    let c = city_base(p);
    m.set_u32(c + 0x58, m.u32(0x498c14)?)?;
    m.set_u32(c + 0x64, 1)?;
    let r = resource_base(p);
    m.set_u32(r + 0x40, m.u32(0x498c3c)?)?;
    for i in 0..4 {
        m.set_u32(r + 0x30 + 4 * i, m.u32(0x498c70 + 4 * i)?)?;
    }
    m.set_u32(r + 0x48, m.u32(0x498c50)?)?;
    let w = war_base(p);
    m.set_u32(w, m.u32(0x498cb4)?)?;
    m.set_u32(w + 8, m.u32(0x498cc8)?)
}
fn ensure_post(m: &mut MemoriaJogo, p: u32, kind: u32, args: &[u32]) -> Result<()> {
    if search(m, p, kind, args)? == u32::MAX {
        post(m, p, kind, args)?;
    }
    Ok(())
}
/// iagRessNoyau 0x485183, including survival transition and resource waiters.
pub fn resource(m: &mut MemoriaJogo, p: u32) -> Result<u32> {
    let b = resource_base(p);
    let state = 0x51a98c + 16 * p;
    if !periodic(m, b + 0x40, 0x4851b8)? {
        return Ok(0);
    }
    for i in 0..4 {
        m.set_u32(b + 16 + 4 * i, m.u32(b + 4 * i)?)?;
    }
    for (i, v) in resources(m, p, false)?.into_iter().enumerate() {
        m.set_u32(b + 4 * i as u32, u32::from(v))?;
    }
    if m.u8(state)? == 0 {
        if built(m, p, 3)? >= 1 && m.u8(b + 0x9c)? == 0 {
            m.set_u8(b + 0x9c, 1)?;
            post(m, p, 11, &[])?;
            post(m, p, 12, &[])?;
        }
        if built(m, p, 4)? >= 1 && m.u8(b + 0x9d)? == 0 {
            m.set_u8(b + 0x9d, 1)?;
            post(m, p, 21, &[])?;
        }
        if m.u8(b + 0x9c)? != 0 && m.u8(b + 0x9d)? != 0 {
            m.set_u8(state + 3, 1)?;
            quit_survival(m, p)?;
        }
    } else {
        if m.u32(b + 0x88)? != 0 {
            let mut found = false;
            for _ in 0..10000 {
                let i = m.u32(b + 0x84)?;
                let a = b.wrapping_add(i.wrapping_mul(8));
                if m.u32(a + 0x58)? != u32::MAX {
                    post(m, p, 13, &[m.u32(a + 0x54)?])?;
                    found = true;
                    break;
                }
                let i = i.wrapping_add(1);
                m.set_u32(
                    b + 0x84,
                    if (i as i32) < (m.u32(0x51bcf0)? as i32) {
                        i
                    } else {
                        0
                    },
                )?;
            }
            if !found {
                return Err(stop("iagRessNoyau cyclic mine rotation", 0x485313));
            }
            let i = m.u32(b + 0x84)?.wrapping_add(1);
            m.set_u32(
                b + 0x84,
                if (i as i32) < (m.u32(0x51bcf0)? as i32) {
                    i
                } else {
                    0
                },
            )?;
        }
        let current = m.u32(b)? as i32;
        let delta = m.u32(b)?.wrapping_sub(m.u32(b + 0x10)?) as i32;
        let threshold = m.u32(b + 0x30)? as i32;
        if delta > (m.u32(b + 0x48)? as i32) {
            if current > threshold {
                ensure_post(m, p, 21, &[])?;
                m.set_u8(state + 2, 1)?;
                m.set_u8(state + 3, 1)?;
                m.set_u32(b + 0x4c, 0)?;
            } else {
                m.set_u32(b + 0x4c, 1)?;
            }
        } else if current > threshold {
            ensure_post(m, p, 11, &[])?;
            m.set_u32(b + 0x4c, 2)?;
        } else if m.u8(state + 1)? == 1
            && search(m, p, 10, &[])? == u32::MAX
            && search(m, p, 3, &[5])? == u32::MAX
        {
            ensure_post(m, p, 8, &[])?;
        } else if m.u8(state + 2)? == 1 {
            m.set_u8(state + 2, 0)?;
        } else if m.u8(state + 3)? == 1 {
            m.set_u8(state + 3, 0)?;
        } else {
            m.set_u32(b + 0x4c, 3)?;
        }
        ensure_post(m, p, 4, &[])?;
        if built(m, p, 5)? >= 1 {
            ensure_post(m, p, 6, &[])?;
        }
        let amounts = resources(m, p, true)?;
        let limit = u32::from(m.u16(0x4dfb2e + 6 * p)?)
            .wrapping_mul(u32::from(m.u16(0x4dfb2c + 6 * p)?))
            .wrapping_sub(15) as i32;
        if limit < (u32::from(amounts[0]) + u32::from(amounts[1])) as i32 {
            ensure_post(m, p, 1, &[3])?;
        }
    }
    let mut previous = u32::MAX;
    let mut current = m.u32(b + 0x44)?;
    for _ in 0..10001 {
        if current == u32::MAX {
            return Ok(0);
        }
        let t = task(current);
        let next = m.u32(t + 8)?;
        let a = b.wrapping_add(4 * u32::from(m.u8(t + 0x5d)?));
        let cost = m.u32(t + 0x10)?;
        if (cost as i32) <= (m.u32(a)? as i32) {
            m.set_u32(a, m.u32(a)?.wrapping_sub(cost))?;
            m.set_u32(a + 0x20, m.u32(a + 0x20)?.wrapping_sub(cost))?;
            frame_tasks::wake(m, current, 1)?;
            if current == m.u32(b + 0x44)? {
                m.set_u32(b + 0x44, m.u32(t + 8)?)?;
            } else {
                m.set_u32(task(previous) + 8, m.u32(t + 8)?)?;
            }
            m.set_u8(t + 0x5d, 255)?;
        }
        previous = current;
        current = next;
    }
    Err(stop("iagRessNoyau cyclic resource waiters", 0x48561b))
}
pub(crate) fn first_group(m: &MemoriaJogo, p: u32, kind: u32) -> Result<u32> {
    m.u32(
        0x51b664u32
            .wrapping_add(p.wrapping_mul(96))
            .wrapping_add(kind.wrapping_mul(4)),
    )
}
/// Typed EnginGetRessource 0x4410d2 outputs in food/stone/parts order.
pub(crate) fn unit_costs(m: &MemoriaJogo, kind: u32) -> Result<[u16; 3]> {
    let a = 0x4965ae + 50 * (kind & 255);
    Ok([m.u16(a + 2)?, m.u16(a + 6)?, m.u16(a + 4)?])
}
fn group_address(index: u32) -> u32 {
    0x51a9e0u32.wrapping_add(index.wrapping_shl(5))
}
fn alive(m: &mut MemoriaJogo, u: u16) -> Result<bool> {
    Ok(crate::frame_spawn::call(m, "EnginEstVivant", &[u32::from(u)])? as u8 != 0)
}
fn ai_random(m: &mut MemoriaJogo, n: u32) -> Result<u32> {
    let mut rng = crate::WorldRng {
        seed: m.u32(0x498db4)?,
    };
    let value = rng.draw() % n;
    m.set_u32(0x498db4, rng.seed)?;
    Ok(value)
}
/// Native recruitment helper 0x486e39. Resource arguments are local u16 values,
/// updated in exactly the order of its original pointer stores.
fn recruit(
    m: &mut MemoriaJogo,
    p: u32,
    table: u32,
    amounts: &mut [u16; 3],
    last: u32,
) -> Result<u32> {
    first_group(m, p, 0)?;
    let army = first_group(m, p, 9)?;
    if last != 255 && search(m, p, 3, &[last])? != u32::MAX {
        return Ok(255);
    }
    let mut count = 0;
    let mut cursor = table;
    let mut terminated = false;
    for _ in 0..256 {
        let t = u32::from(m.u8(cursor)?);
        if t == 255 {
            terminated = true;
            break;
        }
        let enabled = technology(m, "TypeEnginEstRealisable", p & 255, t)? as u8 != 0
            && built(m, p, u32::from(m.u8(cursor + 1)?))? > 0;
        m.set_u8(
            0x51ce24u32
                .wrapping_add(p.wrapping_mul(0x1144))
                .wrapping_add(t),
            u8::from(enabled),
        )?;
        count += u32::from(enabled);
        cursor += 2;
    }
    if !terminated {
        return Err(stop("AI recruitment unterminated type table", 0x486eb3));
    }
    cursor = table;
    for _ in 0..count {
        for _ in 0..256 {
            let t = u32::from(m.u8(cursor)?);
            if m.u8(0x51ce24u32
                .wrapping_add(p.wrapping_mul(0x1144))
                .wrapping_add(t))?
                != 0
            {
                break;
            }
            cursor += 2;
        }
        let t = u32::from(m.u8(cursor)?);
        let costs = unit_costs(m, t)?;
        if costs[0] >= amounts[0] || costs[1] > amounts[1] || costs[2] > amounts[2] {
            continue;
        }
        let g = group_address(army);
        if g != 0 && m.u16(g + 0x12)? != 0 {
            let mut u = crate::frame_raid::first_unit(m, army)?;
            let mut ended = false;
            for _ in 0..801 {
                if u == 65535 {
                    ended = true;
                    break;
                }
                alive(m, u)?;
                u = crate::frame_raid::next_unit(m)?;
            }
            if !ended {
                return Err(stop("AI recruitment cyclic group", 0x486fa8));
            }
        }
        m.u32(city_base(p) + 0x6c)?; // native built-total query, result discarded
        post(m, p, 3, &[t])?;
        for i in 0..3 {
            amounts[i] = amounts[i].wrapping_sub(costs[i]);
        }
        return Ok(t);
    }
    Ok(255)
}
/// iagWarNoyau 0x4866d7: special requests, recruitment and target scheduling.
pub fn war(m: &mut MemoriaJogo, p: u32) -> Result<u32> {
    let b = war_base(p);
    let frame = m.u32(0x508f7c)?;
    if (p as i32) < 4 {
        let period = m.u32(0x521364)?;
        if period == 0 {
            return Err(stop("AI kernel x86 division fault", 0x48672c));
        }
        let stagger = ((period.wrapping_mul(p) as i32) / 4) as u32;
        if frame % period == stagger {
            if ai_random(m, 3)? == 0 {
                if (built(m, p, 16)? as i32) > 0 {
                    ensure_post(m, p, 42, &[])?;
                }
            } else {
                ensure_post(m, p, 39, &[])?;
            }
        }
    }
    if !periodic(m, b, 0x4867c1)? || m.u8(0x51a98f + 16 * p)? == 0 {
        return Ok(0);
    }
    let mut amounts = resources(m, p, false)?;
    for (table, offset) in [(0x498cd8, 12), (0x498cec, 16), (0x498ce0, 20)] {
        let t = recruit(m, p, table, &mut amounts, m.u32(b + offset)?)?;
        if t != 255 {
            m.set_u32(b + offset, t)?;
        }
    }
    let army = first_group(m, p, 0)?;
    let g = group_address(army);
    let mut strength = 0u32;
    if g != 0 && m.u16(g + 0x12)? != 0 {
        let mut u = crate::frame_raid::first_unit(m, army)?;
        let mut ended = false;
        for _ in 0..801 {
            if u == 65535 {
                ended = true;
                break;
            }
            if alive(m, u)? {
                let t = crate::frame_core::call(m, "EnginGetType", &[u32::from(u)])? & 255;
                strength = strength.wrapping_add(m.u32(0x498cf4 + 4 * t)?);
            }
            u = crate::frame_raid::next_unit(m)?;
        }
        if !ended {
            return Err(stop("iagWarNoyau cyclic group", 0x4868dc));
        }
    }
    let mode = m.u32(resource_base(p) + 0x4c)?;
    let mut minimum = 500i32;
    let mut target = 0;
    for q in 0..4u32 {
        if q == p {
            continue;
        }
        let players = m.u32(0x5080a8)?;
        let mut sympathy =
            i32::from(m.u16(players.wrapping_add(20 * (p & 255) + 12 + 2 * (q & 255)))? as i16);
        if crate::frame_economy::call(m, "PlayerEstAllie", &[p, q])? as u8 != 0 {
            sympathy += 25;
        }
        if sympathy < minimum {
            minimum = sympathy;
            target = q;
        }
    }
    let status;
    if mode == 3 && (strength as i32) > (m.u32(0x521350)? as i32) {
        post(m, p, 36, &[0, target, 5])?;
        status = 1;
    } else if (strength as i32) > (m.u32(0x521354)? as i32) {
        post(m, p, 36, &[0, target, 6])?;
        status = 6;
    } else {
        let mut hostile = 0u32;
        for u in 0..800u16 {
            if alive(m, u)?
                && crate::frame_core::call(m, "EnginGetPlayer", &[u32::from(u)])? & 255 == target
            {
                let t = crate::frame_core::call(m, "EnginGetType", &[u32::from(u)])? & 255;
                if 1u32.wrapping_shl(t) & 0x03f1991f != 0 {
                    hostile = hostile.wrapping_add(m.u32(0x498cf4 + 4 * t)?);
                }
            }
        }
        if (hostile as i32) < (m.u32(0x521360)? as i32) {
            post(m, p, 37, &[0, target, 3])?;
            status = 8;
        } else {
            let wealth = resources(m, target, true)?
                .into_iter()
                .map(u32::from)
                .sum::<u32>();
            if (wealth as i32) < (m.u32(0x521358)? as i32) {
                status = 0;
            } else {
                post(m, p, 36, &[1, target, 2])?;
                status = 9;
            }
        }
    }
    m.set_u32(b + 0x18, status)?;
    Ok(0)
}
