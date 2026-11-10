//! Resource cargo removal443dfc, sack search4757af and SacDepot47605d.
use crate::{
    frame::stop,
    player::{unit, Memory},
    MemoriaJogo, Result,
};

pub(crate) fn accepts(m: &MemoriaJogo, cell: u16, wanted: u8) -> Result<bool> {
    let object = m.b(m.d(0x507d74)? + u32::from(cell))?;
    Ok((object == 0 || object == u32::from(wanted))
        && !(object == u32::from(wanted) && m.b(m.d(0x507dac)? + u32::from(cell))? == 31))
}
/// Native AX result. The zero random ordinal deliberately selects no candidate:
/// the native eligible counter increments before comparing with that ordinal.
fn find_info(m: &mut MemoriaJogo, cell: u16, wanted: u8) -> Result<(u16, bool)> {
    if accepts(m, cell, wanted)? {
        return Ok((cell, false));
    }
    let (x, y) = (i32::from(cell & 255), i32::from(cell >> 8));
    let mut any_eligible = false;
    for radius in 1..6i32 {
        let ordinal = crate::frame_core::random(m)? % (radius as u32 * 8);
        let mut eligible = 0;
        for offset in -radius..=radius {
            for (side, (cx, cy)) in [
                (x - radius, y + offset),
                (x + radius, y + offset),
                (x + offset, y - radius),
                (x + offset, y + radius),
            ]
            .into_iter()
            .enumerate()
            {
                if side >= 2 && (offset == -radius || offset == radius) {
                    continue;
                }
                if !(0..256).contains(&cx) || !(0..256).contains(&cy) {
                    continue;
                }
                let candidate = (cx + 256 * cy) as u16;
                if accepts(m, candidate, wanted)? {
                    any_eligible = true;
                    eligible += 1;
                    if eligible == ordinal {
                        return Ok((candidate, false));
                    }
                }
            }
        }
    }
    Ok((65535, any_eligible))
}
/// SacCherchePlaceLibre 0x475bfe: the cell itself when it accepts the sack and the
/// unit can pass (EnginPeutPasser, layer 1), else rings 1..5: per offset s = -r..r
/// the left column (x-r, y+s), the right column (x+r, y+s), then, off the corners,
/// the top row (x+s, y-r) and the bottom row (x+s, y+r), each clipped to the map
/// square; the first acceptable passable cell wins. Each ring first draws Random
/// (0x475ce4) and discards it. 0xffff when nothing qualifies.
pub(crate) fn find_free(m: &mut MemoriaJogo, cell: u16, wanted: u8, u: u32) -> Result<u16> {
    let ok = |m: &mut MemoriaJogo, c: i32| -> Result<bool> {
        Ok(accepts(m, c as u16, wanted)? && crate::chain_search::passable(m, u, c as u32)?)
    };
    if ok(m, i32::from(cell))? {
        return Ok(cell);
    }
    let (x, y) = (i32::from(cell & 255), i32::from(cell >> 8));
    let inside = |v: i32| (0..256).contains(&v);
    for r in 1..6i32 {
        crate::frame_core::random(m)?;
        for s in -r..=r {
            if x - r >= 0 && inside(y + s) && ok(m, x - r + 256 * (y + s))? {
                return Ok((x - r + 256 * (y + s)) as u16);
            }
            if x + r < 256 && inside(y + s) && ok(m, x + r + 256 * (y + s))? {
                return Ok((x + r + 256 * (y + s)) as u16);
            }
            if s == -r || s == r {
                continue;
            }
            if y - r >= 0 && inside(x + s) && ok(m, x + s + 256 * (y - r))? {
                return Ok((x + s + 256 * (y - r)) as u16);
            }
            if y + r < 256 && inside(x + s) && ok(m, x + s + 256 * (y + r))? {
                return Ok((x + s + 256 * (y + r)) as u16);
            }
        }
    }
    Ok(65535)
}
pub(crate) fn find(m: &mut MemoriaJogo, cell: u16, wanted: u8) -> Result<u16> {
    Ok(find_info(m, cell, wanted)?.0)
}
/// BagRetireIndice4473cf, valid caller slot; return the original u16 value.
pub(crate) fn remove_index(m: &mut MemoriaJogo, bag: u32, index: u16) -> Result<u16> {
    m.sw(bag + 0x66, m.w(bag + 0x66)?.wrapping_sub(1))?;
    let value = m.w(bag + 6 + 2 * u32::from(index))? as u16;
    let mut i = u32::from(index);
    let last = m.w(bag)?.wrapping_sub(1) & 65535;
    while i < last {
        m.sb(bag + 0x42 + i, m.b(bag + 0x43 + i)?)?;
        m.sw(bag + 6 + 2 * i, m.w(bag + 8 + 2 * i)?)?;
        i += 1;
    }
    m.sb(bag + 0x42 + i, 0)?;
    m.sw(bag + 6 + 2 * i, 0)?;
    m.sw(bag, m.w(bag)?.wrapping_sub(1))?;
    Ok(value)
}
pub(crate) fn deposit(m: &mut MemoriaJogo, mut cell: u16, resource: u8) -> Result<()> {
    let wanted = resource.wrapping_add(0x42);
    loop {
        let object = m.d(0x507d74)? + u32::from(cell);
        let quantity = m.d(0x507dac)? + u32::from(cell);
        let old = m.b(object)?;
        if old != u32::from(wanted) && old != 0 {
            return Ok(());
        }
        if old == 0 {
            m.sb(quantity, 1)?;
            m.sb(object, u32::from(wanted))?;
            return Ok(());
        }
        let next = (m.b(quantity)? as u8).wrapping_add(1);
        m.sb(quantity, u32::from(next))?;
        if next <= 31 {
            return Ok(());
        }
        m.sb(quantity, u32::from(next.wrapping_sub(1)))?;
        let seed = m.d(0x497164)?;
        let (found, retry_can_select) = find_info(m, cell, wanted)?;
        if found == cell && (!retry_can_select || seed == m.d(0x497164)?) {
            return stop("SacDepot repeated-cell retry continuation", 0x476064);
        }
        // SacDepot itself does not reject ffff: it accesses that mapped grid
        // cell on the next loop. The caller below has a different sentinel test.
        cell = found;
    }
}
fn drop_one(m: &mut MemoriaJogo, member: u16, index: u16) -> Result<bool> {
    let r = unit(m, u32::from(member))?;
    let bag = r + 10;
    let value = m.w(bag + 6 + 2 * u32::from(index))?;
    match m.b(bag + 0x42 + u32::from(index))? {
        1 => {
            release_carried(m, member, index, value as u16)?;
            Ok(false)
        }
        2 => {
            let (chosen, retry_can_select) =
                find_info(m, m.w(r + 8)? as u16, (value as u8).wrapping_add(0x42))?;
            if chosen == 65535 {
                return Ok(retry_can_select);
            }
            let removed = remove_index(m, bag, index)?;
            deposit(m, chosen, removed as u8)?;
            Ok(false)
        }
        _ => Ok(false), // The native routine returns without changing other tags.
    }
}
/// TypeEnginEstTroupe 0x442347, inlined in EnginRetire's bag scan 0x4443a1.
fn troop(kind: u32) -> bool {
    !matches!(kind, 3 | 11..=17 | 25)
}
/// Put a released unit back on the map at `place` (0x443f0f..0x44431d and
/// the identical 0x44416c..0x44431d): position, selection release, state 7
/// for a live unit, targets and orders cleared, collision, sight, minimap 4.
fn place_released(m: &mut MemoriaJogo, v: u16, place: u16) -> Result<()> {
    let n = u32::from(v);
    let rv = unit(m, n)?;
    m.sw(rv + 0x7e, u32::from(place))?;
    m.sw(rv + 0x7c, u32::from(place))?;
    m.sw(rv + 8, u32::from(place))?;
    crate::frame_spawn::release(m, n)?;
    m.sb(rv + 0x88, 0)?;
    if !matches!(m.b(rv + 2)?, 0 | 4) {
        m.sb(rv + 2, 7)?;
        crate::frame_spawn::release(m, n)?;
    }
    m.sw(rv + 0xc0, 0xffff)?;
    m.sw(rv + 0xb8, 0xffff)?;
    m.sb(rv + 0x87, 0)?;
    m.sb(rv + 0x86, 0)?;
    crate::frame_core::call(m, "SetEnginCollision", &[n, m.w(rv + 8)?])?;
    m.sb(rv + 0x72, 1)?;
    sight(m, rv)
}
fn sight(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    let owner = m.b(r)? as u8;
    let cell = m.w(r + 8)? as u16;
    if !matches!(owner, 4 | 200) {
        let radius = m.b(r + 0xbb)? + m.b(r + 0xf5)?;
        crate::player_sight::area(m, cell, (radius + 2) & 0xffff, owner, 0)?;
        crate::player_sight::area(m, cell, radius & 0xffff, owner, 1)?;
    }
    crate::player_sight::minimap(m, owner, cell, 4)?;
    Ok(())
}
/// EnginRetire 0x443dfc, bag tag 1 (0x443e55): a carried unit leaves the
/// transport `member` at the free cell EnginCherchePlace finds around it
/// (the unit index is the passability target). With a dead transport the
/// unit either dies at once (+0xdc bit 2: state 0, selection released) or
/// loses half its life before being placed. A released type-9 unit gets AI
/// message 0x42. A group transport (types 11..=17) left without troops is
/// detached (ResetOrdre, DeselectionEngin, RetireEngin), turns neutral 200
/// and is put back on the map.
fn release_carried(m: &mut MemoriaJogo, member: u16, index: u16, value: u16) -> Result<()> {
    let u = u32::from(member);
    let r = unit(m, u)?;
    let place = crate::spell_remaining::find_place(m, m.w(r + 8)? as u16, value)?;
    if place != 0xffff {
        let v = remove_index(m, r + 10, index)?;
        let n = u32::from(v);
        let r = unit(m, u)?;
        if !matches!(m.b(r + 2)?, 0 | 4) {
            place_released(m, v, place)?;
        } else if crate::frame_core::call(m, "GetBit", &[m.b(unit(m, n)? + 0xdc)?, 2])? as u8 != 0 {
            let rv = unit(m, n)?;
            if !matches!(m.b(rv + 2)?, 0 | 4) {
                m.sb(rv + 2, 0)?;
                crate::frame_spawn::release(m, n)?;
            }
        } else {
            let rv = unit(m, n)?;
            let life = m.w(rv + 0x9a)?;
            m.sw(rv + 0x9a, life.wrapping_sub(life / 2))?;
            place_released(m, v, place)?;
        }
        if m.b(unit(m, n)? + 1)? == 9 {
            crate::frame_ai::call(m, "iagUnitSendMessage", &[n, 0x42, 0])?;
        }
    }
    let r = unit(m, u)?;
    if !(11..=17).contains(&m.b(r + 1)?) {
        return Ok(());
    }
    let bag = r + 10;
    for i in 0..m.w(bag + 2)? {
        if m.b(bag + 0x42 + i)? != 1 {
            continue;
        }
        let carried = unit(m, m.w(bag + 6 + 2 * i)?)?;
        if troop(m.b(carried + 1)?) {
            return Ok(());
        }
    }
    crate::frame_spawn::reset_order(m, u)?;
    crate::player_orders::call(m, "DeselectionEngin", [u, u, u, 0], 0x44442f)?;
    crate::chain_entry::retire(m, member)?;
    m.sb(r, 200)?;
    crate::frame_core::call(m, "SetEnginCollision", &[u, m.w(r + 8)?])?;
    m.sb(r + 0x72, 1)?;
    sight(m, r)
}
pub(crate) fn drop_slot(m: &mut MemoriaJogo, member: u16, index: u16) -> Result<()> {
    drop_one(m, member, index)?;
    Ok(())
}
pub(crate) fn drop_all(m: &mut MemoriaJogo, member: u16) -> Result<()> {
    let bag = unit(m, u32::from(member))? + 10;
    while m.w(bag)? != 0 {
        let count = m.w(bag)?;
        let seed = m.d(0x497164)?;
        let retry_can_select = drop_one(m, member, 0)?;
        if m.w(bag)? == count && (!retry_can_select || seed == m.d(0x497164)?) {
            // Native death loops back to443dfc. If no eligible cell existed in
            // any ring (or the initial accepted cell was the ffff sentinel),
            // the loop cannot remove this item. Keep its exact first-call
            // prefix/RNG as an addressed noncompletion. Ordinal misses with
            // eligible candidates retry normally, consuming the native draws.
            return stop("Death cargo no-removal retry continuation", 0x443dfc);
        }
    }
    Ok(())
}
