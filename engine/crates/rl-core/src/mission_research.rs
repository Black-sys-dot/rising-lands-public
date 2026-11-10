//! Research transitions used by new-campaign setup and IaRechercheAjuste.
//! Direct transcription of InventionSetEffet/RechercheEndTechnologie and table selectors.
use crate::{
    mission_load::{copy, require},
    FrameRegisters, MemoriaJogo, Result,
};
fn unit_technology(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    crate::chain_technology::Technology::default().entry(
        m,
        "EnginSetTechnologie",
        FrameRegisters {
            eax: p,
            ..Default::default()
        },
    )?;
    Ok(())
}
fn structure_technology(m: &mut MemoriaJogo) -> Result<()> {
    for s in 0..1000 {
        let a = 0x49a0c8 + 276 * s;
        if m.u8(a + 5)? == 0 {
            continue;
        }
        let p = u32::from(m.u8(a + 4)?);
        let t = u32::from(m.u8(a + 8)?);
        if t == 3 && p < 4 && m.u16(0x4dfb2c + 6 * p)? != 0 {
            crate::chain_habitation::level(m, p, 0x41ce92)?;
        }
        crate::mission_tail::technology(m, p, t)?;
    }
    Ok(())
}
fn capacity(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    const CAPS: [u16; 26] = [
        0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 2, 5, 1, 3, 1, 6, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    for i in 0..800 {
        let a = crate::frame_spawn::unit(m, i)?;
        if matches!(m.u8(a + 2)?, 0 | 4) || u32::from(m.u8(a)?) != p {
            continue;
        }
        let t = u32::from(m.u8(a + 1)?);
        let cap = CAPS.get(t as usize).copied().unwrap_or(0);
        let bonus = crate::frame_economy::bonus(m, p, t, 1, 0)? as u8;
        m.set_u16(a + 12, cap.wrapping_add(u16::from(bonus)))?;
    }
    Ok(())
}
fn unit_bonus(m: &mut MemoriaJogo, p: u32, t: u32, selector: u8, value: u16) -> Result<()> {
    if selector > 12 {
        return Ok(());
    }
    let types: Vec<u32> = if t == 30 && matches!(selector, 0 | 1 | 3 | 4 | 5 | 6) {
        (0..26)
            .filter(|t| match selector {
                1 => matches!(t, 12 | 13 | 15),
                3..=5 => !matches!(t, 4 | 5 | 6 | 7 | 9 | 10 | 13 | 14 | 17),
                _ => true,
            })
            .collect()
    } else {
        vec![t]
    };
    let (offset, width) = [
        (0, 2),
        (11, 1),
        (2, 1),
        (4, 2),
        (6, 1),
        (8, 2),
        (10, 1),
        (11, 1),
        (13, 1),
        (14, 1),
        (17, 1),
        (15, 1),
        (16, 1),
    ][usize::from(selector)];
    for t in types {
        let v = match selector {
            0 => {
                let base = i32::from(m.u16(0x4965ae + 50 * t + 46)?);
                base.wrapping_mul(i32::from(value)) / 10
            }
            3 => {
                require(value != 0, "BonusSetEngin recharge division", 0x428a67)?;
                let base = i32::from(m.u16(0x4965ae + 50 * t + 48)?);
                base - base * 10 / i32::from(value)
            }
            12 => i32::from(value) - 10,
            _ => i32::from(value),
        };
        let a = 0x4dfba0 + 468 * p + 18 * t + offset;
        if width == 2 {
            m.set_u16(a, v as u16)?;
        } else {
            m.set_u8(a, v as u8)?;
        }
    }
    if selector == 1 {
        capacity(m, p)?;
    }
    Ok(())
}
fn structure_bonus(m: &mut MemoriaJogo, p: u32, t: u32, selector: u8, value: u16) -> Result<()> {
    match selector {
        6 => {
            if t == 30 {
                for t in 0..22 {
                    m.set_u8(0x4e02f0 + 44 * p + 2 * t, value as u8)?;
                }
            } else {
                m.set_u8(0x4e02f0 + 44 * p + 2 * t, value as u8)?;
            }
        }
        1 if t != 30 => m.set_u8(0x4dfbab + 468 * p + 18 * t, value as u8)?,
        // Original BL=30 branch tests its saved BL against 1/2, so writes nothing.
        _ => (),
    }
    Ok(())
}
/// InventionSetEffet 0x470f98. All original effect kinds and unlock branches,
/// and the completion notification.
pub fn effect(m: &mut MemoriaJogo, p: u32, item: u32, branch: u32, notify: bool) -> Result<()> {
    let a = 0x50817c + 360 * (branch & 65535) + 12 * (item & 65535);
    let kind = m.u8(a + 6)?;
    let subtype = m.u8(a + 7)?;
    let target = u32::from(m.u8(a + 8)?);
    let value = u16::from(m.u8(a + 9)?);
    let valid = kind != 255 && m.u8(a + 4)? != 100;
    // 0x470fd0..0x471026: an entry the mission does not grant by itself (kind
    // 255 or availability stage 100) announces its completion to the local
    // player, notify ECX == 1 and AL == [0x508041], with
    // MessageBoxSet(10, low16(branch * 100 + item)), before any effect.
    if !valid && notify && p & 255 == u32::from(m.u8(0x508041)?) {
        let code = branch.wrapping_mul(100).wrapping_add(item) as u16;
        crate::message_box::set(m, 10, code)?;
    }
    match kind {
        0 => match subtype {
            3 => {
                m.set_u8(0x4dfad4 + 22 * p + target, 1)?;
                unit_technology(m, p)?;
                match target {
                    13 => m.set_u8(0x4e1934 + 26 * p + 3, 1)?,
                    16 => m.set_u8(0x4e1934 + 26 * p + 10, 1)?,
                    18 => {
                        m.set_u8(0x4e1934 + 26 * p + 16, 1)?;
                        m.set_u8(0x4e1934 + 26 * p + 17, 1)?;
                    }
                    _ => (),
                }
                if matches!(target, 13 | 16 | 18 | 20) {
                    structure_technology(m)?;
                }
            }
            1 => {
                m.set_u8(0x4e1934 + 26 * p + target, 1)?;
                structure_technology(m)?;
                if matches!(target, 11 | 13 | 15) {
                    m.set_u8(0x4dfad4 + 22 * p + 5, 1)?;
                    unit_technology(m, p)?;
                }
            }
            4 => {
                m.set_u8(0x508f44 + 5 * p + target, 1)?;
                crate::mission_tail::technology(m, p, 6)?;
            }
            _ => (),
        },
        1 => unit_bonus(m, p, target, subtype, value)?,
        2 if matches!(subtype, 1 | 6) => structure_bonus(m, p, target, subtype, value)?,
        _ => (),
    }
    Ok(())
}
/// RechercheEndTechnologie 0x47149d, preserving earlier stores on a missing UI boundary.
pub fn complete(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let a = 0x5080f4 + 16 * (p & 255);
    m.set_u16(0x508174 + 2 * (p & 255), 0)?;
    let branch = u32::from(m.u8(a + 4)?);
    let item = u32::from(m.u8(a + branch)?);
    effect(m, p & 255, item, branch, true)?;
    let branch = u32::from(m.u8(a + 4)?);
    m.set_u8(a + branch, m.u8(a + branch)?.wrapping_add(1))?;
    m.set_u32(a + 8, 0)?;
    m.set_u32(a + 12, 0)
}
/// RechercheCalculNew 0x47150f, used when entering the between-mission research screen.
pub fn recalculate(m: &mut MemoriaJogo) -> Result<()> {
    let mut technology = crate::chain_technology::Technology::default();
    technology.entry(m, "TechnologieReset", FrameRegisters::default())?;
    technology.entry(m, "BonusReset", FrameRegisters::default())?;
    copy(m, 0x5080f4, 0x508134, 64)?;
    for p in 0..4 {
        for branch in 0..4 {
            let level = u32::from(m.u8(0x5080f4 + 16 * p + branch)?);
            require(level <= 30, "RechercheCalculNew stored level", 0x4715a7)?;
            for item in 0..level {
                if m.u8(0x50817c + 360 * branch + 12 * item + 6)? != 255 {
                    effect(m, p, item, branch, false)?;
                }
            }
        }
    }
    for p in 0..4 {
        m.set_u8(0x50871c + p, 0)?;
        for branch in 0..4 {
            for item in 0..30 {
                let a = 0x50817c + 360 * branch + 12 * item;
                if m.u8(a + 6)? == 255 || m.u8(a + 5)? > m.u8(0x496fe4)? {
                    break;
                }
                let level = m.u8(0x5080f4 + 16 * p + branch)?;
                if u32::from(level) > item {
                    continue;
                }
                if m.u8(a + 4)? <= m.u8(0x496fe4)? {
                    effect(m, p, item, branch, true)?;
                    m.set_u8(0x5080f4 + 16 * p + branch, (item + 1) as u8)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(local: u8) -> MemoriaJogo {
        let mut table = vec![0u8; 360 * 4];
        // Branch 1 item 2: kind 255 (no effect), so only the notification runs.
        table[360 + 24 + 4] = 1;
        table[360 + 24 + 6] = 255;
        let mut m = MemoriaJogo::from_blocks([
            (0x50817c, table),
            (0x508041, vec![local]),
            (0x507d54, vec![0; 5]),
            (0x4e7754, vec![0; 518 * 256]),
            (0x509094, b"C:\\RISING\\EVENTS.INI\0".to_vec()),
        ])
        .unwrap();
        crate::message_box::install_events(
            &mut m,
            b"[EVENEMENT]\n102 = LABO3\n[TEXTE]\n102 = Primeira*Segunda\n",
        )
        .unwrap();
        m
    }

    #[test]
    fn local_completion_queues_branch_item_notification() {
        let mut m = world(2);
        effect(&mut m, 2, 2, 1, true).unwrap();
        assert_eq!(m.u16(0x507d56).unwrap(), 1);
        assert_eq!(m.u8(0x4e7754).unwrap(), 10);
        assert_eq!(m.u16(0x4e7756).unwrap(), 102);
        assert_eq!(m.read(0x4e7754 + 0x106, 6).unwrap(), b"LABO3\0");
        assert_eq!(m.read(0x4e7754 + 4, 9).unwrap(), b"Primeira\0");
        for (p, notify) in [(1, true), (2, false)] {
            let mut m = world(2);
            let before = m.clone();
            effect(&mut m, p, 2, 1, notify).unwrap();
            assert_eq!(m, before);
        }
    }
}
