//! Stage-start policy over original mission/INI inputs. No runtime consumer or RNG changes.
use crate::{EngineError, FrameInput, MemoriaJogo, Registers, Result, TickClock, World};

pub const RAID_COUNT: u32 = 0x50a9ce;
pub const RAIDS: u32 = 0x50a9d0;
pub const COMPOSITIONS: u32 = 0x50af84;
pub const WEIGHTS: u32 = 0x50a9c0;
pub const RESOURCES: u32 = 0x4e2ea8;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Easy = 0,
    Normal = 1,
    Hard = 2,
}
impl TryFrom<u32> for Preset {
    type Error = EngineError;
    fn try_from(value: u32) -> Result<Self> {
        match value {
            0 => Ok(Self::Easy),
            1 => Ok(Self::Normal),
            2 => Ok(Self::Hard),
            _ => Err(stop("Invalid difficulty preset", 0x47ac30)),
        }
    }
}
fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}

/// LoadEntrerMonstre's mask cycle, 0x47acc6..0x47acea. AL is the result;
/// the surrounding loader's stack, scratch GPRs and compaction are not this API.
pub fn next_group(mask: u16, previous: u8) -> u8 {
    let mut candidate = previous;
    for _ in 0..16 {
        candidate = candidate.wrapping_add(1) & 15;
        if mask & (1u16 << candidate) != 0 {
            return candidate;
        }
    }
    255
}

/// SetMatiere 0x46799c, directly from the original instructions. Data GPRs
/// and every resource write are preserved (including the untouched eighth byte).
pub fn set_matiere(memory: &mut MemoriaJogo, mut r: Registers) -> Result<Registers> {
    r.ebx = (r.ebx & 0xffff) << 2;
    r.eax = (r.eax & 255).wrapping_add(r.ebx);
    let address = RESOURCES.wrapping_add(r.eax.wrapping_mul(8));
    memory.set_u16(address, r.edx as u16)?;
    memory.set_u16(address.wrapping_add(2), r.edx as u16)?;
    r.edx = 0;
    memory.set_u16(address.wrapping_add(4), 0)?;
    memory.set_u8(address.wrapping_add(6), 0)?;
    Ok(r)
}

fn scale(value: u16, numerator: u32, denominator: u32) -> u16 {
    // Modern preset policy, not original arithmetic. Zero stays zero. A timed
    // raid remains timed; saturation prevents overflow becoming an ambush.
    if value == 0 {
        0
    } else {
        (u32::from(value) * numerator / denominator).clamp(1, u32::from(u16::MAX)) as u16
    }
}
fn smaller_mask(memory: &MemoriaJogo, mask: u16) -> Result<u16> {
    let mut best = None;
    for group in 0..16 {
        if mask & (1 << group) != 0 {
            let mut population = 0u32;
            for kind in 0..26 {
                population += u32::from(memory.u8(COMPOSITIONS + group * 26 + kind)?);
            }
            if best.is_none_or(|(_, count)| population < count) {
                best = Some((group, population));
            }
        }
    }
    Ok(best.map_or(0, |(group, _)| 1 << group))
}

/// Explicit import of original, freshly loaded stage bytes (never a saved world).
/// Unlike World::load, this grants one stage-start difficulty application.
/// Campaign continuation must use continue_victory instead.
pub fn load_fresh(memory: MemoriaJogo, clock: TickClock) -> Result<World> {
    if clock.tick() != 0 {
        return Err(stop("Difficulty fresh import requires tick zero", 0x47ac30));
    }
    let mut world = World::load(memory, clock)?;
    world.difficulty_setup = Some(world.clock.tick());
    Ok(world)
}

/// Apply exactly once to explicitly fresh stage bytes, before load-tail/frame0.
/// Normal preserves even custom INI inputs and sparse layouts, byte for byte.
/// Easy/Hard require the original raid records, compositions and local resources.
/// All changes publish atomically; no RNG is read or advanced.
pub fn apply(world: &mut World, preset: Preset) -> Result<()> {
    // The capability is granted at fresh stage boundaries only: program start
    // (tick zero) or a completed MissionNext/MissionLoad continuation whose clock
    // keeps running. It is valid at exactly the granting tick, before frame0.
    if world.frame_phase != 0 || world.difficulty_setup != Some(world.clock.tick()) {
        return Err(stop(
            "Difficulty requires fresh stage before frame0",
            0x47ac30,
        ));
    }
    apply_at_load_boundary(world, preset)
}

/// Load and configure a new stage inside one transaction. Freshness comes from
/// the successful MissionNext/carry-over/MissionLoad boundary, never the tick.
/// Source and input survive every error. No API arms an existing live/save world.
pub fn continue_victory(
    world: &World,
    name: &[u8],
    eta: &[u8],
    monsters: &[u8],
    input: FrameInput,
    preset: Preset,
) -> Result<(World, FrameInput)> {
    let (mut next, input) =
        crate::mission_entry::continue_victory(world, name, eta, monsters, input)?;
    apply_at_load_boundary(&mut next, preset)?;
    Ok((next, input))
}

