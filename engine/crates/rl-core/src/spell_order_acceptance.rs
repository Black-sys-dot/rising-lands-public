//! Spell order boundary from intact InputGame/ProcessInputGame and 4459c4.
//! UI availability, packet dispatch and the deliberately weaker setter are distinct.
use crate::{
    frame::stop,
    player::{self, Memory},
    FrameRegisters, MemoriaJogo, Result,
};

#[cfg(feature = "validation-trace")]
thread_local! { static CALLS: std::cell::RefCell<Vec<[u32; 5]>> = const { std::cell::RefCell::new(Vec::new()) }; }
fn observe(address: u32, args: [u32; 4]) {
    #[cfg(feature = "validation-trace")]
    CALLS.with(|c| {
        c.borrow_mut()
            .push([address, args[0], args[1], args[2], args[3]])
    });
    #[cfg(not(feature = "validation-trace"))]
    let _ = (address, args);
}
pub fn take_validation_calls() -> Vec<[u32; 5]> {
    #[cfg(feature = "validation-trace")]
    {
        CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
    }
    #[cfg(not(feature = "validation-trace"))]
    {
        Vec::new()
    }
}
fn record(m: &MemoriaJogo, i: u16) -> Result<u32> {
    Ok(m.d(0x4e19a0)?.wrapping_add(276 * u32::from(i)))
}
fn cost(m: &mut MemoriaJogo, spell: u8) -> Result<u8> {
    Ok(crate::spell_queries::entry(
        m,
        crate::spell_queries::GET_COUT_MANA,
        FrameRegisters {
            eax: u32::from(spell),
            ..Default::default()
        },
    )?
    .eax as u8)
}

/// SetEnginLanceSortilege4459c4. ResetOrdre runs even for a wrong type, bad
/// target mode, or self target. Spell is stored before testing the target mode.
/// No learning, ownership, live-state or mana gate exists in this routine.
pub fn set(m: &mut MemoriaJogo, caster: u16, spell: u8, mode: u8, target: u16) -> Result<()> {
    observe(
        0x4459c4,
        [caster.into(), spell.into(), mode.into(), target.into()],
    );
    let a = record(m, caster)?;
    observe(0x44357c, [caster.into(), 0, 0, 0]);
    crate::frame_spawn::reset_order(m, u32::from(caster))?;
    if m.b(a + 1)? != 4 {
        return Ok(());
    }
    m.sb(a + 0xf4, spell.into())?;
    if !(mode == 5 || mode == 1 && caster != target) {
        return Ok(());
    }
    m.sb(a + 2, 24)?;
    m.sb(a + 0x8a, 0)?;
    m.sb(a + 0xb7, mode.into())?;
    m.sw(a + 0xb8, target.into())
}

/// SetLanceSortilegeSelect4737e9 returns immediately after the FIRST live,
/// same-owner type4 in the actual selection list, even when its setter refuses.
/// Selection flags, mana and research are not additional filters in the asm.
pub fn selected(m: &mut MemoriaJogo, owner: u8, spell: u8, mode: u8, target: u16) -> Result<()> {
    observe(
        0x4737e9,
        [owner.into(), spell.into(), mode.into(), target.into()],
    );
    let list = 0x508728 + 512 * u32::from(owner);
    let mut i = 0u32;
    while (i as u16) < m.u16(list)? {
        let u = m.u16(list.wrapping_add(2 + 2 * u32::from(i as u16)))?;
        let a = record(m, u)?;
        if !matches!(m.b(a + 2)?, 0 | 4) && m.b(a)? == u32::from(owner) && m.b(a + 1)? == 4 {
            return set(m, u, spell, mode, target);
        }
        i = i.wrapping_add(1);
    }
    Ok(())
}

/// Complete ProcessInputGame entry for tag22 and the native empty/unknown-tag
/// clearing tail. Other dispatched tags stop before executing their body.
/// Re-read the player's slot for EACH tail store, as the original does.
pub fn process(m: &mut MemoriaJogo, owner: u8) -> Result<u32> {
    let slot = 0x4e2458 + 4 * u32::from(owner);
    let packet = m.d(slot)?;
    let tag = m.w(packet)?;
    if tag == 22 {
        let target = m.u16(packet.wrapping_add(6))?;
        let mode = m.u8(packet.wrapping_add(4))?;
        let spell = m.u8(packet.wrapping_add(2))?;
        selected(m, owner, spell, mode, target)?;
    } else if tag <= 39 {
        let jump = m.d(0x45ed20 + 4 * tag)?;
        if jump != 0x45f2de {
            return stop("ProcessInputGame non-spell packet", jump);
        }
    }
    for offset in [0, 2, 4, 6, 8] {
        m.sw(m.d(slot)?.wrapping_add(offset), 0)?;
    }
    m.d(slot)
}

