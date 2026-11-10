//! Economy cave ownership and technology, pinned54d148d.
use crate::{
    frame::stop,
    player::{structure, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
const RADII: [u32; 22] = [
    20, 8, 10, 8, 5, 8, 8, 8, 8, 0, 7, 7, 0, 8, 8, 8, 8, 4, 8, 4, 8, 4,
];
fn low(v: u32, b: u32) -> u32 {
    (v & !255) | (b & 255)
}
pub(crate) fn level(m: &mut MemoriaJogo, p: u32, site: u32) -> Result<(u32, u32, u32)> {
    let a = 0x4e2ea8 + 32 * p;
    let n = m.w(0x4dfb2c + 6 * p)?;
    let d = m.w(0x4dfb2e + 6 * p)?;
    if n == 0 || d == 0 {
        return stop("cave stock indicator division", site);
    }
    let food = m.w(a + 10)?.wrapping_sub(m.w(a + 8)?) & 65535;
    let stone = m.w(a + 18)?.wrapping_sub(m.w(a + 16)?) & 65535;
    let q = (((food + stone) & 65535) / n) * 7;
    let v = q / d;
    let rem = q % d;
    m.sb(0x4dfb30 + 6 * p, v)?;
    Ok((v, rem, d))
}
/// TerrainDeposeMatiere 0x41bb8a with the flag argument 0 (its only callers):
/// GetRessourceStock 0x46769d reads the player's stock (resource 0 = food
/// words +8/+10, 1 = stone +16/+18); with caves (0x4dfb2c) half of the
/// per-cave share is debited (EnleveMatiere 0x46792c) and the cave indicator
/// recomputed, without caves the whole stock is debited; TerrainDeposeSac
/// 0x4761a9 then drops that many sacks of resource 1 (food), or half as many
/// of resource 2 (stone), at `cell`.
fn depose_matiere(m: &mut MemoriaJogo, p: u32, cell: u32, resource: u32) -> Result<()> {
    let a = 0x4e2ea8 + 32 * p + 8 + 8 * resource;
    let amount = m.w(a + 2)?.wrapping_sub(m.w(a)?) & 65535;
    let mut sacks = amount;
    if amount != 0 {
        let caves = m.w(0x4dfb2c + 6 * p)?;
        let debit = amount.checked_div(caves).map_or(amount, |share| share >> 1);
        // EnleveMatiere: both words lose the amount, clamped at zero.
        for off in [2, 0] {
            m.sw(a + off, m.w(a + off)?.saturating_sub(debit))?;
        }
        sacks = debit;
        if caves != 0 {
            level(m, p, if resource == 0 { 0x41bccf } else { 0x41be27 })?;
        }
    }
    if resource == 1 {
        sacks /= 2;
    }
    for _ in 0..sacks {
        crate::chain_cargo::deposit(m, cell as u16, 1 + resource as u8)?;
    }
    Ok(())
}
/// SortilegeSetTechnologie 0x474a38: the owner's per-type Bag 4 lists every learned
/// spell (flags 0x508f30 + 5 * owner), tag 4.
pub(crate) fn spell_bag(m: &mut MemoriaJogo, owner: u32) -> Result<()> {
    let bag = m
        .d(m.d(0x4e19b8)?.wrapping_add(4 * owner))?
        .wrapping_add(104 * 4);
    bag_init(m, bag, 5)?;
    for i in 0..5 {
        if m.b(0x508f30 + 5 * owner + i)? != 0 {
            bag_take(m, bag, 4, i)?;
        }
    }
    Ok(())
}

pub(crate) fn bag_init(m: &mut MemoriaJogo, a: u32, cap: u32) -> Result<()> {
    for off in [0, 0x66, 4, 2] {
        m.sw(a + off, 0)?
    }
    for i in 0..30 {
        m.sw(a + 6 + 2 * i, 0)?;
        m.sb(a + 0x42 + i, 0)?
    }
    m.sw(a + 2, cap)
}
pub(crate) fn bag_take(m: &mut MemoriaJogo, a: u32, tag: u32, v: u32) -> Result<u32> {
    m.sw(a + 0x66, m.w(a + 0x66)?.wrapping_add(1))?;
    let n = m.w(a)?;
    m.sb(a + 0x42 + n, tag)?;
    m.sw(a + 6 + 2 * n, v)?;
    m.sw(a, n.wrapping_add(1))?;
    Ok(n)
}
/// Complete intact BatimentSetTechnologie 0x41c76c. AL owner, DL type; the
/// owner's per-type Bag is re-initialised and filled in native order. Units
/// use tag 6 behind TypeEnginEstRealisable; type 6 lists known, unapplied
/// inventions with tag 4; type 15 adds tag 3 value 20 behind the network byte;
/// type 20 adds tag 4 value 5 unconditionally. Scratch EAX/EDX follow the
/// last native leaf, as the existing types 2/3 already did.
pub fn technology(m: &mut MemoriaJogo, p: u32, t: u32) -> Result<Option<(u32, Option<u32>)>> {
    if p >= 4 {
        return Ok(None);
    }
    if !matches!(t, 1 | 2 | 3 | 5 | 6 | 13 | 15 | 16 | 18 | 20) {
        // Types without a branch leave the owner's Bag base in EAX and EDX intact.
        return Ok(Some((0x4dd6e8 + 0x8f0 * p, None)));
    }
    let bag = 0x4dd6e8 + 0x8f0 * p + 104 * t;
    let cap = match t {
        1 | 2 => 4,
        3 => 3,
        5 | 6 => 5,
        13 | 15 | 20 => 2,
        _ => 1,
    };
    let mut eax = cap;
    let mut edx = 30;
    bag_init(m, bag, cap)?;
    let units: &[u32] = match t {
        1 => &[0, 1, 2, 8],
        2 => &[5, 6, 7],
        5 => &[11, 12, 13, 14, 15],
        13 => &[3, 25],
        15 | 20 => &[4],
        16 => &[10],
        18 => &[16],
        _ => &[],
    };
    for &u in units {
        eax = m.b(0x4e1934 + 26 * p + u)?;
        edx = 26 * p;
        if eax & 255 != 0 {
            edx = bag_take(m, bag, 6, u)?;
            eax = bag
        }
    }
    match t {
        6 => {
            for i in 0..5 {
                edx = 5 * p;
                eax = m.b(0x508f44 + 5 * p + i)?;
                if eax & 255 == 0 {
                    continue;
                }
                eax = m.b(0x508f30 + 5 * p + i)?;
                if eax & 255 != 0 {
                    continue;
                }
                edx = bag_take(m, bag, 4, i)?;
                eax = bag
            }
        }
        15 => {
            // ReseauActif 0x42c3b0 only reads 0x4e0730; its AL is discarded.
            let flag = m.b(0x4dfad4 + 22 * p + 20)?;
            eax = ((22 * p + 20) & !255) | flag;
            if flag != 0 {
                edx = bag_take(m, bag, 3, 20)?;
                eax = bag
            }
        }
        20 => {
            edx = bag_take(m, bag, 4, 5)?;
            eax = bag
        }
        _ => {}
    }
    Ok(Some((eax, Some(edx))))
}
/// BallonInit 0x441baa (AL=player). Record 0x4e1a14+8p: +0 count, +4 unit,
/// +6 start column. Without a balloon unit yet, the column is Random % width,
/// a type-17 unit is created there (message 0x45), placed with discovery and
/// SetMiniMap 4; the balloon then heads to the player's first live type-2
/// structure, or two rows below its first type-18 one. The count always rises.
pub(crate) fn balloon_init(m: &mut MemoriaJogo, player: u32) -> Result<()> {
    let record = 0x4e1a14 + 8 * player;
    if m.w(record)? == 0 {
        let mut unit = m.w(record + 4)?;
        if unit == 0xffff {
            let width = m.w(0x497168)?;
            if width == 0 {
                return stop("BallonInit map width division", 0x441bf7);
            }
            let column = crate::frame_core::random(m)? % width;
            m.sw(record + 6, column)?;
            unit = crate::frame_spawn::call(m, "CreerEngin", &[column, 1, 0x11, player])? & 65535;
            crate::frame_ai::call(m, "iagUnitSendMessage", &[unit, 0x45, 0])?;
            let u = unit_record(m, unit)?;
            let cell = m.w(u + 8)?;
            crate::frame_core::call(m, "SetEnginCollision", &[unit, cell])?;
            m.sb(u + 0x72, 1)?;
            let owner = m.b(u)?;
            if !matches!(owner, 200 | 4) {
                let sight = m.b(u + 0xbb)? + m.b(u + 0xf5)?;
                crate::player_sight::area(m, cell as u16, (sight + 2) & 65535, owner as u8, 0)?;
                crate::player_sight::area(m, cell as u16, sight & 65535, owner as u8, 1)?;
            }
            crate::frame_spawn::minimap(m, owner as u8, cell as u16, 4)?;
        }
        let mut target = first_live_structure(m, 2, player)?;
        let cell = if target == 0xffff {
            target = first_live_structure(m, 18, player)?;
            if target == 0xffff {
                // 0x441d0d: the original calls BatimentGetCase(0xffff) and reads
                // the record beyond the structure table (0x41af07, process heap
                // in the real game, unmapped on the bench): no reproducible
                // destination exists, so this is an explicit stop, not a guess.
                return stop(
                    "BallonInit destination: no live type-2/type-18 structure of the player",
                    0x41af07,
                );
            }
            crate::frame_core::building_cell(m, target)?.wrapping_add(0x200) & 65535
        } else {
            crate::frame_core::building_cell(m, target)? & 65535
        };
        crate::frame_spawn::order(m, unit, cell as u16)?;
        m.sw(record + 4, unit)?;
    }
    m.sw(record, m.w(record)?.wrapping_add(1))
}
/// BallonPrepareSortie 0x441d4f (AL=player): the last heliport gone sends the
/// balloon away. Count 0x4e1a14 + 8p drops; at zero the balloon (+4) is set to
/// state 30 unless dead/empty (selection released), told to leave for its
/// recorded column (0x4e1a1a + 8p) and receives AI message 0x44.
pub(crate) fn balloon_prepare_exit(m: &mut MemoriaJogo, player: u32) -> Result<()> {
    let record = 0x4e1a14 + 8 * player;
    let count = m.w(record)?;
    if count == 0 {
        return Ok(());
    }
    let count = count.wrapping_sub(1) & 65535;
    m.sw(record, count)?;
    if count != 0 {
        return Ok(());
    }
    let unit = m.w(record + 4)?;
    if unit == 0xffff {
        return stop("BallonPrepareSortie without a balloon unit", 0x441d83);
    }
    let u = unit_record(m, unit)?;
    if !matches!(m.b(u + 2)?, 0 | 4) {
        m.sb(u + 2, 30)?;
        if unit == m.w(0x496304)? {
            m.sb(0x4e0f36, 0)?;
            m.sb(0x4e0f37, 0)?;
            m.sd(0x4e0f3c, 0)?;
            m.sw(0x496304, 0xffff)?;
        }
    }
    let dest = m.w(record + 6)?;
    crate::frame_spawn::call(m, "SetDeplaceEngin", &[unit, dest])?;
    crate::frame_ai::call(m, "iagUnitSendMessage", &[unit, 0x44, 0])?;
    Ok(())
}
fn unit_record(m: &MemoriaJogo, unit: u32) -> Result<u32> {
    Ok(m.d(0x4e19a0)?.wrapping_add(276 * (unit & 65535)))
}
/// 0x45d864: first live structure (state != 0) of a type owned by a player.
fn first_live_structure(m: &MemoriaJogo, kind: u32, owner: u32) -> Result<u32> {
    for s in 0..1000u32 {
        let b = structure(s);
        if m.b(b + 5)? != 0 && m.b(b + 8)? == kind && m.b(b + 4)? == owner {
            return Ok(s);
        }
    }
    Ok(0xffff)
}
pub fn habitable(m: &mut MemoriaJogo, s: u32) -> Result<u32> {
    let e = structure(s);
    let t = m.b(e + 8)?;
    let p = m.b(e + 4)?;
    technology(m, p, t)?;
    let mut eax = t;
    if !matches!(t, 17 | 9 | 12) {
        if t == 4 {
            m.sw(e + 0x78, 0)?
        } else if t == 3 && p < 4 && m.w(0x4dfb2c + 6 * p)? != 0 {
            level(m, p, 0x41cd55)?;
        }
        if t > 21 {
            return stop("BatimentHabitable", 0x41cd62);
        }
        let cell = crate::frame_core::building_cell(m, s)?;
        let radius = RADII[t as usize];
        crate::player_sight::area(m, cell as u16, radius + 3, p as u8, 0)?;
        crate::player_sight::area(m, cell as u16, radius, p as u8, 1)?;
        // entrada_estruturas.illumine_terrain: sign-extended row bound DX + 1.
        eax = (radius.wrapping_add(1) as u16 as i16 as i32) as u32;
    }
    m.sw(e + 0x84, 200)?;
    Ok(eax)
}
pub fn own(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    match name {
        "BatimentHabitable" => r.eax = habitable(m, r.eax & 65535)?,
        "BatimentSetTechnologie" => {
            if let Some((a, d)) = technology(m, r.eax & 255, r.edx & 255)? {
                r.eax = a;
                if let Some(d) = d {
                    r.edx = d
                }
            }
        }
        "TypeEnginEstRealisable" => {
            let p = r.eax & 255;
            let t = r.edx & 255;
            r.eax = m.b(0x4e1934 + 26 * p + t)?;
            r.edx = 26 * p
        }
        "EnginGetTimeRecrute" => r.eax = m.w(0x4965ae + 50 * (r.eax & 255))?,
        "EnginGetRessource" => {
            let t = r.eax & 255;
            let a = 0x4965ae + 50 * t;
            let (food, parts, stone) = (m.w(a + 2)?, m.w(a + 4)?, m.w(a + 6)?);
            m.sw(r.edx, food)?;
            m.sw(r.ebx, stone)?;
            m.sw(r.ecx, parts)?;
            r.eax = parts;
            r.ebx = t;
            r.edx = (r.edx & 0xffff0000) | stone
        }
        "BatimentGetCapacite" => {
            let t = m.b(structure(r.eax) + 8)?;
            let mut c = m.b(0x495d8c + 52 * t + 18)?;
            if t == 14 {
                c = c.wrapping_sub(1)
            } else if t == 19 {
                c = c.wrapping_sub(m.b(0x495be8)?)
            }
            r.eax = low(13 * t, c)
        }
        "InitBatimentPlayer" => {
            let e = structure(r.eax);
            let p = r.edx & 255;
            let t = m.b(e + 8)?;
            r.eax = (r.eax & 0xffff0000) | (p << 8) | t;
            r.edx = e;
            if t == 18 {
                // 0x41bec6: BallonInit for the player, then the owner's balloon
                // count against its limit decides a fresh type-18 Bag with unit 16.
                balloon_init(m, p)?;
                m.sb(0x4dfb4c + p, m.b(0x4dfb4c + p)?.wrapping_add(1))?;
                let owner = m.b(e + 4)?;
                let count = m.b(0x4dfb4c + owner)?;
                let limit = m.w(0x4e1a3c + 52 * owner + 32)?;
                if count as i32 > limit as i32 {
                    let bag = 0x4dd6e8 + 0x8f0 * p + 104 * 18;
                    if crate::chain_bags::body(
                        m,
                        "BagContient",
                        R {
                            eax: bag,
                            edx: 6,
                            ebx: 0x10,
                            ..Default::default()
                        },
                    )?
                    .eax & 65535
                        == 0
                    {
                        bag_init(m, bag, 1)?;
                        bag_take(m, bag, 6, 0x10)?;
                    }
                }
            } else if t == 6 {
                m.sb(0x4dfb48 + p, m.b(0x4dfb48 + p)?.wrapping_add(1))?;
                r.eax = p
            } else if t == 3 {
                let a = 0x4dfb2c + 6 * p;
                let n = m.w(a)?.wrapping_add(1) & 65535;
                m.sw(a, n)?;
                r.eax = p;
                if p < 4 {
                    if n == 0 {
                        r.edx = 0
                    } else {
                        (r.eax, r.edx, _) = level(m, p, 0x41bfbe)?
                    }
                }
            }
        }
        "InitBatimentNeutre" => {
            let s = r.eax & 65535;
            let e = structure(s);
            let t = m.b(e + 8)?;
            let p = r.edx & 255;
            r.eax = low(r.eax, t);
            if t == 18 {
                // 0x41c05d: the balloon leaves, the heliport count drops, and
                // while the owner is back within its limit the type-18 recruit
                // Bag is rebuilt (capacity 1) without unit 16.
                balloon_prepare_exit(m, p)?;
                m.sb(0x4dfb4c + p, m.b(0x4dfb4c + p)?.wrapping_sub(1) & 255)?;
                let owner = m.b(e + 4)?;
                let count = m.b(0x4dfb4c + owner)?;
                let limit = m.w(0x4e1a3c + 52 * owner + 32)?;
                if count as i32 <= limit as i32 {
                    let bag = 0x4dd6e8 + 0x8f0 * p + 104 * 18;
                    bag_init(m, bag, 1)?;
                    crate::chain_bags::body(
                        m,
                        "BagRetire",
                        R {
                            eax: bag,
                            edx: 6,
                            ebx: 0x10,
                            ..Default::default()
                        },
                    )?;
                }
            } else if t == 20 {
                m.sb(e + 0x7a, 0)?;
                m.sb(e + 0x104, 0)?;
                m.sd(e + 0x108, 1)?
            } else if t == 6 {
                let n = m.b(0x4dfb48 + p)?.wrapping_sub(1) & 255;
                m.sb(0x4dfb48 + p, n)?;
                if n == 0 {
                    // 0x41c152 SortilegeResetRealisable 0x4749ca: the last library is gone,
                    // so the player's five spell flags are cleared; 0x41c159
                    // SortilegeSetTechnologie 0x474a38 then rebuilds the spell Bag (empty).
                    for i in 0..5 {
                        m.sb(0x508f30 + 5 * p + i, 0)?;
                    }
                    spell_bag(m, p)?;
                }
                r.eax = n;
                r.edx = low(r.edx, n)
            } else if t == 3 {
                let a = 0x4dfb2c + 6 * p;
                m.sw(a, m.w(a)?.wrapping_sub(1))?;
                if m.b(e + 5)? == 3 {
                    r.eax = 3
                } else {
                    // 0x41c0ef / 0x41c10f: TerrainDeposeMatiere for food, then stone, of
                    // player [0x4dfab8] two rows/columns inside the cave (flag 0).
                    let depositor = m.b(0x4dfab8)?;
                    let cell = (m.w(e + 2)? + 0x202) & 65535;
                    depose_matiere(m, depositor, cell, 0)?;
                    depose_matiere(m, depositor, cell, 1)?;
                    r.eax = cell;
                    r.edx = 2;
                }
            }
        }
        _ => return stop("unported cave owner entry", 0x41cc70),
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
