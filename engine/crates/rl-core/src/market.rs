//! Market trade and hangar exchange, transcribed from RISING.EXE 0x46027c..0x467640:
//! messenger arrival (BatimentLogeEngin 0x41f807), InitTableauMarche 0x463317,
//! MarcheRemplitTableauHautBas 0x462f88, the AI offer ComposeCommande 0x460341,
//! the proposal screen CalculMarche 0x467557 (zone 0x1a) and tags 23..31.
//!
//! Per-player tables, all at original addresses:
//! 0x4e2e7c state (0 idle, 1/2 waiting for the icon click, 3 proposing, 4 settling);
//! 0x4e2928 "home" side flag, 0x496f8c partner, 0x496f88 give/want page,
//! 0x4e292c accepted flag, 0x496fd0 tab (1 units, 2 goods), 0x4e2930 value,
//! 0x4e2938 and 0x4e2940 the two counters, 0x4e296c ten hangars each;
//! 0x4e29bc/0x4e2a8c units offered/wanted (26 words per player),
//! 0x4e2cfc/0x4e2d14 goods offered/wanted (3 words per player),
//! 0x4e2d2c/0x4e2dfc the available lists, 0x4e2e14 the kinds on sale.
use crate::{
    frame::stop,
    player::{self, structure, unit, Memory},
    MemoriaJogo, Result,
};

const STATE: u32 = 0x4e2e7c;
const HOME: u32 = 0x4e2928;
const PARTNER: u32 = 0x496f8c;
const PAGE: u32 = 0x496f88;
const ACCEPTED: u32 = 0x4e292c;
const TAB: u32 = 0x496fd0;
const VALUE: u32 = 0x4e2930;
const COUNT_A: u32 = 0x4e2938;
const COUNT_B: u32 = 0x4e2940;
const HANGARS: u32 = 0x4e296c;
const GIVE_UNITS: u32 = 0x4e29bc;
const WANT_UNITS: u32 = 0x4e2a8c;
const GIVE_GOODS: u32 = 0x4e2cfc;
const WANT_GOODS: u32 = 0x4e2d14;
const AVAIL_UNITS: u32 = 0x4e2d2c;
const AVAIL_GOODS: u32 = 0x4e2dfc;
const ON_SALE: u32 = 0x4e2e14;
const STOCK: u32 = 0x4e2ea8;
const LOCAL: u32 = 0x508041;
const MOUSE_X: u32 = 0x5099a8;
const MOUSE_Y: u32 = 0x5099aa;
const CLICK: u32 = 0x5099af;
const SCROLL_OWN: u32 = 0x4e290e;
const SCROLL_OTHER: u32 = 0x4e290c;
const WINDOW_X: u32 = 0x4e2906;
const WINDOW_Y: u32 = 0x496f80;
const SYMPATHY_DELTA: u32 = 0x4e2948;
const UNIT_LIST_B: u32 = 0x4e2958;