/// InputGame42a110..42a36a page5; called AFTER the original map hover work.
/// `cell` is the caller's computed SI, not a new coordinate/camera interface.
/// This gate deliberately does not recheck the learned spell bag.
pub fn input_cast(m: &mut MemoriaJogo, owner: u8, cell: u16) -> Result<()> {
    let caster = player::first(m, owner.into())? as u16;
    m.sw(0x4e0628, caster.into())?;
    let ready = if caster == u16::MAX {
        false
    } else {
        let a = record(m, caster)?;
        m.b(a + 1)? == 4 && m.u8(a + 0xda)? >= cost(m, m.u8(0x496208)?)?
    };
    if !ready {
        m.sb(0x4e19b0, 0)?;
        m.sb(0x496207, 255)?;
        m.sw(0x4e05ae, 0)?;
        return m.sb(0x496d48, 1);
    }
    let spell = m.u8(0x496208)?;
    m.sd(0x49a014, m.d(0x508f80)? & 7)?;
    m.sw(0x49a010, 0x1e71 + 8 * u32::from(spell))?;
    if spell > 4 {
        return Ok(());
    }
    let unit = m.u16(0x496ac2)?;
    let valid = match spell {
        0 | 4 => true,
        1 => {
            if unit == u16::MAX {
                false
            } else {
                let t = m.u8(record(m, unit)? + 1)?;
                t != 4 && t.wrapping_sub(11) > 6
            }
        }
        2 => {
            if unit == u16::MAX {
                false
            } else {
                let t = m.u8(record(m, unit)? + 1)?;
                t != 4
                    && (t <= 3
                        || t == 8
                        || (11..=12).contains(&t)
                        || (15..=16).contains(&t)
                        || (18..=25).contains(&t))
                    && t != 16
            }
        }
        3 => unit != u16::MAX,
        _ => unreachable!(),
    };
    if !valid {
        m.sw(0x49a010, 0x1e0e)?;
        return m.sd(0x49a014, 0);
    }
    if m.b(0x5099ae)? != 0 && m.b(0x5099af)? != 0 {
        let ground = spell == 0 || spell == 4;
        player::order(
            m,
            owner.into(),
            22,
            [
                spell.into(),
                if ground { 5 } else { 1 },
                if ground { cell.into() } else { unit.into() },
                0,
            ],
            0x42998c,
        )?;
    }
    Ok(())
}

/// Returned InputGame local tooltip state for its Bag-code4 branch.
#[derive(Clone, Copy, Debug)]
pub struct PanelInfo {
    pub text: u32,
    pub secondary: u32,
    pub default_delay: bool,
}
/// InputGame42be78..42c05e. All learning costs are published, but the actual
/// school learning body is an explicit dependency until independently adopted.
/// Caster branch consumes the real selected caster's mana and arms page5 only
/// on a genuine pressed/released click. Bag contents remain caller-owned.
pub fn input_panel(
    m: &mut MemoriaJogo,
    owner: u8,
    spell: u8,
    inherited_costs: Option<[u16; 3]>,
) -> Result<PanelInfo> {
    let info = PanelInfo {
        text: 0x497180 + 46 * u32::from(spell),
        secondary: 65535,
        default_delay: false,
    };
    m.sw(0x4e0614, 0)?;
    m.sw(0x4e0612, 0)?;
    m.sw(0x4e0616, 0)?;
    m.sw(0x4e0618, 0)?;
    let structure = player::selected_structure(m, owner.into())?;
    if structure != 65535 {
        school_panel(m, owner, spell, structure)?;
        return Ok(info);
    }
    let mana = cost(m, spell)?;
    let Some(inherited_costs) = inherited_costs else {
        return stop(
            "InputGame requires declared inherited tooltip costs",
            0x42bffa,
        );
    };
    // The native caster branch publishes INHERITED stack locals, which its
    // prologue does not initialize. They must be supplied explicitly, never
    // replaced by an unknown zero/default. They affect tooltip costs only.
    m.sw(0x4e0612, inherited_costs[0].into())?;
    m.sw(0x4e0614, inherited_costs[1].into())?;
    m.sw(0x4e0616, inherited_costs[2].into())?;
    m.sw(0x4e0618, mana.into())?;
    if m.b(0x5099ae)? != 0 && m.b(0x5099af)? != 0 {
        let u = player::first(m, owner.into())? as u16;
        if m.u8(record(m, u)? + 0xda)? >= cost(m, spell)? {
            m.sb(0x496208, spell.into())?;
            m.sw(0x4e05ae, 5)?;
        }
    }
    Ok(info)
}