/// Prepared-shell stage load (rl_shell_campaign_load: after real briefing input
/// set mode0) and configuration inside one transaction, as continue_victory.
/// Serves campaign continuation, new game and restart. Source/input survive errors.
pub fn load_prepared(
    world: &World,
    name: &[u8],
    eta: &[u8],
    monsters: &[u8],
    input: FrameInput,
    preset: Preset,
) -> Result<(World, FrameInput)> {
    let (mut next, input) = crate::mission_entry::load_prepared(world, name, eta, monsters, input)?;
    apply_at_load_boundary(&mut next, preset)?;
    Ok((next, input))
}

// Private: only the three audited fresh-stage entry paths above can bypass the
// tick-zero import rule. Consume the capability only after complete success.
fn apply_at_load_boundary(world: &mut World, preset: Preset) -> Result<()> {
    if preset == Preset::Normal {
        world.difficulty_setup = None;
        return Ok(());
    }
    let mut memory = world.project()?;
    let count = u32::from(memory.u16(RAID_COUNT)?);
    if count > 100 {
        return Err(stop(
            "Difficulty raid count exceeds original table",
            0x47ac30,
        ));
    }
    let (period_n, period_d, resource_n, resource_d, weights) = match preset {
        Preset::Easy => (3, 2, 3, 2, [2, 2, 1]),
        Preset::Hard => (3, 4, 3, 4, [10, 2, 4]),
        Preset::Normal => unreachable!(),
    };
    for index in 0..count {
        let record = RAIDS + index * 12;
        // Empty and zero-period records retain the original ambush/disabled meaning.
        if memory.u16(record)? == 0xffff || memory.u16(record + 4)? == 0 {
            continue;
        }
        if memory.u32(record + 8)? != 0 {
            return Err(stop("Difficulty requires unstarted raid timers", 0x47acc0));
        }
        let period = scale(memory.u16(record + 4)?, period_n, period_d);
        memory.set_u16(record + 4, period)?;
        if preset == Preset::Easy {
            let mask = smaller_mask(&memory, memory.u16(record + 2)?)?;
            memory.set_u16(record + 2, mask)?;
            let current = memory.u8(record + 6)?;
            if current >= 16 || mask & (1 << current) == 0 {
                memory.set_u8(record + 6, next_group(mask, current))?;
            }
        }
    }
    for (index, value) in weights.into_iter().enumerate() {
        memory.set_u32(WEIGHTS + index as u32 * 4, value)?;
    }
    let player = u32::from(memory.u8(0x508041)?);
    if player >= 4 {
        return Err(stop(
            "Difficulty local player outside resource table",
            0x46799c,
        ));
    }
    // MissionLoad calls SetMatiere for Pecas, Comida and Pedra only (not the fourth
    // retained resource slot): 0x46c430, 0x46c464, 0x46c495.
    for kind in 0..3 {
        let address = RESOURCES + player * 32 + kind * 8;
        if memory.u16(address)? != memory.u16(address + 2)?
            || memory.u16(address + 4)? != 0
            || memory.u8(address + 6)? != 0
        {
            return Err(stop(
                "Difficulty requires initial resource balances",
                0x46799c,
            ));
        }
        let amount = scale(memory.u16(address)?, resource_n, resource_d);
        set_matiere(
            &mut memory,
            Registers {
                eax: kind,
                ebx: player,
                edx: u32::from(amount),
                ..Registers::default()
            },
        )?;
    }
    world.replace_memory(memory)?;
    world.difficulty_setup = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TickClock;
    fn fixture() -> World {
        let mut m = MemoriaJogo::from_blocks([
            (RAID_COUNT, vec![0; 2 + 4 * 12]),
            (COMPOSITIONS, vec![0; 416]),
            (WEIGHTS, vec![0; 12]),
            (RESOURCES, vec![0; 128]),
            (0x508041, vec![1]),
            (crate::WORLD_SEED, 42u32.to_le_bytes().to_vec()),
        ])
        .unwrap();
        m.set_u16(RAID_COUNT, 4).unwrap();
        for (i, (cell, period, mask)) in [
            (100, 9000, 3),
            (200, 0, 0),
            (0xffff, 200, 3),
            (300, 65535, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let p = RAIDS + i as u32 * 12;
            m.set_u16(p, cell).unwrap();
            m.set_u16(p + 2, mask).unwrap();
            m.set_u16(p + 4, period).unwrap();
            m.set_u8(p + 6, 1).unwrap();
        }
        m.set_u8(COMPOSITIONS, 1).unwrap();
        m.set_u8(COMPOSITIONS + 26, 3).unwrap();
        for kind in 0..3 {
            set_matiere(
                &mut m,
                Registers {
                    eax: kind,
                    ebx: 1,
                    edx: 100,
                    ..Registers::default()
                },
            )
            .unwrap();
        }
        load_fresh(m, TickClock::new(20, 0).unwrap()).unwrap()
    }
    #[test]
    fn presets_preserve_rng_other_players_and_ambushes() {
        for (preset, period, resource, mask) in
            [(Preset::Easy, 13500, 150, 1), (Preset::Hard, 6750, 75, 3)]
        {
            let mut world = fixture();
            let before = world.project().unwrap();
            apply(&mut world, preset).unwrap();
            let after = world.project().unwrap();
            assert_eq!(after.u16(RAIDS + 4).unwrap(), period);
            assert_eq!(after.u16(RAIDS + 2).unwrap(), mask);
            assert_eq!(after.u16(RESOURCES + 32).unwrap(), resource);
            assert_eq!(
                after.read(RESOURCES, 32).unwrap(),
                before.read(RESOURCES, 32).unwrap()
            );
            assert_eq!(
                after.read(RAIDS + 12, 24).unwrap(),
                before.read(RAIDS + 12, 24).unwrap()
            );
            assert_eq!(after.u32(crate::WORLD_SEED).unwrap(), 42);
            assert_eq!(
                after.u8(RAIDS + 6).unwrap(),
                if preset == Preset::Easy { 0 } else { 1 }
            );
            assert_eq!(
                after.u16(RAIDS + 40).unwrap(),
                if preset == Preset::Easy { 65535 } else { 49151 }
            );
        }
    }
    #[test]
    fn normal_is_identity_and_errors_are_atomic() {
        let mut world = fixture();
        let before = world.project().unwrap();
        apply(&mut world, Preset::Normal).unwrap();
        assert_eq!(world.generation, 0);
        assert_eq!(world.project().unwrap(), before);
        world.clock.advance(1);
        assert!(apply(&mut world, Preset::Easy).is_err());
        assert_eq!(world.project().unwrap(), before);
        let mut world = fixture();
        let mut m = world.project().unwrap();
        m.set_u16(RESOURCES + 34, 99).unwrap();
        world.replace_memory(m).unwrap();
        let before = world.project().unwrap();
        let generation = world.generation;
        assert!(apply(&mut world, Preset::Easy).is_err());
        assert_eq!(world.project().unwrap(), before);
        assert_eq!(world.generation, generation);
        assert!(Preset::try_from(3).is_err());
    }
    #[test]
    fn boundaries() {
        assert_eq!(scale(0, 3, 4), 0);
        assert_eq!(scale(1, 3, 4), 1);
        assert_eq!(scale(65535, 3, 2), 65535);
        assert_eq!(next_group(0, 255), 255);
        assert_eq!(next_group(0x8001, 15), 0);
        assert_eq!(next_group(0x8000, 15), 15);
    }
    #[test]
    fn setup_is_single_use_and_saves_never_restore_permission() {
        for preset in [Preset::Easy, Preset::Normal, Preset::Hard] {
            let mut world = fixture();
            apply(&mut world, preset).unwrap();
            let bytes = crate::save::encode(&world, &crate::save::ResumeState::default()).unwrap();
            for again in [Preset::Easy, Preset::Normal, Preset::Hard] {
                assert!(apply(&mut world, again).is_err());
                assert_eq!(
                    crate::save::encode(&world, &crate::save::ResumeState::default()).unwrap(),
                    bytes
                );
            }
            let (mut restored, _) = crate::save::decode(&bytes).unwrap();
            assert_eq!(restored.clock.tick(), 0);
            assert!(apply(&mut restored, Preset::Easy).is_err());
            assert_eq!(
                crate::save::encode(&restored, &crate::save::ResumeState::default()).unwrap(),
                bytes
            );
        }
        let world = fixture();
        let bytes = crate::save::encode(&world, &crate::save::ResumeState::default()).unwrap();
        let (mut restored, _) = crate::save::decode(&bytes).unwrap();
        assert!(apply(&mut restored, Preset::Normal).is_err());
        let mut clock = TickClock::new(37, u32::MAX - 5).unwrap();
        clock.advance(17043);
        assert!(load_fresh(world.project().unwrap(), clock).is_err());
    }
    #[test]
    fn policy_errors_preserve_the_unused_boundary_and_replacement_cannot_reopen_it() {
        for (address, value) in [(RAID_COUNT, 101), (RAIDS + 8, 1), (RESOURCES + 34, 99)] {
            let mut world = fixture();
            let original = world.project().unwrap();
            let mut bad = original.clone();
            bad.set_u16(address, value).unwrap();
            world.replace_memory(bad).unwrap();
            let before = crate::save::encode(&world, &crate::save::ResumeState::default()).unwrap();
            assert!(apply(&mut world, Preset::Easy).is_err());
            assert_eq!(
                crate::save::encode(&world, &crate::save::ResumeState::default()).unwrap(),
                before
            );
            world.replace_memory(original).unwrap();
            apply(&mut world, Preset::Easy).unwrap();
            world.replace_memory(world.project().unwrap()).unwrap();
            assert!(apply(&mut world, Preset::Hard).is_err());
        }
        let mut world = World::new(20, 0).unwrap();
        assert!(apply(&mut world, Preset::Easy).is_err());
        apply(&mut world, Preset::Normal).unwrap();
    }
}
