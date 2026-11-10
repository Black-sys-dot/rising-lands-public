use super::*;

fn sample() -> (World, ResumeState) {
    let m = MemoriaJogo::from_blocks([
        (WORLD_SEED, 1234567u32.to_le_bytes().to_vec()),
        (0x100000, vec![0; 8192]),
        (CRT_GETTER, SINGLE_THREAD_GETTER.to_le_bytes().to_vec()),
        (THREAD_DATA, 0x100000u32.to_le_bytes().to_vec()),
    ])
    .unwrap();
    let mut w = World::load(m, TickClock::new(37, u32::MAX - 100).unwrap()).unwrap();
    w.clock.advance(u64::MAX - 3);
    w.generation = u64::MAX;
    w.frame_phase = 2;
    w.player_cursor = (799, 599);
    w.frame_events.push(FrameEvent {
        kind: 3,
        args: [1, 2, 3, 4, 5],
        ..Default::default()
    });
    w.player_events.push(player::PlayerEvent {
        values: [u32::MAX; 12],
        ..Default::default()
    });
    w.frame_freed = MemoriaJogo::from_blocks([(0x200000, vec![9; 32])]).unwrap();
    let mut resume = ResumeState {
        input: FrameInput {
            cursor_x: 3,
            offscreen: 0xdeadbeef,
            registers: FrameRegisters {
                esp: 0x100001,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    resume.pending.push_back(PendingInput {
        phase: 2,
        answers: DeclaredInput {
            cursor: Some((12, 13)),
            messages: vec![[0x200, 1, 2], [0x201, 3, 4]],
            keys: [(32, 0x8000), (65, u32::MAX)].into(),
            display_sound_boundary: true,
            display_text_boundary: true,
        },
    });
    (w, resume)
}
#[test]
fn complete_snapshot_and_future_rng_ticks_equal() {
    let (mut a, resume) = sample();
    let encoded = encode(&a, &resume).unwrap();
    let (mut b, loaded) = decode(&encoded).unwrap();
    assert_eq!(loaded, resume);
    assert_eq!(a.state(), b.state());
    assert_eq!(a.clock, b.clock);
    assert_eq!(encoded, encode(&b, &loaded).unwrap());
    assert!(encoded.len() < 1000);
    for _ in 0..200 {
        for routine in [Routine::Random, Routine::CrtRand] {
            assert_eq!(
                a.step(routine, Registers::default()),
                b.step(routine, Registers::default())
            );
        }
        a.clock.advance(1);
        b.clock.advance(1);
        assert_eq!(a.state(), b.state());
        assert_eq!(encode(&a, &resume).unwrap(), encode(&b, &loaded).unwrap());
    }
}
#[test]
fn malformed_old_version_and_corruption_are_rejected() {
    let (w, r) = sample();
    let bytes = encode(&w, &r).unwrap();
    for n in [0, 1, 8, 16, 32, 63, 64, bytes.len() - 1] {
        assert!(decode(&bytes[..n]).is_err());
    }
    let mut old = bytes.clone();
    old[8..12].copy_from_slice(&0u32.to_le_bytes());
    assert!(matches!(
        decode(&old),
        Err(SaveError::Version { format: 0, .. })
    ));
    let mut schema = bytes.clone();
    schema[12] = 0;
    assert!(matches!(decode(&schema), Err(SaveError::Version { .. })));
    let mut bad = bytes.clone();
    bad[32] ^= 1;
    assert!(matches!(decode(&bad), Err(SaveError::Checksum)));
    let mut bad = bytes.clone();
    let n = bad.len() - 1;
    bad[n] ^= 1;
    assert!(decode(&bad).is_err());
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(decode(&bad).is_err());
    let mut bomb = bytes.clone();
    bomb[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(decode(&bomb), Err(SaveError::Limit)));
    for n in 0..bytes.len() {
        let mut bad = bytes.clone();
        bad[n] ^= 0x40;
        assert!(decode(&bad).is_err(), "byte {n}");
    }
}
#[test]
fn compression_boundaries_and_incompressible_data() {
    for n in [0, 1, 2, 3, 127, 128, 129, 255, 256, 1000] {
        for bytes in [
            vec![7; n],
            (0..n).map(|v| v as u8).collect(),
            (0..n).map(|v| (v / 2) as u8).collect(),
        ] {
            let packed = compress(&bytes);
            assert_eq!(decompress(&packed, bytes.len()).unwrap(), bytes);
            assert!(decompress(&packed, bytes.len() + 1).is_err());
        }
    }
    assert!(decompress(&[255, 0], 1).is_err());
}
#[test]
fn files_and_stage_autosave_preserve_previous_on_failure() {
    let (w, r) = sample();
    let mut slot = StageAutosave::default();
    assert!(slot.bytes().is_none());
    slot.capture(&w, &r).unwrap();
    let original = slot.bytes().unwrap().to_vec();
    let path = std::env::temp_dir().join(format!("rl-save-test-{}.rls", std::process::id()));
    save_file(&path, &w, &r).unwrap();
    let (loaded, resume) = load_file(&path).unwrap();
    assert_eq!(original, encode(&loaded, &resume).unwrap());
    let mut invalid = r;
    invalid.pending[0].phase = 1;
    assert!(slot.capture(&w, &invalid).is_err());
    assert_eq!(slot.bytes().unwrap(), original);
    assert!(save_file(&path, &w, &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::remove_file(path).unwrap();
}