/// 0x42bee2..0x42bf8d: the page's structure branch. A type-20 tower with
/// [+0x104] = 3 arms the targeting page on a click and recentres the camera
/// on its cell (0x41aee2, 0x4265fd). Any other structure publishes the
/// learning costs (GetCoutApprentissage 0x4746fe) and, on a click it can
/// pay for (0x4676e5), sends InitApprentissage (order 0x23).
fn school_panel(m: &mut MemoriaJogo, owner: u8, spell: u8, structure: u32) -> Result<()> {
    let clicked = m.b(0x5099ae)? != 0 && m.b(0x5099af)? != 0;
    let kind = m.b(crate::player::structure(structure) + 8)?;
    if kind == 0x14 {
        let armed = m.b(crate::player::structure(structure) + 0x104)? == 3;
        if armed && clicked {
            // 0x42bfbb: BatimentNovaPrete, page 0xd with an eight-tick countdown,
            // then BatimentRecentreSurCase 0x4265fd on BatimentGetCase.
            m.sb(0x496208, spell.into())?;
            m.sw(0x4e062c, 8)?;
            m.sw(0x4e05ae, 13)?;
            let cell = crate::frame_core::building_cell(m, structure)?;
            crate::player_spy::recentre(m, cell)?;
        }
        return Ok(());
    }
    let (stone, food, third) = learning_costs(m, spell)?;
    m.sw(0x4e0612, stone)?;
    m.sw(0x4e0614, food)?;
    m.sw(0x4e0616, third)?;
    m.sw(0x4e0618, 0)?;
    if clicked {
        let p = u32::from(owner) << 5;
        if stone <= m.w(p + 0x4e2eb2)? && food <= m.w(p + 0x4e2eba)? && third <= m.w(p + 0x4e2eaa)?
        {
            player::order(m, owner.into(), 0x23, [spell.into(), 0, 0, 0], 0x45ecd4)?;
        }
    }
    Ok(())
}

fn learning_costs(m: &MemoriaJogo, spell: u8) -> Result<(u32, u32, u32)> {
    let offset = 46 * u32::from(spell);
    Ok((
        m.w(0x497178 + offset)?,
        m.w(0x497176 + offset)?,
        m.w(0x49717a + offset)?,
    ))
}

/// InitApprentissage 0x422d9f: a structure not yet learning pays the food and
/// stone costs (EnleveMatiere 0x46792c, cave indicator when it has caves),
/// and starts learning `spell`: +0x74 = 1, state 4, progress 1, +0x7b = spell.
pub fn init_apprentissage(m: &mut MemoriaJogo, p: u32, spell: u32) -> Result<()> {
    let s = m.w(0x495be0 + 2 * p)?;
    if s >= 1000 {
        return stop("InitApprentissage with no structure selected", 0x422dd2);
    }
    let r = crate::player::structure(s);
    if m.b(r + 0x74)? != 0 {
        return Ok(());
    }
    let (first, second, _) = learning_costs(m, spell as u8)?;
    for (resource, amount) in [(1u8, first), (2, second)] {
        crate::chain_restore::debit(m, resource, amount as u16, p as u8)?;
    }
    m.sb(r + 0x74, 1)?;
    m.sb(r + 5, 4)?;
    m.sd(r + 0x8c, 1)?;
    m.sb(r + 0x7b, spell)
}

/// IconeFillContenu Bag-code4/mode1 caster continuation45e2f9. `caster` is
/// the original complete caller's first selection, `slot` its visible icon.
/// This publishes availability only; it neither unlocks nor spends mana.
pub fn caster_icon(m: &mut MemoriaJogo, caster: u16, spell: u8, slot: u16) -> Result<()> {
    let active = u16::from(m.u8(record(m, caster)? + 0xda)? >= cost(m, spell)?);
    let id = 0x29ea + u32::from(spell);
    let a = 0x4e2410 + 6 * u32::from(slot);
    if m.w(a)? != id || m.u16(a + 2)? != active || m.w(a + 4)? != 0 {
        m.sb(0x4e05a2, 1)?;
    }
    m.sw(a, id)?;
    m.sw(a + 2, active.into())?;
    m.sw(a + 4, 0)
}

/// EnginNextEtape's shared456886 consumer, restricted by its caller to state24.
/// Existing target validation can reset/refuse an invalid target; only a still
/// active spell order reaches the already-proved complete attack/effect body.
/// Reload, fatigue and the enclosing unit tick stay in the existing provider.
pub fn attack_state24(m: &mut MemoriaJogo, caster: u16, record: u32) -> Result<()> {
    observe(0x456886, [caster.into(), record, 0, 0]);
    crate::chain_attack::validate_target(m, caster, record)?;
    if m.u8(record + 2)? == 24 {
        observe(0x451914, [caster.into(), 0, 0, 0]);
        crate::chain_attack::approach(m, caster)?;
    }
    Ok(())
}