fn kind_of(m: &MemoriaJogo, u: u32) -> Result<u32> {
    m.b(unit(m, u)? + 1)
}
fn owner_of(m: &MemoriaJogo, u: u32) -> Result<u32> {
    m.b(unit(m, u)?)
}
fn player_type(m: &mut MemoriaJogo, p: u32) -> Result<u32> {
    Ok(crate::frame_economy::call(m, "PlayerGetType", &[p & 255])? & 255)
}
fn allied(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<u32> {
    Ok(crate::frame_economy::call(m, "PlayerEstAllie", &[a & 255, b & 255])? & 255)
}
/// EnginEchangeable 0x440e78.
fn tradable(kind: u32) -> bool {
    let k = kind as u8;
    if k < 0x12 {
        k < 0x10
    } else {
        k > 0x18
    }
}
/// TypeEnginEstEngin 0x442434: the table at 0x442418 sends 11..=17 to true.
fn vehicle(kind: u32) -> bool {
    (11..=17).contains(&(kind & 255))
}
/// PlayerGetSympathie 0x46dc6c.
fn sympathy(m: &MemoriaJogo, a: u32, b: u32) -> Result<i32> {
    let at = m
        .d(0x5080a8)?
        .wrapping_add(20 * (a & 255) + 0xc + 2 * (b & 255));
    Ok(m.w(at)? as u16 as i16 as i32)
}
/// CalculNewSympathie 0x475267.
fn new_sympathy(m: &mut MemoriaJogo, a: u32, b: u32, delta: u32) -> Result<()> {
    if a >= 4 || b >= 4 {
        return Ok(());
    }
    let v = (delta as i32)
        .wrapping_add(sympathy(m, a, b)?)
        .clamp(-50, 50);
    let at = m.d(0x5080a8)?.wrapping_add(20 * a + 0xc + 2 * b);
    m.sw(at, v as u32 & 0xffff)
}
/// EnginSetEtat 0x44115f.
fn set_state(m: &mut MemoriaJogo, u: u32, state: u32) -> Result<()> {
    let r = unit(m, u)?;
    if matches!(m.b(r + 2)?, 0 | 4) {
        return Ok(());
    }
    m.sb(r + 2, state)?;
    if u & 0xffff == m.w(0x496304)? {
        m.sb(0x4e0f36, 0)?;
        m.sb(0x4e0f37, 0)?;
        m.sd(0x4e0f3c, 0)?;
        m.sw(0x496304, 0xffff)?;
    }
    Ok(())
}
fn bag_of(s: u32) -> u32 {
    structure(s) + 0xc
}
fn bag_kind(m: &MemoriaJogo, bag: u32, i: u32) -> Result<u32> {
    m.w(bag + 2 * (i & 0xffff) + 6)
}
fn bag_code(m: &MemoriaJogo, bag: u32, i: u32) -> Result<u32> {
    m.b(bag + (i & 0xffff) + 0x42)
}
/// BagPrend 0x447483.
fn bag_take(m: &mut MemoriaJogo, bag: u32, tag: u32, value: u32) -> Result<()> {
    m.sw(bag + 0x66, m.w(bag + 0x66)?.wrapping_add(1) & 0xffff)?;
    let n = m.w(bag)?;
    m.sb(bag + n + 0x42, tag & 255)?;
    m.sw(bag + 2 * n + 6, value & 0xffff)?;
    m.sw(bag, n.wrapping_add(1) & 0xffff)
}
/// BagRetireInd 0x4473cf.
fn bag_retire_index(m: &mut MemoriaJogo, bag: u32, index: u32) -> Result<u32> {
    let index = index & 0xffff;
    m.sw(bag + 0x66, m.w(bag + 0x66)?.wrapping_sub(1) & 0xffff)?;
    let value = m.w(bag + 2 * index + 6)?;
    let mut i = index;
    loop {
        let last = (m.w(bag)? as i32).wrapping_sub(1);
        if (i as i32) >= last {
            break;
        }
        m.sb(bag + i + 0x42, m.b(bag + i + 0x43)?)?;
        m.sw(bag + 2 * i + 6, m.w(bag + 2 * i + 8)?)?;
        i = (i + 1) & 0xffff;
    }
    m.sb(bag + i + 0x42, 0)?;
    m.sw(bag + 2 * i + 6, 0)?;
    m.sw(bag, m.w(bag)?.wrapping_sub(1) & 0xffff)?;
    Ok(value)
}
/// BagInit 0x447148.
fn bag_init(m: &mut MemoriaJogo, bag: u32, capacity: u32) -> Result<()> {
    m.sw(bag, 0)?;
    m.sw(bag + 0x66, 0)?;
    m.sw(bag + 4, 0)?;
    m.sw(bag + 2, 0)?;
    for i in 0..30 {
        m.sw(bag + 2 * i + 6, 0)?;
        m.sb(bag + i + 0x42, 0)?;
    }
    m.sw(bag + 2, capacity & 0xffff)
}
/// BatimentGetBagOrdre 0x41c734 for table index 8.
fn order_bag(p: u32) -> u32 {
    0x4dd6e8 + 0x8f0 * (p & 255) + 104 * 8
}
fn hangar_at(m: &MemoriaJogo, p: u32, j: u32) -> Result<u32> {
    m.w(HANGARS + 20 * (p & 255) + 2 * j)
}
/// RechercheHangar 0x4611e6.
fn first_hangar(m: &MemoriaJogo, p: u32) -> Result<u32> {
    for j in 0..10 {
        let s = hangar_at(m, p, j)?;
        if s != 0xffff {
            return Ok(s);
        }
    }
    Ok(0xffff)
}
fn credit(m: &mut MemoriaJogo, p: u32, r: u32, amount: u32) -> Result<()> {
    let base = STOCK + 32 * (p & 255) + 8 * r;
    m.sw(base + 2, m.w(base + 2)?.wrapping_add(amount) & 0xffff)?;
    m.sw(base, m.w(base)?.wrapping_add(amount) & 0xffff)
}
fn debit(m: &mut MemoriaJogo, r: u32, amount: u32, p: u32) -> Result<()> {
    crate::chain_restore::debit(m, r as u8, amount as u16, p as u8)
}
/// GetRessource 0x467670: the three stored words, indexed as the callers do.
fn resource(m: &MemoriaJogo, p: u32, r: u32) -> Result<u32> {
    let p = p & 255;
    Ok(match r & 255 {
        0 => m.w(0x4e2eaa + 32 * p)?,
        1 => m.w(0x4e2eb2 + 32 * p)?,
        2 => m.w(0x4e2eba + 32 * p)?,
        _ => 0xffff,
    })
}
/// iagRessGetStockAlerte 0x485ff9 fields, in the order the callers write them.
fn alert(m: &MemoriaJogo, p: u32) -> Result<(i32, i32, i32)> {
    let base = 168 * (p & 255);
    Ok((
        m.d(0x51ba6c + base)? as i32,
        m.d(0x51ba70 + base)? as i32,
        m.d(0x51ba74 + base)? as i32,
    ))
}
fn message(m: &mut MemoriaJogo, code: u16) -> Result<()> {
    crate::messagebox_state::enqueue(m, 0x21, code)
}
fn init_zone(m: &mut MemoriaJogo) -> Result<()> {
    crate::frame_core::call(m, "InitGameZone", &[1, 0])?;
    Ok(())
}
fn set_zone(m: &mut MemoriaJogo, z: u32, x: u32, y: u32, xx: u32, yy: u32) -> Result<()> {
    crate::frame_core::call(m, "MouseSetZone", &[z, x, y, xx, yy])?;
    Ok(())
}

/// MarcheRemplitTableauHautBas 0x462f88.
fn fill_table(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    for i in 0..26 {
        m.sb(ON_SALE + 26 * p + i, 0)?;
        m.sw(AVAIL_UNITS + 52 * p + 2 * i, 0)?;
    }
    for i in 0..26 {
        // TypeEnginEstRealisable 0x44228e.
        if tradable(i) && m.b(0x4e1934 + 26 * p + i)? != 0 && i != 0x19 {
            m.sb(ON_SALE + 26 * p + i, 1)?;
            let a = AVAIL_UNITS + 52 * p + 2 * i;
            m.sw(a, m.w(a)?.wrapping_add(1) & 0xffff)?;
        }
    }
    let table = AVAIL_GOODS + 6 * p;
    if m.b(HOME + p)? == 0 || player_type(m, p)? == 1 {
        m.sw(table, 1)?;
        m.sw(table + 2, 1)?;
        m.sw(table + 4, 1)?;
        return Ok(());
    }
    let (a1, a2, a3) = alert(m, p)?;
    let have = |want: i32, r: u32, m: &MemoriaJogo| -> Result<u32> {
        Ok(u32::from(want.wrapping_sub(resource(m, p, r)? as i32) <= 0))
    };
    let v0 = have(a3, 0, m)?;
    m.sw(table, v0)?;
    let v1 = have(a1, 1, m)?;
    m.sw(table + 2, v1)?;
    let v2 = have(a2, 2, m)?;
    m.sw(table + 4, v2)
}

/// InitTableauMarche 0x463317 (a = proposer, b = market owner).
fn init_table(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<()> {
    let (a, b) = (a & 255, b & 255);
    for p in [a, b] {
        for i in 0..26 {
            m.sw(GIVE_UNITS + 52 * p + 2 * i, 0)?;
            m.sw(WANT_UNITS + 52 * p + 2 * i, 0)?;
        }
        for r in 0..3 {
            m.sw(GIVE_GOODS + 6 * p + 2 * r, 0)?;
            m.sw(WANT_GOODS + 6 * p + 2 * r, 0)?;
        }
    }
    m.sb(ACCEPTED + a, 0)?;
    m.sw(TAB + 2 * a, 1)?;
    m.sb(ACCEPTED + b, 0)?;
    m.sw(TAB + 2 * b, 1)?;
    fill_table(m, a)?;
    fill_table(m, b)?;
    let code = if player_type(m, a)? == 2 {
        m.sb(STATE + a, 2)?;
        m.sb(STATE + b, 2)?;
        m.sb(PAGE + a, 1)?;
        m.sb(ACCEPTED + a, 1)?;
        m.sb(PAGE + b, 1)?;
        m.sb(ACCEPTED + b, 1)?;
        compose_offer(m, a, b)?;
        0x25d
    } else {
        0x25e
    };
    message(m, code)?;
    m.sb(HOME + a, 1)?;
    m.sb(PARTNER + a, b)?;
    m.sb(HOME + b, 0)?;
    m.sb(PARTNER + b, a)?;
    init_zone(m)
}

/// ComposeCommande 0x460341, the AI proposer's offer (a = AI, b = other side).
fn compose_offer(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<()> {
    let random = crate::frame_core::call(m, "Random", &[])?;
    let mut budget = (random % 3) + 1;
    let mode = m.d(0x51ba88 + 168 * a)?;
    if mode != 1 && mode != 3 {
        for e in 0..9u32 {
            let kind = m.w(0x496f94 + 2 * e)?;
            if m.b(kind.wrapping_add(26 * b).wrapping_add(ON_SALE))? == 0 {
                continue;
            }
            if budget == 0 {
                return Ok(());
            }
            dispose(m, a, 1, kind & 255)?;
            let full = m.w(0x496f94 + 2 * e)?;
            m.sb(full.wrapping_add(26 * a).wrapping_add(ON_SALE), 0)?;
            budget -= 1;
            m.sw(AVAIL_UNITS + 52 * a + 2 * full, 0)?;
        }
    }
    for e in 0..4u32 {
        let kind = m.w(0x496fa6 + 2 * e)?;
        if m.b(kind.wrapping_add(26 * b).wrapping_add(ON_SALE))? == 0 {
            continue;
        }
        if budget == 0 {
            break;
        }
        dispose(m, a, 1, kind & 255)?;
        let full = m.w(0x496fa6 + 2 * e)?;
        m.sb(full.wrapping_add(26 * a).wrapping_add(ON_SALE), 0)?;
        budget -= 1;
        m.sw(AVAIL_UNITS + 52 * a + 2 * full, 0)?;
    }
    let (a1, a2, a3) = alert(m, a)?;
    if a1.wrapping_sub(resource(m, a, 1)? as i32) > 0 {
        m.sb(ON_SALE + 26 * a + 5, 0)?;
        m.sw(AVAIL_UNITS + 52 * a + 10, 0)?;
    }
    if a2.wrapping_sub(resource(m, a, 2)? as i32) > 0 {
        m.sb(ON_SALE + 26 * a + 7, 0)?;
        m.sw(AVAIL_UNITS + 52 * a + 14, 0)?;
    }
    let deficit = |want: i32, r: u32, m: &MemoriaJogo| -> Result<i32> {
        let v = want.wrapping_sub(resource(m, a, r)? as i32);
        Ok(if v > 0 { v } else { 0 })
    };
    let mut d0 = deficit(a3, 0, m)?;
    let mut d1 = deficit(a1, 1, m)?;
    let mut d2 = deficit(a2, 2, m)?;
    if d0 > resource(m, b, 0)? as i32 {
        d0 = resource(m, b, 0)? as i32;
    }
    if d1 > resource(m, b, 1)? as i32 {
        d1 = resource(m, b, 1)? as i32;
    }
    if d2 > resource(m, b, 2)? as i32 {
        d2 = resource(m, b, 2)? as i32;
    }
    for (count, r) in [(d0, 0), (d1, 1), (d2, 2)] {
        for _ in 0..count {
            dispose(m, a, 2, r)?;
        }
    }
    Ok(())
}

/// MarcheDebarasse 0x46249d.
fn dispose(m: &mut MemoriaJogo, p: u32, kind: u32, index: u32) -> Result<()> {
    let p = p & 255;
    let index = index & 255;
    let page = m.b(PAGE + p)?;
    match kind & 255 {
        1 => {
            let table = if page == 0 { GIVE_UNITS } else { WANT_UNITS };
            let mut sum = 0u32;
            for i in 0..26 {
                let a = table + 52 * p + 2 * i;
                if m.w(a)? != 0 {
                    sum = sum.wrapping_add(m.b(a)?) & 255;
                }
            }
            if sum >= 8 {
                return Ok(());
            }
            info(m, page, 1, index, 0)?;
            m.sb(0x496f83, index)?;
            m.sb(0x496f84, page)?;
            m.sb(0x496f82, 1)?;
            let a = table + 52 * p + 2 * index;
            m.sw(a, m.w(a)?.wrapping_add(1) & 0xffff)
        }
        2 => {
            info(m, page, 2, index, 0)?;
            m.sb(0x496f83, index)?;
            m.sb(0x496f84, page)?;
            m.sb(0x496f82, 2)?;
            let (a, owner) = if page == 0 {
                (GIVE_GOODS + 6 * p + 2 * index, p)
            } else {
                (WANT_GOODS + 6 * p + 2 * index, m.b(PARTNER + p)?)
            };
            if player_type(m, owner)? == 1 {
                return m.sw(a, m.w(a)?.wrapping_add(1) & 0xffff);
            }
            let held = m.w(a)? as i32;
            if (resource(m, owner, index)? as i32) / 5 > held {
                m.sw(a, m.w(a)?.wrapping_add(1) & 0xffff)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
fn info(m: &mut MemoriaJogo, page: u32, kind: u32, index: u32, group: u32) -> Result<()> {
    m.sb(0x4e2912, page)?;
    m.sb(0x4e2915, index)?;
    m.sb(0x4e2914, 0)?;
    m.sb(0x4e2911, group)?;
    m.sb(0x4e2910, 1)?;
    m.sb(0x4e2913, kind)
}
/// MarcheDebarasseMiniDonne 0x462c65.
fn mini_give(m: &mut MemoriaJogo, p: u32, pos: u32) -> Result<()> {
    let (p, pos) = (p & 255, pos & 255);
    let mut seen = 0u32;
    let page = m.b(PAGE + p)?;
    for i in 0..26 {
        let a = GIVE_UNITS + 52 * p + 2 * i;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        if pos == seen {
            m.sb(0x496f83, i)?;
            m.sb(0x496f82, 1)?;
            m.sb(0x4e2910, 1)?;
            m.sb(0x4e2913, 1)?;
            m.sb(0x4e2911, 1)?;
            m.sb(0x496f84, page)?;
            m.sb(0x4e2912, page)?;
            m.sb(0x4e2915, i)?;
            m.sb(0x4e2914, 0)?;
            m.sw(a, n.wrapping_sub(1) & 0xffff)?;
            let v = AVAIL_UNITS + 52 * p + 2 * i;
            return m.sw(v, m.w(v)?.wrapping_add(1) & 0xffff);
        }
        seen = (seen + 1) & 255;
    }
    for r in 0..3 {
        let a = GIVE_GOODS + 6 * p + 2 * r;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        if pos == seen {
            m.sb(0x496f83, r)?;
            m.sb(0x496f82, 2)?;
            m.sw(a, n.wrapping_sub(1) & 0xffff)?;
            m.sb(0x496f84, page)?;
            m.sb(0x4e2912, page)?;
            m.sb(0x4e2915, r)?;
            let v = AVAIL_GOODS + 6 * p + 2 * r;
            m.sw(v, m.w(v)?.wrapping_add(1) & 0xffff)?;
            m.sb(0x4e2910, 1)?;
            m.sb(0x4e2913, 2)?;
            m.sb(0x4e2911, 1)?;
            return m.sb(0x4e2914, 0);
        }
        seen = (seen + 1) & 255;
    }
    Ok(())
}
/// MarcheDebarasseMiniVeut 0x462d89.
fn mini_want(m: &mut MemoriaJogo, p: u32, pos: u32) -> Result<()> {
    let (p, pos) = (p & 255, pos & 255);
    let mut seen = 0u32;
    let page = m.b(PAGE + p)?;
    for i in 0..26 {
        let a = WANT_UNITS + 52 * p + 2 * i;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        if pos == seen {
            m.sb(0x496f83, i)?;
            m.sb(0x496f82, 1)?;
            m.sb(0x496f84, page)?;
            return m.sw(a, n.wrapping_sub(1) & 0xffff);
        }
        seen = (seen + 1) & 255;
    }
    for r in 0..3 {
        let a = WANT_GOODS + 6 * p + 2 * r;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        if pos == seen {
            m.sb(0x496f83, r)?;
            m.sb(0x496f82, 2)?;
            m.sb(0x4e2913, 2)?;
            m.sb(0x496f84, page)?;
            m.sb(0x4e2912, page)?;
            m.sb(0x4e2914, 0)?;
            m.sb(0x4e2910, 1)?;
            m.sb(0x4e2915, r)?;
            m.sb(0x4e2911, 1)?;
            return m.sw(a, n.wrapping_sub(1) & 0xffff);
        }
        seen = (seen + 1) & 255;
    }
    Ok(())
}
/// The free hangar scan shared by MarcheEchangePrend and MarcheEchangeDonne.
fn free_hangar(m: &MemoriaJogo, owner: u32) -> Result<u32> {
    let half = m.b(0x495be9)? as i32 / 2;
    for j in 0..10 {
        let s = hangar_at(m, owner, j)?;
        if s == 0xffff {
            continue;
        }
        let bag = bag_of(s);
        let n = m.w(bag)?;
        let building_owner = m.b(structure(s) + 4)?;
        let mut foreign = 0u32;
        for i in 0..n {
            if owner_of(m, bag_kind(m, bag, i)?)? != building_owner {
                foreign = (foreign + 1) & 0xffff;
            }
        }
        if (foreign as i32) < half {
            return Ok(s);
        }
    }
    Ok(0xffff)
}
/// RechercheTroupeHangar 0x4612fe: (unit, hangar) or None.
fn find_troop(m: &MemoriaJogo, p: u32, kind: u32) -> Result<Option<(u32, u32)>> {
    for j in 0..10 {
        let s = hangar_at(m, p, j)?;
        if s == 0xffff {
            continue;
        }
        let bag = bag_of(s);
        let n = m.w(bag)?;
        for i in 0..n {
            let u = bag_kind(m, bag, i)?;
            if kind_of(m, u)? == kind & 255 {
                return Ok(Some((u, s)));
            }
        }
    }
    Ok(None)
}
/// BatimentRetireType 0x46153a (s = the hangar holding the unit).
fn retire_type(m: &mut MemoriaJogo, s: u32, u: u32) -> Result<()> {
    let p = m.b(structure(s) + 4)?;
    for j in 0..10 {
        let h = hangar_at(m, p, j)?;
        if h == 0xffff {
            continue;
        }
        let bag = bag_of(h);
        let n = m.w(bag)?;
        for i in 0..n {
            if bag_code(m, bag, i)? == 1 && bag_kind(m, bag, i)? == u & 0xffff {
                bag_retire_index(m, bag, i)?;
            }
        }
    }
    Ok(())
}
fn create(m: &mut MemoriaJogo, cell: u32, kind: u32, owner: u32) -> Result<u32> {
    Ok(crate::frame_spawn::call(m, "CreerEngin", &[cell & 0xffff, 1, kind, owner])? & 0xffff)
}
/// The vehicle-and-driver creation shared by MarcheEchangeDonne and
/// MarcheCreerEngin 0x464a82; the new unit is lodged, set to state 14 and taken
/// into the hangar's bag.
fn create_for(m: &mut MemoriaJogo, s: u32, kind: u32, owner: u32) -> Result<()> {
    let cell = crate::frame_core::building_cell(m, s)?;
    let made = if vehicle(kind) {
        let driver = create(m, cell, 9, owner)?;
        let car = create(m, cell, kind, 0xc8)?;
        crate::spell_remaining::lodge_unit(m, car as u16, driver as u16)?;
        set_state(m, driver, 0xe)?;
        car
    } else {
        create(m, cell, kind, owner)?
    };
    set_state(m, made, 0xe)?;
    bag_take(m, bag_of(s), 1, made)
}
/// MarcheEchangePrend 0x461621.
fn exchange_take(m: &mut MemoriaJogo, p: u32, q: u32) -> Result<()> {
    for i in 0..26 {
        let a = GIVE_UNITS + 52 * q + 2 * i;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        for _ in 0..n {
            let s = free_hangar(m, p)?;
            if s == 0xffff || player_type(m, q)? != 1 {
                continue;
            }
            if let Some((u, from)) = find_troop(m, q, i)? {
                retire_type(m, from, u)?;
                bag_take(m, bag_of(s), 1, u)?;
                m.sw(a, m.w(a)?.wrapping_sub(1) & 0xffff)?;
            }
        }
    }
    for r in 0..3 {
        let a = GIVE_GOODS + 6 * q + 2 * r;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        if player_type(m, q)? == 1 {
            debit(m, r, n, q)?;
        }
        let v = m.w(a)?;
        credit(m, p, r, v)?;
        m.sw(a, 0)?;
    }
    Ok(())
}
/// MarcheEchangeDonne 0x46189a.
fn exchange_give(m: &mut MemoriaJogo, p: u32, q: u32) -> Result<()> {
    for i in 0..26 {
        let a = WANT_UNITS + 52 * q + 2 * i;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        for _ in 0..n {
            let s = free_hangar(m, q)?;
            if s == 0xffff {
                continue;
            }
            match player_type(m, p)? {
                1 => {
                    if let Some((u, from)) = find_troop(m, p, i)? {
                        retire_type(m, from, u)?;
                        bag_take(m, bag_of(s), 1, u)?;
                        m.sw(a, m.w(a)?.wrapping_sub(1) & 0xffff)?;
                    }
                }
                2 => {
                    create_for(m, s, i, p)?;
                    m.sw(a, m.w(a)?.wrapping_sub(1) & 0xffff)?;
                }
                _ => {}
            }
        }
    }
    for r in 0..3 {
        let a = WANT_GOODS + 6 * q + 2 * r;
        let n = m.w(a)?;
        if (n as i32) <= 0 {
            continue;
        }
        if player_type(m, p)? == 1 {
            debit(m, r, n, p)?;
        }
        let v = m.w(a)?;
        credit(m, q, r, v)?;
        m.sw(a, 0)?;
    }
    Ok(())
}
/// MarcheOrdreEchange 0x461bea (tag 23).
fn order_exchange(m: &mut MemoriaJogo, p: u32, q: u32) -> Result<()> {
    let (p, q) = (p & 255, q & 255);
    let toggled = u32::from(m.b(0x4e2968 + p)? == 0);
    m.sb(0x4e2968 + p, toggled)?;
    if m.b(0x4e2968 + q)? == 0 && player_type(m, p)? != 2 {
        return Ok(());
    }
    exchange_take(m, p, q)?;
    exchange_give(m, p, q)?;
    m.sb(0x4e2968 + p, 0)?;
    m.sb(0x4e2968 + q, 0)
}
/// MarcheOrdreSortie 0x461ce5 (tag 25).
fn order_exit(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    for i in 0..26 {
        let n = m.w(GIVE_UNITS + 52 * p + 2 * i)?;
        if (n as i32) <= 0 {
            continue;
        }
        for _ in 0..n {
            let s = first_hangar(m, p)?;
            if s == 0xffff {
                continue;
            }
            let bag = bag_of(s);
            let count = m.w(bag)?;
            let building_owner = m.b(structure(s) + 4)?;
            for j in 0..count {
                let u = bag_kind(m, bag, j)?;
                if owner_of(m, u)? == building_owner && kind_of(m, u)? == i {
                    crate::frame_spawn::call(m, "EnginSetPlayer", &[u, 0xc8])?;
                    let a = GIVE_UNITS + 52 * p + 2 * i;
                    m.sw(a, m.w(a)?.wrapping_sub(1) & 0xffff)?;
                    break;
                }
            }
        }
    }
    Ok(())
}
/// PlayerSetAllie 0x46d877 (the allied byte and the local notice).
fn set_ally(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<()> {
    let (a, b) = (a & 255, b & 255);
    if a != b && a <= 4 && b <= 4 {
        let at = m.d(0x5080a8)?.wrapping_add(20 * a + 6);
        m.sb(at, m.b(at)? | (1 << b))?;
        if a == m.b(LOCAL)? {
            crate::messagebox_state::enqueue(m, 10, b as u16 + 0x18f)?;
        }
    }
    Ok(())
}
/// MarcheChangePlayerContact 0x4637e0 (tag 29). The local toggle-on calls
/// EvenementInit with a null name, a fault in the original (see event_flc::init).
fn change_contact(m: &mut MemoriaJogo, p: u32, other: u32) -> Result<()> {
    let (p, other) = (p & 255, other & 255);
    if allied(m, p, other)? == 0 {
        set_ally(m, p, other)?;
        if p == m.b(LOCAL)? {
            crate::event_flc::init(m, 0)?;
        }
    } else {
        crate::messagebox_state::reset_allie(m, p as u8, other as u8)?;
    }
    Ok(())
}
/// ResetMarche 0x46634e.
fn reset(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<()> {
    for p in [a & 255, b & 255] {
        m.sb(STATE + p, 0)?;
        m.sb(ACCEPTED + p, 1)?;
        m.sb(HOME + p, 0)?;
        m.sb(PARTNER + p, 0xff)?;
        m.sw(VALUE + 2 * p, 0)?;
        m.sw(COUNT_B + 2 * p, 0)?;
        m.sw(COUNT_A + 2 * p, 0)?;
    }
    Ok(())
}
/// Input-side tags 23..31 of ProcessInputGame.
pub fn order(m: &mut MemoriaJogo, p: u32, tag: u32, w: [u32; 5]) -> Result<()> {
    match tag {
        23 => order_exchange(m, p, w[1] & 255),
        24 => {
            let q = m.b(PARTNER + (p & 255))?;
            reset(m, p, q)
        }
        25 => order_exit(m, p),
        26 => dispose(m, p, w[1], w[2]),
        27 => mini_give(m, p, w[1]),
        28 => mini_want(m, p, w[1]),
        29 => change_contact(m, p, w[1]),
        30 => Ok(()),
        31 => m.sb(PAGE + (p & 255), w[1] & 255),
        _ => stop("market order tag", 0x45f27e),
    }
}
/// BatimentLogeEngin 0x41f807: a messenger arriving at another player's market.
pub fn lodge(m: &mut MemoriaJogo, s: u16, u: u16) -> Result<bool> {
    let a = structure(u32::from(s));
    let rec = unit(m, u32::from(u))?;
    let owner = m.b(a + 4)?;
    let visitor = m.b(rec)?;
    if m.b(STATE + owner)? != 0
        || m.b(STATE + visitor)? != 0
        || first_hangar(m, owner)? == 0xffff
        || first_hangar(m, visitor)? == 0xffff
    {
        return Ok(false);
    }
    init_table(m, visitor, owner)?;
    if player_type(m, visitor)? != 2 {
        m.sb(STATE + owner, 1)?;
        m.sb(STATE + visitor, 1)?;
    }
    if m.b(a + 4)? == m.b(LOCAL)? {
        crate::chain_s1::entry(m, "VoixOffAsk", Default::default())?;
    }
    Ok(true)
}
/// ForgetMarche 0x4610a4.
fn forget(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    if player_type(m, p)? != 2 {
        return Ok(());
    }
    let mut staff = false;
    for j in 0..10 {
        let s = hangar_at(m, p, j)?;
        if s == 0xffff {
            continue;
        }
        let bag = bag_of(s);
        let n = m.w(bag)?;
        for _ in 0..n {
            if bag_code(m, bag, 0)? != 1 {
                continue;
            }
            if kind_of(m, bag_kind(m, bag, 0)?)? == 9 && !staff {
                staff = true;
                continue;
            }
            let u = bag_retire_index(m, bag, 1)?;
            set_state(m, u, 0)?;
        }
    }
    Ok(())
}
fn empty_hangar(m: &mut MemoriaJogo, p: u32, list: u32) -> Result<()> {
    let mut staff = false;
    let mut out = 0u32;
    for j in 0..10 {
        let s = hangar_at(m, p, j)?;
        if s == 0xffff {
            continue;
        }
        let bag = bag_of(s);
        let n = m.w(bag)?;
        for _ in 0..n {
            if bag_code(m, bag, 0)? != 1 {
                continue;
            }
            if kind_of(m, bag_kind(m, bag, 0)?)? == 9 && !staff {
                staff = true;
                continue;
            }
            let u = bag_retire_index(m, bag, 1)?;
            let group = m.d(unit(m, u)? + 0x10c)?;
            if group != u32::MAX {
                crate::frame_raid::remove_unit(m, group, u as u16)?;
            }
            m.sw(list + 2 * out, u)?;
            out += 1;
            if out >= 8 {
                return Ok(());
            }
        }
    }
    Ok(())
}
fn fill_hangar(m: &mut MemoriaJogo, p: u32, list: u32) -> Result<()> {
    for j in 0..10 {
        let s = hangar_at(m, p, j)?;
        if s == 0xffff {
            continue;
        }
        let bag = bag_of(s);
        for i in 0..8 {
            let u = m.w(list + 2 * i)?;
            if u != 0xffff {
                let owner = m.b(structure(hangar_at(m, p, j)?) + 4)?;
                crate::frame_spawn::call(m, "EnginSetPlayer", &[u, owner])?;
                bag_take(m, bag, 1, u)?;
            }
        }
    }
    Ok(())
}
/// VideMatiere 0x4663eb.
fn empty_goods(m: &mut MemoriaJogo, a: u32, b: u32) -> Result<()> {
    let (a, b) = (a & 255, b & 255);
    for r in 0..3 {
        if player_type(m, a)? == 1 {
            debit(m, r, m.w(GIVE_GOODS + 6 * a + 2 * r)?, a)?;
        }
        let v = m.w(GIVE_GOODS + 6 * a + 2 * r)?;
        credit(m, b, r, v)?;
    }
    for r in 0..3 {
        if player_type(m, b)? == 1 {
            debit(m, r, m.w(WANT_GOODS + 6 * a + 2 * r)?, b)?;
        }
        let v = m.w(WANT_GOODS + 6 * a + 2 * r)?;
        credit(m, a, r, v)?;
    }
    Ok(())
}
/// TypeEnginTrouverDansHangar 0x4649a7.
fn hangar_has(m: &MemoriaJogo, p: u32, kind: u32, wanted: u32) -> Result<bool> {
    let mut found = 0u32;
    for j in 0..10 {
        let s = hangar_at(m, p, j)?;
        if s == 0xffff {
            continue;
        }
        let bag = bag_of(s);
        let n = m.w(bag)?;
        for i in 0..n {
            if bag_code(m, bag, i)? != 1 {
                continue;
            }
            if kind_of(m, bag_kind(m, bag, i)?)? == kind {
                found = (found + 1) & 0xffff;
                if found == wanted & 0xffff {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
/// MarcheCreerEngin 0x464a82.
fn create_one(m: &mut MemoriaJogo, kind: u32, owner: u32) -> Result<()> {
    let s = first_hangar(m, owner)?;
    if s == 0xffff {
        return Ok(());
    }
    create_for(m, s, kind & 255, owner & 255)
}
/// EnginEstDemander 0x464cfd.
fn unit_requested(m: &mut MemoriaJogo, p: u32, i: u32) -> Result<bool> {
    let (p, i) = (p & 255, i & 255);
    let home = if m.b(HOME + p)? == 1 {
        p
    } else {
        m.b(PARTNER + p)?
    };
    let bag = order_bag(p);
    let mut missing = false;
    if home == p {
        let n = m.w(GIVE_UNITS + 52 * p + 2 * i)?;
        if n != 0 && !hangar_has(m, p, i, n)? {
            missing = true;
            bag_take(m, bag, 6, i)?;
        }
        let want = WANT_UNITS + 52 * home + 2 * i;
        if m.w(want)? != 0 {
            let partner = m.b(PARTNER + p)?;
            create_one(m, i, partner)?;
            let n = m.w(want)?;
            if !hangar_has(m, partner, i, n)? {
                missing = true;
            }
            m.sw(want, m.w(want)?.wrapping_sub(1) & 0xffff)?;
        }
    } else {
        let want = WANT_UNITS + 52 * home + 2 * i;
        let n = m.w(want)?;
        if n != 0 {
            let local = m.b(LOCAL)?;
            if !hangar_has(m, local, i, n)? {
                missing = true;
                bag_take(m, bag, 6, i)?;
            }
        }
        let give = GIVE_UNITS + 52 * home + 2 * i;
        if m.w(give)? != 0 {
            let partner = m.b(PARTNER + p)?;
            create_one(m, i, partner)?;
            let n = m.w(want)?;
            if !hangar_has(m, home, i, n)? {
                missing = true;
            }
            m.sw(give, m.w(give)?.wrapping_sub(1) & 0xffff)?;
        }
    }
    Ok(missing)
}
/// TestRessource 0x4676e5 for the three words of a goods table.
fn can_pay(m: &mut MemoriaJogo, table: u32, p: u32) -> Result<bool> {
    let (v0, v1, v2) = (m.w(table)?, m.w(table + 2)?, m.w(table + 4)?);
    Ok(crate::frame_economy::call(m, "TestRessource", &[p, v1, v2, v0])? & 255 != 0)
}
/// MatiereEstDemander 0x464ba2.
fn goods_requested(m: &mut MemoriaJogo, p: u32) -> Result<bool> {
    let p = p & 255;
    let home = if m.b(HOME + p)? == 1 {
        p
    } else {
        m.b(PARTNER + p)?
    };
    let partner = m.b(PARTNER + home)?;
    let mut short = !can_pay(m, GIVE_GOODS + 6 * home, home)?;
    if !can_pay(m, WANT_GOODS + 6 * home, partner)? {
        short = true;
    }
    Ok(short)
}
/// CalculValeur 0x466533: settles the trade once every requested item exists.
fn settle(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    bag_init(m, order_bag(p), 6)?;
    let mut ready = true;
    for i in 0..26 {
        if unit_requested(m, p, i)? {
            ready = false;
        }
    }
    if goods_requested(m, p)? {
        ready = false;
    }
    if !ready {
        return Ok(());
    }
    let home = if m.b(HOME + p)? == 1 {
        p
    } else {
        m.b(PARTNER + p)?
    };
    let partner = m.b(PARTNER + p)?;
    let delta = m.w(SYMPATHY_DELTA)?;
    let (first, sym_side) = if home == p {
        (home, p)
    } else {
        (m.b(LOCAL)?, m.b(LOCAL)?)
    };
    if delta != 0 {
        new_sympathy(m, partner, sym_side, ((delta as i32) / 5) as u32)?;
    }
    for i in 0..8 {
        m.sw(SYMPATHY_DELTA + 2 * i, 0xffff)?;
        m.sw(UNIT_LIST_B + 2 * i, 0xffff)?;
    }
    empty_hangar(m, first, SYMPATHY_DELTA)?;
    empty_hangar(m, partner, UNIT_LIST_B)?;
    fill_hangar(m, first, UNIT_LIST_B)?;
    fill_hangar(m, partner, SYMPATHY_DELTA)?;
    empty_goods(m, first, partner)?;
    let local = m.b(LOCAL)?;
    empty_goods(m, partner, local)?;
    reset(m, first, partner)?;
    message(m, 0x25b)
}
/// EvalueCoutProposition 0x466bac.
fn evaluate(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    let (mut give, mut want) = (0i32, 0i32);
    for i in 0..26 {
        let price = m.b(0x496fb0 + i)? as i32;
        give = give.wrapping_add((m.w(GIVE_UNITS + 52 * p + 2 * i)? as i32).wrapping_mul(price));
        want = want.wrapping_add((m.w(WANT_UNITS + 52 * p + 2 * i)? as i32).wrapping_mul(price));
    }
    for r in 0..3 {
        let price = m.b(0x496fcc + r)? as i32;
        give = give.wrapping_add((m.w(GIVE_GOODS + 6 * p + 2 * r)? as i32).wrapping_mul(price));
        want = want.wrapping_add((m.w(WANT_GOODS + 6 * p + 2 * r)? as i32).wrapping_mul(price));
    }
    let partner = m.b(PARTNER + p)?;
    let friend =
        |m: &mut MemoriaJogo| -> Result<i32> { Ok(allied(m, partner, p)? as i32 * 40 - 20) };
    let (value, difference) = if player_type(m, p)? != 2 {
        let weight = sympathy(m, partner, p)?
            .wrapping_add(100)
            .wrapping_add(friend(m)?);
        let scaled = give.wrapping_mul(weight) / 100;
        let v = if want != 0 {
            scaled.wrapping_mul(100) / want
        } else if scaled != 0 {
            0xc8
        } else {
            0
        };
        (v, scaled.wrapping_sub(want))
    } else {
        let weight = sympathy(m, p, partner)?
            .wrapping_add(100)
            .wrapping_add(friend(m)?);
        let scaled = want.wrapping_mul(100) / weight;
        let v = if give != 0 {
            scaled.wrapping_mul(100) / give
        } else {
            0x64
        };
        (v, scaled.wrapping_sub(give))
    };
    m.sw(VALUE + 2 * p, value as u32 & 0xffff)?;
    m.sw(SYMPATHY_DELTA, difference as u32 & 0xffff)
}
fn side(m: &MemoriaJogo, p: u32) -> Result<u32> {
    let p = p & 255;
    if m.b(HOME + p)? == 1 {
        Ok(p)
    } else {
        m.b(PARTNER + p)
    }
}
/// CalculCompteurMarche 0x466dd1.
pub fn counter_market(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let s = side(m, p)?;
    if m.b(STATE + s)? != 3 {
        return Ok(());
    }
    if m.b(ACCEPTED + s)? != 0 {
        let period = m.w(0x496f90)?;
        if period == 0 {
            return stop("CalculCompteurMarche period 0", 0x466e1a);
        }
        if m.d(0x508f7c)? % period == 0 {
            m.sw(
                COUNT_A + 2 * s,
                m.w(COUNT_A + 2 * s)?.wrapping_add(1) & 0xffff,
            )?;
        }
        evaluate(m, s)?;
    }
    if m.w(COUNT_A + 2 * s)? != 0x11 {
        return Ok(());
    }
    let code = if m.w(VALUE + 2 * s)? >= 0x64 {
        let other = m.b(PARTNER + s)?;
        m.sb(STATE + s, 4)?;
        m.sb(STATE + other, 4)?;
        0x25a
    } else {
        let other = m.b(PARTNER + s)?;
        reset(m, s, other)?;
        0x259
    };
    message(m, code)
}
/// CalculCompteurProposition 0x466e9b.
pub fn counter_proposal(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    let s = side(m, p)?;
    match m.b(STATE + s)? {
        4 => {
            let period = m.w(0x496f92)?;
            let frame = m.d(0x508f7c)?;
            if period == 0 {
                return stop("CalculCompteurProposition period 0", 0x466ee1);
            }
            if frame % period == 0 {
                m.sw(
                    COUNT_B + 2 * s,
                    m.w(COUNT_B + 2 * s)?.wrapping_add(1) & 0xffff,
                )?;
            }
            if frame % (period << 3) == 0 {
                message(m, 0x25a)?;
            }
            let local = m.b(LOCAL)?;
            settle(m, local)?;
            if m.w(COUNT_B + 2 * s)? <= 0x20 {
                return Ok(());
            }
            forget(m, p)?;
            let partner = m.b(PARTNER + p)?;
            forget(m, partner)?;
            reset(m, p, m.b(PARTNER + p)?)?;
            message(m, 0x25c)?;
            // 0x466f7f: the partner byte was just reset to 0xff, so the two
            // MAJSympathie calls below are the original's bounded no-ops.
            let cleared = m.b(PARTNER + p)?;
            if player_type(m, cleared)? == 2 {
                crate::frame_economy::call(m, "MAJSympathie", &[cleared, p, 0xb])?;
            }
            if player_type(m, p)? == 2 {
                crate::frame_economy::call(m, "MAJSympathie", &[p, cleared, 0xb])?;
            }
            Ok(())
        }
        5 => settle(m, p),
        _ => Ok(()),
    }
}
/// Stockage 0x462870: may this item be added to the panel?
fn can_stock(m: &MemoriaJogo, p: u32, index: u32, kind: u32) -> Result<bool> {
    let p = p & 255;
    let page = m.b(PAGE + p)?;
    let occupied = match kind {
        1 => m.w(AVAIL_UNITS + 52 * p + 2 * index)? != 0,
        2 => {
            let t = if page == 0 { GIVE_GOODS } else { WANT_GOODS };
            m.w(t + 6 * p + 2 * index)? != 0
        }
        _ => false,
    };
    if occupied {
        return Ok(true);
    }
    let (units, goods) = if page == 0 {
        (GIVE_UNITS, GIVE_GOODS)
    } else {
        (WANT_UNITS, WANT_GOODS)
    };
    let mut count = 0u32;
    for i in 0..26 {
        if (m.w(units + 52 * p + 2 * i)? as i32) > 0 {
            count = (count + 1) & 255;
        }
    }
    for r in 0..3 {
        if (m.w(goods + 6 * p + 2 * r)? as i32) > 0 {
            count = (count + 1) & 255;
        }
    }
    Ok(count < 6)
}
fn in_box(mx: u32, my: u32, left: u32, top: u32, right: u32, bottom: u32) -> bool {
    mx as i32 > left as i32
        && (my as i32) > top as i32
        && (mx as i32) < right as i32
        && (my as i32) < bottom as i32
}
fn scroll_click(
    m: &mut MemoriaJogo,
    a: u32,
    tag: u32,
    x_offset: u32,
    y: u32,
    site: u32,
) -> Result<()> {
    let (x, top) = (m.w(WINDOW_X)?, m.w(WINDOW_Y)?);
    let (mx, my) = (m.w(MOUSE_X)?, m.w(MOUSE_Y)?);
    if (mx as i32) <= (x + 0x28) as i32
        || (my as i32) <= (top + y) as i32
        || (mx as i32) >= (x + 0xdc) as i32
        || (my as i32) >= (top + y + 0x18) as i32
        || m.b(CLICK)? == 0
    {
        return Ok(());
    }
    let index = (mx as i32).wrapping_sub((x + x_offset) as i32) / 30;
    player::order(m, a, tag, [index as u32 & 0xffff, 0, 0, 0], site)?;
    Ok(())
}
/// MesEngins 0x46348e and MesEnginsContact 0x46361d.
fn pick_units(m: &mut MemoriaJogo, a: u32, base: u32, other: bool) -> Result<()> {
    let source = if other { m.b(PARTNER + a)? } else { a };
    let scroll = m.w(if other { SCROLL_OTHER } else { SCROLL_OWN })?;
    let (mut shown, mut skipped) = (0u32, 0u32);
    for i in 0..26u32 {
        if m.b(ON_SALE + 26 * source + i)? == 0 || !tradable(i) {
            continue;
        }
        if skipped & 0xffff != scroll {
            skipped = skipped.wrapping_add(1);
        } else {
            let x = (shown + 1) * 6 + shown * 0x38 + m.w(WINDOW_X)?;
            let y = base + m.w(WINDOW_Y)?;
            let (mx, my) = (m.w(MOUSE_X)?, m.w(MOUSE_Y)?);
            if in_box(mx, my, x + 0x21, y, x + 0x59, y + 0x30)
                && m.b(CLICK)? != 0
                && can_stock(m, source, i, 1)?
            {
                player::order(m, a, 0x1a, [1, i, 0, 0], 0x46358b)?;
            }
            shown += 1;
        }
        if shown & 0xffff > 2 {
            break;
        }
    }
    if other {
        scroll_click(m, a, 0x1c, 0x31, 0xb3, 0x4637d3)
    } else {
        scroll_click(m, a, 0x1b, 0x28, 0x6f, 0x463610)
    }
}
/// MesMatiere 0x4668aa and MesMatiereContact 0x466a09.
fn pick_goods(m: &mut MemoriaJogo, a: u32, base: u32, other: bool) -> Result<()> {
    let source = if other { m.b(PARTNER + a)? } else { a };
    let page = m.b(PAGE + a)?;
    let mut shown = 0u32;
    for r in 0..3u32 {
        let stocked = (m.w(AVAIL_GOODS + 6 * source + 2 * r)? as i32) > 0;
        if !(stocked && page == 0) && page != 1 {
            continue;
        }
        let x = (shown + 1) * 6 + shown * 0x37 + m.w(WINDOW_X)?;
        let y = base + m.w(WINDOW_Y)?;
        let (mx, my) = (m.w(MOUSE_X)?, m.w(MOUSE_Y)?);
        if in_box(mx, my, x + 0x21, y, x + 0x59, y + 0x30)
            && m.b(CLICK)? != 0
            && can_stock(m, source, r, 2)?
        {
            player::order(m, a, 0x1a, [2, r, 0, 0], 0x4669b1)?;
            break;
        }
        shown += 1;
    }
    if other {
        scroll_click(m, a, 0x1c, 0x28, 0xb3, 0x466ba7)
    } else {
        scroll_click(m, a, 0x1b, 0x28, 0x6f, 0x466a04)
    }
}
fn clear_zones(m: &mut MemoriaJogo) -> Result<()> {
    for z in 0xc..0x16 {
        set_zone(m, z, 0, 0, 0, 0)?;
    }
    Ok(())
}
fn count_on_sale(m: &MemoriaJogo, source: u32) -> Result<u32> {
    let mut n = 0u32;
    for i in 0..26 {
        if m.b(ON_SALE + 26 * source + i)? != 0 && tradable(i) {
            n += 1;
        }
    }
    Ok(n)
}
/// CalculMarcheProposition 0x4670a8: the proposal screen, once per frame.
fn proposal_screen(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    let (own, other) = if m.b(HOME + p)? == 1 {
        (p, m.b(PARTNER + p)?)
    } else {
        (m.b(PARTNER + p)?, p)
    };
    let middle = (m.w(0x496f7e)? >> 1).wrapping_add(m.w(WINDOW_Y)?);
    m.sb(PAGE + own, u32::from(m.w(MOUSE_Y)? as i32 > middle as i32))?;
    if m.b(PAGE + own)? == 0 {
        match m.w(TAB + 2 * own)? {
            1 => pick_units(m, own, 0x31, false)?,
            2 => pick_goods(m, own, 0x31, false)?,
            _ => {}
        }
    } else if m.b(ACCEPTED + own)? == 0 {
        let partner = m.b(PARTNER + own)?;
        if partner != 0xff {
            match m.w(TAB + 2 * partner)? {
                1 => pick_units(m, own, 0xda, true)?,
                2 => pick_goods(m, own, 0xda, true)?,
                _ => {}
            }
        }
    }
    for e in 0..10u32 {
        let z = 0x496e90 + 24 * e;
        let (id, x, y, w, h) = (
            m.w(z)?,
            m.w(z + 4)?,
            m.w(z + 8)?,
            m.w(z + 0xc)?,
            m.w(z + 0x10)?,
        );
        let left = x.wrapping_add(m.w(WINDOW_X)?) & 0xffff;
        let top = y.wrapping_add(m.w(WINDOW_Y)?) & 0xffff;
        set_zone(
            m,
            id,
            left,
            top,
            left.wrapping_add(w) & 0xffff,
            top.wrapping_add(h) & 0xffff,
        )?;
    }
    let zone = crate::player_input::zone(m)?;
    if m.b(CLICK)? != 0 {
        match zone.wrapping_sub(0xc) & 0xffff {
            0 => {
                m.sw(TAB + 2 * own, 1)?;
                m.sb(0x496ed4, 0)?;
                m.sb(0x496eec, 0)?;
            }
            1 => {
                m.sw(TAB + 2 * own, 2)?;
                m.sb(0x496ed4, 0)?;
                m.sb(0x496eec, 0)?;
            }
            2 => {
                player::order(m, own, 0x18, [0, 0, 0, 0], 0x4672f9)?;
                m.sb(0x496fd8, 0)?;
                init_zone(m)?;
            }
            3 => {
                if m.b(0x496ed4)? != 0 {
                    m.sb(0x496ed5, 1)?;
                    let v = m.w(SCROLL_OWN)?;
                    m.sw(SCROLL_OWN, if v as i32 > 0 { v - 1 } else { v })?;
                }
            }
            4 => {
                if m.b(0x496eec)? != 0 {
                    m.sb(0x496eed, 1)?;
                    let v = m.w(SCROLL_OWN)?;
                    let n = count_on_sale(m, own)?;
                    let next = if ((n & 0xffff).wrapping_sub(3) as i32) > v as i32 {
                        v + 1
                    } else {
                        v
                    };
                    m.sw(SCROLL_OWN, next & 0xffff)?;
                }
            }
            5 => {
                if m.b(0x496f04)? != 0 {
                    m.sb(0x496f05, 1)?;
                    let v = m.w(SCROLL_OTHER)?;
                    m.sw(SCROLL_OTHER, if v as i32 > 0 { v - 1 } else { v })?;
                }
            }
            6 => {
                if m.b(0x496f1c)? != 0 {
                    m.sb(0x496f1d, 1)?;
                    let v = m.w(SCROLL_OTHER)?;
                    let n = count_on_sale(m, other)?;
                    let next = if ((n & 0xffff).wrapping_sub(3) as i32) > v as i32 {
                        v + 1
                    } else {
                        v
                    };
                    m.sw(SCROLL_OTHER, next & 0xffff)?;
                }
            }
            7 => {
                let partner = m.b(PARTNER + own)?;
                m.sb(0x496f04, 0)?;
                m.sb(0x496f1c, 0)?;
                m.sw(TAB + 2 * partner, 1)?;
            }
            8 => {
                let partner = m.b(PARTNER + own)?;
                m.sb(0x496f04, 0)?;
                m.sb(0x496f1c, 0)?;
                m.sw(TAB + 2 * partner, 2)?;
            }
            9 => {
                if m.b(ACCEPTED + own)? == 0 {
                    m.sb(ACCEPTED + own, 1)?;
                } else if m.w(VALUE + 2 * own)? >= 0x64 {
                    for (from, to) in [(GIVE_UNITS, 0x4e2b5c), (WANT_UNITS, 0x4e2c2c)] {
                        let bytes = m.read(from, 0xd0)?.to_vec();
                        m.write(to, &bytes)?;
                    }
                    m.sb(STATE + own, 4)?;
                    let partner = m.b(PARTNER + own)?;
                    m.sb(STATE + partner, 4)?;
                    init_zone(m)?;
                }
            }
            _ => {}
        }
    }
    clear_zones(m)
}
/// InputGame zone 0x1a, 0x42bbff: dragging the panel, then CalculMarche.
pub fn input_zone(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    if m.b(0x5099b1)? != 0 {
        m.sw(0x49a010, 0x1e0b)?;
        if m.b(0x4e05a9)? == 0 {
            m.sb(0x4e05a9, 1)?;
            let (x, y) = (m.w(MOUSE_X)?, m.w(MOUSE_Y)?);
            m.sw(0x4e2908, m.w(WINDOW_X)?.wrapping_sub(x) & 0xffff)?;
            m.sw(0x4e290a, m.w(WINDOW_Y)?.wrapping_sub(y) & 0xffff)?;
        } else {
            let (x, y) = (m.w(MOUSE_X)?, m.w(MOUSE_Y)?);
            set_pos(m, x, y)?;
            init_zone(m)?;
        }
    } else {
        m.sb(0x4e05a9, 0)?;
    }
    calcul_marche(m, p)
}
/// MarcheSetPos 0x462082.
pub fn set_pos(m: &mut MemoriaJogo, x: u32, y: u32) -> Result<()> {
    let x = x.wrapping_add(m.w(0x4e2908)?) & 0xffff;
    let y = y.wrapping_add(m.w(0x4e290a)?) & 0xffff;
    m.sw(WINDOW_X, if x & 0x8000 != 0 { 0 } else { x })?;
    m.sw(WINDOW_Y, if y & 0x8000 != 0 { 0 } else { y })?;
    recenter(m)
}
/// MarcheRecentre 0x461efc.
fn recenter(m: &mut MemoriaJogo) -> Result<()> {
    let margin = (m.b(0x508f90)? as u8 as i8 as i32) << 5;
    for (pos, span, size) in [
        (WINDOW_X, 0x508724, 0x496f7c),
        (WINDOW_Y, 0x508726, 0x496f7e),
    ] {
        let value = m.w(pos)? as i32;
        if value < margin {
            m.sw(pos, margin as u32 & 0xffff)?;
        } else {
            let limit = ((m.w(span)? << 5) as i32).wrapping_sub(margin);
            if value + m.w(size)? as i32 + 2 > limit {
                let v = ((m.w(span)? << 5) as i32)
                    .wrapping_sub(margin)
                    .wrapping_sub(m.w(size)? as i32)
                    .wrapping_sub(2);
                m.sw(pos, v as u32 & 0xffff)?;
            }
        }
    }
    for (pos, span, max) in [
        (0x508720, 0x508724, 0x497168),
        (0x508722, 0x508726, 0x49716a),
    ] {
        let (a, b, c) = (m.w(pos)?, m.w(span)?, m.w(max)?);
        if a + b >= c {
            let excess = a.wrapping_add(b).wrapping_sub(c);
            m.sw(pos, a.wrapping_sub(excess) & 0xffff)?;
        }
    }
    Ok(())
}
/// CalculMarche 0x467557.
pub fn calcul_marche(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let p = p & 255;
    let s = side(m, p)?;
    match m.b(STATE + s)? {
        1 | 2 => {
            if m.b(CLICK)? != 0 && crate::player_input::zone(m)? == 0x1a {
                m.sb(STATE + s, 3)?;
                m.sb(STATE + p, 3)?;
                m.sb(0x496fd8, 1)?;
                init_zone(m)?;
            }
            Ok(())
        }
        3 => proposal_screen(m, p),
        _ => Ok(()),
    }
}
/// The writes of AffMarche 0x46677d the game reads back, applied at the start of
/// the next frame: the same point in the frame order as the original's end-of-frame
/// draw, so the draw of this frame still sees the selection highlight.
pub fn display_tick(m: &mut MemoriaJogo) -> Result<()> {
    let p = m.b(LOCAL)?;
    match m.b(STATE + p)? {
        1 | 2 => init_zone(m),
        3 => {
            let h = side(m, p)?;
            let own = u32::from(m.w(TAB + 2 * p)? == 1 && count_on_sale(m, p)? > 3);
            let (first, second) = if p == h {
                (0x496ed4, 0x496eec)
            } else {
                (0x496f04, 0x496f1c)
            };
            m.sb(first, own)?;
            m.sb(second, own)?;
            let partner = m.b(PARTNER + p)?;
            let other = u32::from(m.w(TAB + 2 * partner)? == 1 && count_on_sale(m, partner)? > 3);
            let (first, second) = if p != h {
                (0x496ed4, 0x496eec)
            } else {
                (0x496f04, 0x496f1c)
            };
            m.sb(first, other)?;
            m.sb(second, other)?;
            number_tick(m, h, GIVE_UNITS, GIVE_GOODS, 0)?;
            number_tick(m, h, WANT_UNITS, WANT_GOODS, 1)?;
            m.sb(0x496f82, 0xff)?;
            m.sb(0x496f83, 0xff)
        }
        4 => m.sw(0x49a018, 0x320),
        _ => Ok(()),
    }
}
/// The 30-frame number overlay of AffMiniEmplacementsDonne and Veut.
fn number_tick(m: &mut MemoriaJogo, h: u32, units: u32, goods: u32, page: u32) -> Result<()> {
    if m.b(PAGE + h)? != page {
        return Ok(());
    }
    let step = |m: &mut MemoriaJogo, kind: u32, item: u32, count: u32| -> Result<()> {
        if count == 0
            || m.b(0x4e2910)? == 0
            || m.b(0x4e2911)? != 0
            || m.b(0x4e2913)? != kind
            || m.b(0x4e2915)? != item
        {
            return Ok(());
        }
        let c = m.b(0x4e2914)?.wrapping_add(1) & 255;
        m.sb(0x4e2914, c)?;
        if c > 0x1e {
            for a in [0x4e2910, 0x4e2913, 0x4e2915, 0x4e2914, 0x4e2911] {
                m.sb(a, 0)?;
            }
        }
        Ok(())
    };
    for i in 0..26 {
        let n = m.w(units + 52 * h + 2 * i)?;
        step(m, 1, i, n)?;
    }
    for r in 0..3 {
        let n = m.w(goods + 6 * h + 2 * r)?;
        step(m, 2, r, n)?;
    }
    Ok(())
}
