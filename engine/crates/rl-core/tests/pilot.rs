use rl_core::*;

fn memory(blocks: &[(u32, &[u8])]) -> MemoriaJogo {
    MemoriaJogo::from_blocks(blocks.iter().map(|(a, b)| (*a, b.to_vec()))).unwrap()
}
#[test]
fn memory_widths_and_order() {
    let mut m = memory(&[(20, &[0; 8]), (0, &[1, 2, 3, 4])]);
    assert_eq!(m.blocks().map(|(a, _)| a).collect::<Vec<_>>(), vec![0, 20]);
    assert_eq!(m.u16(0).unwrap(), 0x0201);
    assert_eq!(m.u32(0).unwrap(), 0x04030201);
    m.set_u32(20, 0x89abcdef).unwrap();
    assert_eq!(m.read(20, 4).unwrap(), [0xef, 0xcd, 0xab, 0x89]);
    m.set_u16(21, 0x1234).unwrap();
    m.set_u8(23, 0x56).unwrap();
    assert_eq!(m.u32(20).unwrap(), 0x561234ef);
}
#[test]
fn undeclared_and_cross_block_access_never_zero_fill() {
    let mut m = memory(&[(0, &[1, 2]), (2, &[3, 4])]);
    let before = m.clone();
    assert!(m.u32(0).is_err());
    assert!(m.u8(4).is_err());
    assert!(m.set_u32(0, 0).is_err());
    assert_eq!(m, before);
}
#[test]
fn regions_reject_overlap_empty_and_overflow_without_mutation() {
    let mut m = memory(&[(5, &[1, 2])]);
    let before = m.clone();
    assert_eq!(m.declare(6, vec![9]), Err(EngineError::OverlappingRegions));
    assert_eq!(m.declare(0, vec![]), Err(EngineError::EmptyRegion));
    assert_eq!(
        m.declare(u32::MAX, vec![1, 2]),
        Err(EngineError::AddressOverflow)
    );
    assert_eq!(m, before);
    m.declare(u32::MAX, vec![7]).unwrap();
    assert_eq!(m.u8(u32::MAX).unwrap(), 7);
}
#[test]
fn partially_captured_semantic_field_is_not_absent() {
    assert!(matches!(
        GameState::load(memory(&[(WORLD_SEED, &[1, 2])])),
        Err(EngineError::IncompleteField { .. })
    ));
}
#[test]
fn typed_state_round_trip_retains_every_opaque_byte() {
    let m = memory(&[
        (WORLD_SEED, &1u32.to_le_bytes()),
        (CRT_GETTER, &SINGLE_THREAD_GETTER.to_le_bytes()),
        (THREAD_DATA, &0x1000u32.to_le_bytes()),
        (0x100c, &3u32.to_le_bytes()),
        (0x600, &[0x12, 0x34, 0x56]),
    ]);
    let (mut state, layout) = GameState::load(m.clone()).unwrap();
    assert_eq!(state.project(&layout).unwrap(), m);
    state.world_rng.as_mut().unwrap().seed = 99;
    state.crt_rng.as_mut().unwrap().seed = 88;
    let projected = state.project(&layout).unwrap();
    assert_eq!(projected.u32(WORLD_SEED).unwrap(), 99);
    assert_eq!(projected.u32(0x100c).unwrap(), 88);
    assert_eq!(projected.read(0x600, 3).unwrap(), [0x12, 0x34, 0x56]);
    let (loaded, new_layout) = GameState::load(projected.clone()).unwrap();
    assert_eq!(loaded, state);
    assert_eq!(loaded.project(&new_layout).unwrap(), projected);
}
#[test]
fn rng_wrapping_edges_are_explicit_in_debug_builds() {
    assert_eq!(WorldRng { seed: 0 }.draw(), 0x7fffffff);
    assert_eq!(WorldRng { seed: 1 }.draw(), 16807);
    assert_eq!(WorldRng { seed: u32::MAX }.draw(), 16807); // Unsigned DIV path, then wrapping correction.
    assert_eq!(CrtRng { seed: u32::MAX }.draw(), 15929);
}
#[test]
fn all_u16_values_have_unclamped_bcd_round_trip() {
    for value in 0..=u16::MAX {
        let digits = BcdDigits::encode(value);
        assert_eq!(digits.decode_aliasing(0, 100), value);
    }
    assert_eq!(BcdDigits::encode(65535), BcdDigits([5, 3, 5, 65]));
}
#[test]
fn bcd_load_project_and_unusual_digits() {
    let mut m = memory(&[(0, &[255, 254, 253, 252]), (10, &[9, 9, 9, 9])]);
    let digits = BcdDigits::load(&m, 0).unwrap();
    digits.project(&mut m, 10).unwrap();
    assert_eq!(m.read(10, 4).unwrap(), m.read(0, 4).unwrap());
    let expected = 255u16
        .wrapping_add(254u16.wrapping_mul(10))
        .wrapping_add(253u16.wrapping_mul(100))
        .wrapping_add(252u16.wrapping_mul(1000));
    assert_eq!(digits.decode_aliasing(0, 10), expected);
}
#[test]
fn bcd_aliasing_reads_zeroed_and_updated_destination() {
    // source==destination: first two source bytes are erased by initial zero.
    assert_eq!(BcdDigits([9, 8, 7, 6]).decode_aliasing(100, 100), 6700);
    // Destination begins at source+2: later source digits see accumulated result.
    assert_eq!(BcdDigits([9, 8, 7, 6]).decode_aliasing(100, 102), 43989);
}
#[test]
fn missing_bcd_byte_failure_is_atomic() {
    let mut w = World::load(memory(&[(100, &[1, 2, 3])]), TickClock::new(20, 0).unwrap()).unwrap();
    let before = w.project().unwrap();
    assert!(w
        .step(
            Routine::IntToBcd,
            Registers {
                eax: 100,
                edx: 1234,
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(w.project().unwrap(), before);
}
#[test]
fn missing_rng_and_unsupported_getter_are_named_atomic_errors() {
    let mut w = World::new(20, 0).unwrap();
    assert!(matches!(
        w.step(Routine::Random, Registers::default()),
        Err(EngineError::UndeclaredAccess { .. })
    ));
    w.replace_memory(memory(&[(CRT_GETTER, &1u32.to_le_bytes())]))
        .unwrap();
    let before = w.project().unwrap();
    assert!(matches!(
        w.step(Routine::CrtRand, Registers::default()),
        Err(EngineError::UnsupportedThreadGetter {
            address: 0x48c513,
            ..
        })
    ));
    assert_eq!(w.project().unwrap(), before);
}
#[test]
fn crt_wrapped_null_seed_pointer_has_no_store() {
    let mut w = World::load(
        memory(&[
            (CRT_GETTER, &SINGLE_THREAD_GETTER.to_le_bytes()),
            (THREAD_DATA, &0xfffffff4u32.to_le_bytes()),
        ]),
        TickClock::new(20, 0).unwrap(),
    )
    .unwrap();
    let before = w.project().unwrap();
    let r = w
        .step(
            Routine::LongRand,
            Registers {
                edx: 123,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(r.eax, 0);
    assert_eq!(r.edx, 123);
    assert_eq!(w.project().unwrap(), before);
}
#[test]
fn bcd_aliasing_an_rng_refreshes_semantic_state() {
    let mut w = World::load(
        memory(&[(WORLD_SEED, &1u32.to_le_bytes())]),
        TickClock::new(20, 0).unwrap(),
    )
    .unwrap();
    w.step(
        Routine::IntToBcd,
        Registers {
            eax: WORLD_SEED,
            edx: 1234,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(w.state().world_rng.unwrap().seed, 0x01020304);
    assert_eq!(w.project().unwrap().u32(WORLD_SEED).unwrap(), 0x01020304);
}
#[test]
fn deterministic_ticks_and_virtual_clock_wrap() {
    assert_eq!(TickClock::new(0, 0), Err(EngineError::InvalidTickRate));
    let mut a = TickClock::new(60, 0).unwrap();
    a.advance(60);
    assert_eq!(a.milliseconds(), 1000);
    let mut b = TickClock::new(60, 0).unwrap();
    for _ in 0..60 {
        b.advance(1);
    }
    assert_eq!(a, b);
    let mut c = TickClock::new(20, u32::MAX - 49).unwrap();
    c.advance(1);
    assert_eq!(c.milliseconds(), 0);
    c.advance(u64::MAX);
    assert_eq!(c.tick(), 0);
    assert_eq!(c.milliseconds(), u32::MAX - 49);
}
#[test]
fn clock_pacing_never_changes_rng_results_or_routine_tick_count() {
    let m = memory(&[(WORLD_SEED, &987u32.to_le_bytes())]);
    let mut a = World::load(m.clone(), TickClock::new(20, 0).unwrap()).unwrap();
    let mut b = World::load(m, TickClock::new(60, 100).unwrap()).unwrap();
    for _ in 0..100 {
        a.clock.advance(1);
        b.clock.advance(5);
        assert_eq!(
            a.step(Routine::Random, Registers::default()).unwrap(),
            b.step(Routine::Random, Registers::default()).unwrap()
        );
    }
    assert_eq!(a.project().unwrap(), b.project().unwrap());
    assert_eq!(a.clock.tick(), 100);
    assert_eq!(b.clock.tick(), 500);
}

#[test]
fn projection_refuses_removed_or_unbound_semantic_fields() {
    let (mut state, layout) =
        GameState::load(memory(&[(WORLD_SEED, &1u32.to_le_bytes())])).unwrap();
    state.world_rng = None;
    assert!(matches!(
        state.project(&layout),
        Err(EngineError::ProjectionFieldMismatch {
            address: WORLD_SEED
        })
    ));
    let (mut state, layout) = GameState::load(MemoriaJogo::default()).unwrap();
    state.crt_rng = Some(CrtRng { seed: 123 });
    assert!(matches!(
        state.project(&layout),
        Err(EngineError::ProjectionFieldMismatch {
            address: THREAD_DATA
        })
    ));
    let (mut state, layout) = GameState::load(memory(&[
        (CRT_GETTER, &SINGLE_THREAD_GETTER.to_le_bytes()),
        (THREAD_DATA, &0x1000u32.to_le_bytes()),
        (0x100c, &1u32.to_le_bytes()),
    ]))
    .unwrap();
    state.crt_rng = None;
    assert!(matches!(
        state.project(&layout),
        Err(EngineError::ProjectionFieldMismatch { address: 0x100c })
    ));
}
