//! Full Economy CalculTroupe/SortieTroupe continuation, including abandonment.
use crate::{
    frame::stop,
    player::{structure, unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
const RADII: [u32; 22] = [
    20, 8, 10, 8, 5, 8, 8, 8, 8, 0, 7, 7, 0, 8, 8, 8, 8, 4, 8, 4, 8, 4,
];
fn bag(m: &mut MemoriaJogo, n: &str, b: u32, tag: u32, v: u32) -> Result<u32> {
    Ok(crate::chain_bags::body(
        m,
        n,
        R {
            eax: b,
            edx: tag,
            ebx: v,
            ..Default::default()
        },
    )?
    .eax)
}
pub fn init(m: &mut MemoriaJogo, t: u32, p: u32) -> Result<bool> {
    let s = m.w(0x495be0 + 2 * p)?;
    let a = structure(s);
    let r = 0x4965ae + 50 * t;
    let (food, parts, stone) = (m.w(r + 2)?, m.w(r + 4)?, m.w(r + 6)?);
    if !crate::player_orders::resources(m, p, food, stone, parts)?
        || !matches!(m.b(a + 8)?, 1 | 2 | 5 | 13 | 15 | 16 | 18 | 20)
        || m.b(a + 0x74)? != 0
    {
        return Ok(false);
    }
    for (resource, qty) in [(1, food), (2, stone), (0, parts)] {
        crate::chain_restore::debit(m, resource, qty as u16, p as u8)?;
    }
    m.sb(a + 0x74, 1)?;
    m.sb(a + 0x7a, 0)?;
    m.sb(a + 5, 4)?;
    m.sd(a + 0x8c, 1)?;
    m.sb(a + 0x7b, t)?;
    m.sw(a + 0x7e, 0)?;
    if m.b(a + 8)? != 20 {
        m.sw(a + 0x80, 0)?;
        m.sw(a + 0x82, 0)?;
    }
    let c = crate::chain_vision::exit_cell(m, m.w(0x495be0 + 2 * p)?)?;
    m.sw(a + 0x7c, u32::from(c))?;
    Ok(true)
}
pub fn reset(m: &mut MemoriaJogo, a: u32) -> Result<()> {
    m.sw(a + 0x78, 65535)?;
    m.sb(a + 0x7b, 255)?;
    m.sw(a + 0x7c, 0)?;
    m.sw(a + 0x7e, 0)?;
    m.sw(a + 0x86, 65535)?;
    if m.b(a + 8)? != 20 {
        m.sw(a + 0x80, 0)?;
        m.sw(a + 0x82, 0)?;
    }
    m.sb(a + 0x74, 0)?;
    m.sd(a + 0x8c, 0)?;
    if m.b(a + 5)? != 3 {
        m.sb(a + 5, 5)?;
    }
    Ok(())
}
pub fn abandon(m: &mut MemoriaJogo, s: u32, neuter: bool) -> Result<()> {
    let a = structure(s);
    let t = m.b(a + 8)?;
    if !neuter && matches!(t, 1 | 2 | 5) {
        reset(m, a)?;
    }
    let p = m.b(a + 4)?;
    if !matches!(t, 9 | 12) {
        if t > 21 {
            return stop("SortieTroupe radius", 0x42123d);
        }
        let c = crate::frame_core::building_cell(m, s)?;
        crate::player_sight::area(m, c as u16, RADII[t as usize], p as u8, 2)?;
    }
    if neuter {
        crate::chain_habitation::own(
            m,
            "InitBatimentNeutre",
            R {
                eax: s,
                edx: p,
                ..Default::default()
            },
        )?;
        if matches!(m.b(a + 8)?, 1 | 2 | 5) {
            reset(m, a)?;
        }
    }
    if neuter || m.w(0x495be0 + 2 * p)? == s {
        m.sw(0x495be0 + 2 * p, 65535)?;
    }
    m.sb(a + 4, 200)?;
    crate::chain_vision::structure_minimap(m, s as u16, 1)
}
pub fn set_sort(m: &mut MemoriaJogo, u: u32, c: u32) -> Result<()> {
    place(m, u, c).map(|_| ())
}
fn place(m: &mut MemoriaJogo, u: u32, c: u32) -> Result<(u32, u32)> {
    let r = unit(m, u)?;
    for off in [0x7e, 0x7c, 8, 0x7e] {
        m.sw(r + off, c)?;
    }
    crate::frame_spawn::release(m, u)?;
    m.sb(r + 0x88, 0)?;
    if !matches!(m.b(r + 2)?, 0 | 4) {
        m.sb(r + 2, 7)?;
        crate::frame_spawn::release(m, u)?;
    }
    m.sw(r + 0xc0, 65535)?;
    m.sw(r + 0xb8, 65535)?;
    m.sb(r + 0x87, 0)?;
    m.sb(r + 0x86, 0)?;
    crate::frame_core::call(m, "SetEnginCollision", &[u, c])?;
    m.sb(r + 0x72, 1)?;
    let p = m.b(r)?;
    if !matches!(p, 4 | 200) {
        let radius = m.b(r + 0xbb)? + m.b(r + 0xf5)?;
        if radius > 11 {
            return stop("SetSortEngin vision sum outside validated domain", 0x446146);
        }
        crate::player_sight::area(m, m.w(r + 8)? as u16, radius + 2, p as u8, 0)?;
        crate::player_sight::area(
            m,
            m.w(r + 8)? as u16,
            m.b(r + 0xbb)? + m.b(r + 0xf5)?,
            m.b(r)? as u8,
            1,
        )?;
    }
    let owner = m.b(r)?;
    let cell = m.w(r + 8)?;
    let rows = crate::player_sight::minimap(m, owner as u8, cell as u16, 4)?;
    // SetMiniMap leaves EDX at the live local light pointer, replacing DL
    // with the owner colour only when the final pixel is illuminated.
    let light = m.d(0x507d88 + 4 * m.b(0x508041)?)?;
    let edx = if m.b(light.wrapping_add(cell))? & 31 == 0 {
        let color = if owner == 200 {
            30
        } else {
            m.b(0x4e24dc + owner)?
        };
        (light & !255) | color
    } else {
        light
    };
    Ok((rows, edx))
}
fn heal(m: &mut MemoriaJogo, u: u32, amount: u16) -> Result<(u32, u32)> {
    let r = unit(m, u)?;
    let life = m.w(r + 0x9a)?.wrapping_add(u32::from(amount)) & 65535;
    m.sw(r + 0x9a, life)?;
    let maximum = m.w(0x4965b6 + 50 * m.b(r + 1)?)?;
    let clamped = life > maximum;
    if clamped {
        m.sw(r + 0x9a, maximum)?;
    }
    Ok(((maximum & 0xff00) | u32::from(clamped), r))
}
fn reset_fatigue(m: &mut MemoriaJogo, u: u32) -> Result<u32> {
    let r = unit(m, u)?;
    m.sw(r + 0x74, 0)?;
    m.sw(r + 4, m.w(r + 0x76)?)?;
    Ok(r)
}
fn experience(m: &mut MemoriaJogo, u: u32) -> Result<u32> {
    let r = unit(m, u)?;
    let p = m.b(r)?;
    if p >= 4 {
        return stop("EnginSetExperimente owner", 0x44fe42);
    }
    let a = 0x4e19c4 + 10 * p; // original queue binding verified against recuperacao_experiencia
    let exp = m.b(a + 1)?;
    if exp == 0 || m.b(a)? != m.b(r + 1)? {
        return Ok(p * 10);
    }
    m.sb(r + 0xaf, exp)?;
    let level = exp >> 5;
    let t = m.b(r + 1)?;
    let class = matches!(t,0..=3|8|11|12|15|16|18..=25);
    match level {
        2 => m.sb(r + 0xf6, 1)?,
        3 => m.sb(r + 0xf5, 1)?,
        1 if class => m.sb(r + 0xf7, 1)?,
        1 | 4 => {
            let v = m.w(0x4965ae + 50 * t + 0x2e)?;
            m.sb(
                r + 0xf8,
                v * (if level == 4 && !class { 2 } else { 1 }) / 100,
            )?;
        }
        _ => {}
    }
    for i in 0..4 {
        m.sb(a + 2 * i + 1, m.b(a + 2 * i + 3)?)?;
        m.sb(a + 2 * i, m.b(a + 2 * i + 2)?)?;
    }
    m.sb(a + 9, 0)?;
    m.sb(a + 8, 255)?;
    Ok(p * 5)
}
pub fn sortie(m: &mut MemoriaJogo, s: u32, dir: u32) -> Result<()> {
    let a = structure(s);
    let time = m.w(0x4965ae + 50 * m.b(a + 0x7b)?)?;
    if time * 100 > m.d(a + 0x8c)? {
        return Ok(());
    }
    let b = a + 12;
    let u = bag(m, "BagRetire", b, 1, 9)? & 65535;
    let rec = unit(m, u)?;
    let owner = m.b(a + 4)?;
    for resource in [2, 1] {
        while bag(m, "BagContient", rec + 10, 2, resource)? & 65535 != 0 {
            let q = bag(m, "BagRetire", rec + 10, 2, resource)? & 65535;
            crate::chain_entry::credit(m, resource as u8, q as u16, owner as u8, s as u16)?;
        }
    }
    let c = crate::chain_vision::position(
        m,
        R {
            eax: s,
            ebx: dir,
            esi: u,
            edi: a,
            ..Default::default()
        },
    )? & 65535;
    crate::frame_spawn::call(m, "EnginSetType", &[u, c, dir, m.b(a + 0x7b)?])?;
    heal(m, u, 1000)?;
    reset_fatigue(m, u)?;
    experience(m, u)?;
    set_sort(m, u, c)?;
    crate::frame_spawn::order(m, u, m.w(a + 0x7c)? as u16)?;
    if m.b(a + 4)? == m.b(0x508041)? {
        crate::player_orders::ask(m, u, 6, 0, u)?;
    }
    let t = m.b(a + 8)?;
    let mut inhabited = matches!(t, 2 | 21) && bag(m, "BagContientTroupe", b, 0, 0)? & 255 != 0;
    if !inhabited {
        inhabited = bag(m, "BagContient", b, 1, 9)? & 65535 != 0;
    }
    if !inhabited {
        abandon(m, s, false)?;
    }
    reset(m, a)?;
    crate::frame_ai::call(m, "iagUnitSendMessage", &[u, 0x46, 0])?;
    crate::frame_ai::call(m, "iagVilleSendMessage", &[s, 3, 0])?;
    Ok(())
}
/// BatimentEnleveUneTroupeHospi 0x41da6f (type 19): the first bag entry leaves,
/// fed (EnginResetFatigue 0x4530fd), and walks to the exit cell.
fn hospital_release(m: &mut MemoriaJogo, s: u32) -> Result<u32> {
    let a = structure(s);
    let exit = crate::chain_vision::exit_cell(m, s)?;
    m.sw(a + 0x7c, u32::from(exit))?;
    let position = crate::chain_vision::position(
        m,
        R {
            eax: s,
            ..Default::default()
        },
    )? & 65535;
    let b = a + 12;
    for i in 0..m.w(b)? {
        let u = u32::from(crate::chain_cargo::remove_index(m, b, i as u16)?);
        if u == 65535 {
            continue;
        }
        let r = unit(m, u)?;
        m.sw(r + 0x74, 0)?;
        m.sw(r + 4, m.w(r + 0x76)?)?;
        set_sort(m, u, position)?;
        crate::frame_spawn::call(m, "SetDeplaceEngin", &[u, m.w(a + 0x7c)?])?;
        return Ok(u);
    }
    Ok(65535)
}
pub fn retire_colonist(m: &mut MemoriaJogo, s: u32) -> Result<u32> {
    let a = structure(s);
    let t = m.b(a + 8)?;
    if t == 12 {
        return Ok(65535);
    }
    let b = a + 12;
    let one = bag(m, "BagContient", b, 1, 9)? & 65535 == 1;
    if one && matches!(t, 5 | 8 | 14) {
        // 0x41de73 (types 5/8) / 0x41de3f (type 14): the last colonist leaves
        // only when +5 != 4 and both +0x76 and +0x75 are zero; otherwise 0xffff
        // (type 14 also sets the owner's byte at 0x4dfb54).
        let free = m.b(a + 5)? != 4 && m.b(a + 0x76)? == 0 && m.b(a + 0x75)? == 0;
        if !free {
            if t == 14 {
                m.sb(0x4dfb54 + m.b(a + 4)?, 1)?;
            }
            return Ok(65535);
        }
    }
    if matches!(t, 19 | 21) {
        let mut result = 65535u32;
        if t == 19 {
            // 0x41df1c: every patient leaves, one per BatimentEnleveUneTroupeHospi.
            while bag(m, "BagContientTroupe", b, 0, 0)? & 255 != 0 {
                result = hospital_release(m, s)?;
            }
        } else {
            // 0x41df46: everybody leaves the tent, or goes back in the bag when
            // RechercheEmplacementSortieTente finds no cell. An empty bag leaves
            // the result an uninitialized word in the original; no caller of a
            // tent uses it.
            for _ in 0..m.w(b)? {
                let u = u32::from(crate::chain_cargo::remove_index(m, b, 0)?);
                result = u;
                if u == 65535 {
                    continue;
                }
                let c = crate::chain_vision::tent_exit(m, s, u)?;
                if c == 65535 {
                    crate::chain_bags::body(
                        m,
                        "BagPrend",
                        R {
                            eax: b,
                            edx: 1,
                            ebx: u,
                            ..Default::default()
                        },
                    )?;
                } else {
                    set_sort(m, u, u32::from(c))?;
                }
            }
        }
        // 0x41dfbd: a tent only keeps troops; other types keep colonists.
        let occupied = if t == 21 {
            bag(m, "BagContientTroupe", b, 0, 0)? & 255 != 0
        } else {
            bag(m, "BagContient", b, 1, 9)? & 65535 != 0
        };
        if !occupied {
            abandon(m, s, true)?;
        }
        return Ok(result);
    }
    if t == 8 {
        // 0x41d964: the hangar releases every bag entry (BagRetireInd index 0);
        // each live one gets RechercheEmplacementSortieHangar, SetSortEngin and
        // message 0x42. The result is the last BagRetireInd value (an empty bag
        // returns an uninitialized stack word in the original: explicit stop).
        let n = m.w(b)?;
        // An empty bag returns an uninitialized word in the original; no caller
        // consumes it when the hangar is empty.
        let mut result = 65535;
        for _ in 0..n {
            let u = u32::from(crate::chain_cargo::remove_index(m, b, 0)?);
            result = u;
            if u == 65535 {
                continue;
            }
            let c = crate::chain_vision::hangar_exit(m, s, u)?;
            set_sort(m, u, u32::from(c))?;
            crate::frame_ai::call(m, "iagUnitSendMessage", &[u, 0x42, 0])?;
        }
        if bag(m, "BagContient", b, 1, 9)? & 65535 != 0 {
            return Ok(result);
        }
        abandon(m, s, true)?;
        return Ok(result);
    }
    let c = if t == 5 {
        // 0x41d9e1: the type-5 exit cell (BatimentCalculCaseSortie also sets +0x7c).
        u32::from(crate::chain_vision::case_sortie(m, s)?)
    } else {
        let c = crate::chain_vision::exit_cell(m, s)?;
        m.sw(a + 0x7c, u32::from(c))?;
        crate::chain_vision::position(
            m,
            R {
                eax: s,
                ..Default::default()
            },
        )? & 65535
    };
    let mut result = 65535;
    for _ in 0..m.w(b)? {
        let u = bag(m, "BagRetire", b, 1, 9)?;
        if u & 65535 == 65535 {
            continue;
        }
        let r = unit(m, u)?;
        m.sw(r + 0x74, 0)?;
        m.sw(r + 4, m.w(r + 0x76)?)?;
        set_sort(m, u & 65535, c)?;
        crate::frame_ai::call(m, "iagUnitSendMessage", &[u & 65535, 0x42, 0])?;
        crate::frame_spawn::order(m, u & 65535, m.w(a + 0x7c)? as u16)?;
        result = u;
        break;
    }
    if t == 2 && bag(m, "BagContientTroupe", b, 0, 0)? & 255 != 0 {
        return Ok(result);
    }
    if bag(m, "BagContient", b, 1, 9)? & 65535 != 0 {
        return Ok(result);
    }
    abandon(m, s, true)?;
    Ok(result)
}

/// CalculStatusRecrute0x423f9d, exactly the selected structure's raw progress.
pub fn status(m: &MemoriaJogo, s: u32, t: u32) -> Result<u32> {
    if s == 65535 {
        return Ok(0);
    }
    let a = structure(s);
    let kind = m.b(a + 8)?;
    let divisor =
        if matches!(kind, 1 | 2 | 5 | 13 | 15 | 16 | 20) || (kind == 18 && m.b(a + 0x7a)? == 0) {
            m.w(0x4965ae + 50 * (t & 255))?
        } else if kind == 6 {
            m.w(0x497174 + 46 * (t & 255))?
        } else if matches!(kind, 14 | 18) {
            m.w(0x4965b6 + 50 * m.b(unit(m, m.w(a + 0x88)?)? + 1)?)?
        } else {
            return Ok(0);
        };
    if divisor == 0 {
        return stop("CalculStatusRecrute division", 0x423f9d);
    }
    Ok(m.d(a + 0x8c)? / divisor)
}

/// Core recruitment owners, exact seven data GPRs plus RET for original fixtures.
/// These are Rust validation entries; no C ABI identifier or layout is changed.
pub fn entry(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    let index = r.eax & 65535;
    let u = unit(m, index)?;
    match name {
        "EnginSetType" => {
            let kind = r.ecx & 255;
            if kind >= 26 || !matches!(m.b(u)?, 0..=4 | 200) {
                return stop("EnginSetType recruitment domain", 0x457b87);
            }
            r.eax = crate::frame_spawn::call(m, name, &[index, r.edx & 65535, r.ebx & 255, kind])?;
            r.ebx = 0x1d;
            r.ecx = u.wrapping_add(10);
            r.edx = 0x1e;
        }
        "EnginAddVie" => {
            if m.b(u + 1)? >= 26 {
                return stop("EnginAddVie recruitment type", 0x457122);
            }
            (r.eax, r.edx) = heal(m, index, r.edx as u16)?;
        }
        "EnginResetFatigue" => r.eax = reset_fatigue(m, index)?,
        "EnginSetExperimente" => r.eax = experience(m, index)?,
        "SetSortEngin" => {
            let cell = r.edx & 65535;
            let (w, h) = (m.w(0x497168)?, m.w(0x49716a)?);
            if m.b(u + 1)? >= 26
                || !matches!(m.b(u)?, 0..=4 | 200)
                || !(1..=256).contains(&w)
                || !(1..=256).contains(&h)
                || cell & 255 >= w
                || cell >> 8 >= h
                || m.b(0x508041)? >= 4
                || (m.w(0x496fe0)?, m.w(0x496fe2)?) != (128, 128)
            {
                return stop("SetSortEngin recruitment domain", 0x445fbc);
            }
            (r.eax, r.edx) = place(m, index, cell)?;
        }
        _ => return stop("unknown recruitment owner", 0x420f69),
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
