use rl_core::frame_state::{FrameState, STRUCTURE_COUNT, UNIT_COUNT};
use rl_core::{EngineError, MemoriaJogo};

fn mission_layout() -> MemoriaJogo {
    // Explicit synthetic opaque bytes exercise preservation; these are not an
    // invented MissionLoad state and do not add any original-native claim.
    let globals = (0..0x90000)
        .map(|i| (i as u8).wrapping_mul(37).wrapping_add(19))
        .collect();
    let units = (0..UNIT_COUNT * 276)
        .map(|i| (i as u8).wrapping_mul(13).wrapping_add(7))
        .collect();
    let mut memory = MemoriaJogo::from_blocks([(0x495000, globals), (0x60000000, units)]).unwrap();
    memory.set_u32(0x4e19a0, 0x60000000).unwrap();
    memory
}

#[test]
fn semantic_frame_projection_preserves_every_opaque_byte() {
    let original = mission_layout();
    let state = FrameState::load(&original).unwrap().unwrap();
    assert_eq!(state.units.len(), UNIT_COUNT);
    assert_eq!(state.structures.len(), STRUCTURE_COUNT);
    assert_eq!(state.unit_pool_base(), 0x60000000);
    let mut projected = original.clone();
    state.project(&mut projected).unwrap();
    assert_eq!(projected, original);
    assert_eq!(FrameState::load(&projected).unwrap().unwrap(), state);
}

#[test]
fn semantic_mutations_project_at_original_widths_and_reload() {
    let original = mission_layout();
    let mut state = FrameState::load(&original).unwrap().unwrap();
    state.units[17].provisions = 0xfedc;
    state.units[17].fatigue_ticks = 0xffff;
    state.units[17].facing = 0xab;
    state.units[17].group = 0x87654321;
    state.units[17].path_mode = 5;
    state.units[17].step_progress = 31;
    state.units[17].walk_frame = 7;
    state.structures[9].origin_cell = 0x4321;
    state.structures[9].production_progress = 0xdeadbeef;
    state.globals.game_frame = u32::MAX;
    state.globals.permutation_cursor = 255;
    state.globals.unit_counts[4] = 0x9876;
    state.globals.unit_kind_counts[3][25] = 0x5432;
    let mut projected = original.clone();
    state.project(&mut projected).unwrap();
    let mut expected = original;
    expected.set_u16(0x60000000 + 17 * 276 + 4, 0xfedc).unwrap();
    expected
        .set_u16(0x60000000 + 17 * 276 + 0x74, 0xffff)
        .unwrap();
    expected.set_u8(0x60000000 + 17 * 276 + 0x90, 0xab).unwrap();
    expected
        .set_u32(0x60000000 + 17 * 276 + 0x10c, 0x87654321)
        .unwrap();
    expected.set_u8(0x60000000 + 17 * 276 + 0x88, 5).unwrap();
    expected.set_u8(0x60000000 + 17 * 276 + 0x97, 31).unwrap();
    expected.set_u8(0x60000000 + 17 * 276 + 0xa4, 7).unwrap();
    expected.set_u16(0x49a0c8 + 9 * 276 + 2, 0x4321).unwrap();
    expected
        .set_u32(0x49a0c8 + 9 * 276 + 0x8c, 0xdeadbeef)
        .unwrap();
    expected.set_u32(0x496d40, u32::MAX).unwrap();
    expected.set_u8(0x4e1d18, 255).unwrap();
    expected.set_u16(0x4e1928 + 8, 0x9876).unwrap();
    expected
        .set_u16(0x4e1a3c + 3 * 0x34 + 25 * 2, 0x5432)
        .unwrap();
    assert_eq!(projected, expected);
    assert_eq!(FrameState::load(&projected).unwrap().unwrap(), state);
}

#[test]
fn sparse_captures_never_gain_synthetic_frame_state() {
    assert_eq!(FrameState::load(&MemoriaJogo::default()).unwrap(), None);
    let mut partial = MemoriaJogo::from_blocks([
        (0x4e19a0, 0x60000000u32.to_le_bytes().to_vec()),
        (0x60000000, vec![0xa7; 275]),
    ])
    .unwrap();
    assert_eq!(FrameState::load(&partial).unwrap(), None);
    partial = MemoriaJogo::from_blocks([(0x4e19a0, vec![0, 0])]).unwrap();
    assert_eq!(
        FrameState::load(&partial),
        Err(EngineError::IncompleteField { address: 0x4e19a0 })
    );
}

#[test]
fn changed_pool_binding_or_cardinality_is_rejected_before_writing() {
    let original = mission_layout();
    let state = FrameState::load(&original).unwrap().unwrap();
    let mut changed = original.clone();
    changed.set_u32(0x4e19a0, 0x61000000).unwrap();
    let before = changed.clone();
    assert_eq!(
        state.project(&mut changed),
        Err(EngineError::ProjectionFieldMismatch { address: 0x4e19a0 })
    );
    assert_eq!(changed, before);
    let mut invalid = state;
    invalid.structures.pop();
    let mut unchanged = original.clone();
    assert_eq!(
        invalid.project(&mut unchanged),
        Err(EngineError::ProjectionFieldMismatch { address: 0x4e19a0 })
    );
    assert_eq!(unchanged, original);
}

#[test]
fn canonical_state_requires_bound_frame_presence_and_projects_semantic_mutation() {
    let memory = mission_layout();
    let (mut state, layout) = rl_core::GameState::load(memory.clone()).unwrap();
    let frame = state.frame.take().unwrap();
    assert_eq!(
        state.project(&layout),
        Err(EngineError::ProjectionFieldMismatch { address: 0x4e19a0 })
    );
    state.frame = Some(frame);
    state.frame.as_mut().unwrap().units[3].owner = 2;
    let changed = state.project(&layout).unwrap();
    assert_eq!(changed.u8(0x60000000 + 3 * 276).unwrap(), 2);
    assert_eq!(
        changed.read(0x60000000 + 3 * 276 + 3, 1).unwrap(),
        memory.read(0x60000000 + 3 * 276 + 3, 1).unwrap()
    );
    let (mut sparse, sparse_layout) = rl_core::GameState::load(MemoriaJogo::default()).unwrap();
    sparse.frame = state.frame;
    assert_eq!(
        sparse.project(&sparse_layout),
        Err(EngineError::ProjectionFieldMismatch { address: 0x4e19a0 })
    );
}
