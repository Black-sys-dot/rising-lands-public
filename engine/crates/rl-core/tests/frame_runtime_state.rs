use rl_core::frame_runtime_state::{RuntimeState, GROUP_COUNT, TASK_COUNT};
use rl_core::{EngineError, MemoriaJogo};
fn fixture() -> MemoriaJogo {
    let bytes = (0..0x90000)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(13))
        .collect();
    MemoriaJogo::from_blocks([(0x495000, bytes)]).unwrap()
}
#[test]
fn runtime_semantic_projection_is_identity_for_all_declared_bytes() {
    let original = fixture();
    let state = RuntimeState::load(&original).unwrap().unwrap();
    assert_eq!(state.tasks.len(), TASK_COUNT);
    assert_eq!(state.groups.len(), GROUP_COUNT);
    let mut projection = original.clone();
    state.project(&mut projection).unwrap();
    assert_eq!(projection, original);
    assert_eq!(RuntimeState::load(&projection).unwrap().unwrap(), state);
}
#[test]
fn task_group_and_runtime_mutations_use_the_original_fields() {
    let original = fixture();
    let mut state = RuntimeState::load(&original).unwrap().unwrap();
    state.tasks[7].next_task = None;
    state.tasks[7].parent_task = Some(9);
    state.tasks[7].instruction = 631;
    state.tasks[7].wait_ticks = -7;
    state.tasks[7].locals[14] = 0xdeadbeef;
    state.tasks[7].sleeping = 1;
    state.groups[18].first_unit = Some(16);
    state.groups[18].unit_count = 1;
    state.groups[18].waiting_task = None;
    state.free_tasks.first = Some(7);
    state.contexts[4].count = 101;
    state.active_tasks = 1;
    state.action_count = 1;
    state.pending_groups[3] = Some(18);
    state.pending_count = 4;
    state.live_monsters = 17;
    let mut actual = original.clone();
    state.project(&mut actual).unwrap();
    let mut expected = original;
    let t = 0x512320 + 7 * 96;
    let g = 0x51a9e0 + 18 * 32;
    expected.set_u32(t, u32::MAX).unwrap();
    expected.set_u32(t + 4, 9).unwrap();
    expected.set_u32(t + 12, 631).unwrap();
    expected.set_u32(t + 0x14, (-7i32) as u32).unwrap();
    expected.set_u32(t + 0x50, 0xdeadbeef).unwrap();
    expected.set_u8(t + 0x5e, 1).unwrap();
    expected.set_u16(g + 16, 16).unwrap();
    expected.set_u16(g + 18, 1).unwrap();
    expected.set_u32(g + 8, u32::MAX).unwrap();
    expected.set_u32(0x5148a0, 7).unwrap();
    expected.set_u32(0x5148f4, 101).unwrap();
    expected.set_u32(0x514904, 1).unwrap();
    expected.set_u32(0x50ae80, 1).unwrap();
    expected.set_u32(0x50b138 + 3 * 4, 18).unwrap();
    expected.set_u8(0x50b134, 4).unwrap();
    expected.set_u32(0x50b178, 17).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(RuntimeState::load(&actual).unwrap().unwrap(), state);
}
#[test]
fn unknown_task_and_group_fields_remain_opaque() {
    let original = fixture();
    let mut state = RuntimeState::load(&original).unwrap().unwrap();
    state.tasks[0].result = 19;
    state.groups[0].behaviour = 9;
    let mut projected = original.clone();
    state.project(&mut projected).unwrap();
    for (a, n) in [
        (0x512328, 4),
        (0x512330, 4),
        (0x51237d, 1),
        (0x51a9f4, 4),
        (0x51a9fa, 1),
        (0x51a9fc, 4),
    ] {
        assert_eq!(projected.read(a, n).unwrap(), original.read(a, n).unwrap());
    }
}
#[test]
fn sparse_cardinality_and_list_binding_invariants_are_explicit() {
    assert_eq!(RuntimeState::load(&MemoriaJogo::default()).unwrap(), None);
    let sparse = MemoriaJogo::from_blocks([(0x512320, vec![0xa7; 96])]).unwrap();
    assert_eq!(RuntimeState::load(&sparse).unwrap(), None);
    let original = fixture();
    let state = RuntimeState::load(&original).unwrap().unwrap();
    let mut broken = state.clone();
    broken.groups.pop();
    let mut projected = original.clone();
    assert_eq!(
        broken.project(&mut projected),
        Err(EngineError::ProjectionFieldMismatch { address: 0x512320 })
    );
    assert_eq!(projected, original);
    projected.set_u32(0x5148ac, 0x12345678).unwrap();
    let before = projected.clone();
    assert_eq!(
        state.project(&mut projected),
        Err(EngineError::ProjectionFieldMismatch { address: 0x512320 })
    );
    assert_eq!(projected, before);
}
