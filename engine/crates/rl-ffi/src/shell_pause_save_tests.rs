use super::*;
/// Pause menu16 -> save list14 -> slot activation -> SAVE_SLOT -> host save ->
/// rl_shell_resume: the original MenuNewSelect 0x42c570 tail (MenuInit(18) and
/// the HUD refresh latch), returning straight to the game, with the RESUME
/// world/event/input contract. Real S1 frame600 world; the pause menu is entered
/// with the original MenuInit(16) as InputGame's pause order does.
#[test]
#[ignore = "requires RL_B_GAME_SAVE (S1 frame600 .rls), RL_B_INPUTS, RL_CD_ROOT"]
fn pause_save_slot_completes_into_the_game() {
    use rl_core::{mission_files::Presentation, render_assets::sha256, save};
    use std::{fs, path::PathBuf};
    let path = |name| PathBuf::from(std::env::var_os(name).expect(name));
    let source = fs::read(path("RL_B_GAME_SAVE")).unwrap();
    assert_eq!(
        sha256(&source)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>(),
        "78b5a20c1e246b05b4840f3c283c618e52951641c63416d97328d33c5d2169ac"
    );
    let inputs = path("RL_B_INPUTS");
    let cd = path("RL_CD_ROOT");
    let presentation = Presentation::read(&inputs).unwrap();
    let mut blobs = (0..9)
        .map(|i| fs::read(inputs.join(format!("asset{i}.bin"))).unwrap())
        .collect::<Vec<_>>();
    blobs.push(fs::read(cd.join("INSTALHD/GRAPHICS/MENU.CMP")).unwrap());
    blobs.push(fs::read(cd.join("INSTALHD/GRAPHICS/INVENTE.CMP")).unwrap());
    let desc = |b: &[u8]| RlShellBytes {
        data: b.as_ptr(),
        length: b.len() as u32,
    };
    let descriptors = blobs.iter().map(|b| desc(b)).collect::<Vec<_>>();
    let (mut world, resume) = save::decode(&source).unwrap();
    let mut memory = world.project().unwrap();
    assert_eq!(memory.u8(rl_core::shell_state::MODE).unwrap(), 18);
    let mut none = HostFiles {
        files: BTreeMap::new(),
        active: None,
        position: 0,
    };
    rl_core::campaign_flow::menu_init(&mut memory, 0, 16, &mut none).unwrap();
    memory.set_u32(0x508f64, 0).unwrap();
    world.replace_memory(memory.clone()).unwrap();
    let tick = world.clock.tick();
    let paused = super::super::mission_abi::publish(world).unwrap();
    let mut shell = 0;
    unsafe {
        assert_eq!(
            rl_shell_create(
                1,
                paused,
                descriptors.as_ptr(),
                11,
                desc(&presentation.surface),
                &mut shell
            ),
            0,
            "{}",
            last_error()
        );
    }
    // Keyboard only; the mouse stays where the game left it (S1's last click).
    let mut input = RlShellInput {
        version: 1,
        mouse_x: memory.u16(0x5099a8).unwrap() as u32,
        mouse_y: memory.u16(0x5099aa).unwrap() as u32,
        mouse_ae: memory.u8(0x5099ae).unwrap() as u32,
        mouse_af: memory.u8(0x5099af).unwrap() as u32,
        mouse_b0: memory.u8(0x5099b0).unwrap() as u32,
        clock_7c: memory.u32(0x508f7c).unwrap(),
        animation_84: memory.u32(0x508f84).unwrap(),
        ..Default::default()
    };
    let mut event = RlShellEvent::default();
    let mut g = 0;
    unsafe {
        // Pause item1 ("salvar") -> SAVE_LOAD 14, completed by the engine (no slot files).
        input.keys = 4;
        assert_eq!(
            rl_shell_input(shell, g, &input, &mut event),
            0,
            "{}",
            last_error()
        );
        g = event.generation;
        input.keys = 1;
        assert_eq!(
            rl_shell_input(shell, g, &input, &mut event),
            0,
            "{}",
            last_error()
        );
        g = event.generation;
        assert_eq!((event.kind, event.mode), (6, 14));
        assert_eq!(
            rl_shell_menu_init(shell, g, ptr::null(), 0, &mut g),
            0,
            "{}",
            last_error()
        );
        // Enter on slot0 -> SAVE_SLOT (kind13) with the original slot filename.
        assert_eq!(
            rl_shell_input(shell, g, &input, &mut event),
            0,
            "{}",
            last_error()
        );
        g = event.generation;
    }
    assert_eq!((event.kind, event.mode, event.reserved), (13, 14, 0));
    let slot_path = event.path[..event.path_length as usize].to_vec();
    assert!(!slot_path.is_empty());
    let before = session(shell, g, |s| Ok(s.memory.clone())).unwrap();
    assert_eq!(before.u8(rl_core::shell_state::MODE).unwrap(), 14);
    let carried = resume.input;
    let (mut out, mut out_input) = (777u64, FrameInput::default());
    unsafe {
        // Stale generation and null pointers: refused, nothing published.
        assert_eq!(
            rl_shell_resume(shell, g - 1, &carried, &mut out, &mut out_input),
            11
        );
        assert_eq!(
            rl_shell_resume(shell, g, ptr::null(), &mut out, &mut out_input),
            1
        );
    }
    assert_eq!((out, out_input), (777, FrameInput::default()));
    session(shell, g, |s| {
        assert_eq!(s.memory, before);
        Ok(())
    })
    .unwrap();
    // The host writes its own modern save here; then the original continuation.
    unsafe {
        assert_eq!(
            rl_shell_resume(shell, g, &carried, &mut out, &mut out_input),
            0,
            "{}",
            last_error()
        );
    }
    assert_eq!(out_input, carried);
    let resumed = with_world(out, |w| {
        assert_eq!(w.clock.tick(), tick);
        Ok(w.project()?)
    })
    .unwrap();
    assert_eq!(resumed.u8(rl_core::shell_state::MODE).unwrap(), 18);
    assert_eq!(resumed.u8(0x4e05a2).unwrap(), 1);
    // Gameplay identity with the paused world: RNGs, units, players, resources.
    for (label, address, n) in [
        ("rng", 0x497164u32, 4usize),
        ("ai_rng", 0x498db4, 4),
        ("players", memory.u32(0x5080a8).unwrap(), 100),
        ("resources", 0x4e2ea8, 128),
        ("units", memory.u32(0x4e19a0).unwrap(), 220800),
    ] {
        assert_eq!(
            resumed.read(address, n).unwrap(),
            memory.read(address, n).unwrap(),
            "{label}"
        );
    }
    let mut events = vec![rl_core::FrameEvent::default(); 16];
    let mut required = 0;
    unsafe {
        assert_eq!(
            super::super::rl_eventos_quadro(out, events.as_mut_ptr(), 16, &mut required),
            0
        );
    }
    // Binding ShowCursor, then the boundaries LoadPie (101) and MCICD_Play (102).
    let kinds: Vec<(u32, u32)> = events[..required as usize]
        .iter()
        .map(|e| (e.kind, e.id))
        .collect();
    assert_eq!(kinds, [(1, 1), (2, 101), (2, 102)]);
    // The session is spent: a second completion is refused.
    let mut again = 555u64;
    unsafe {
        assert_eq!(
            rl_shell_resume(shell, g + 1, &carried, &mut again, &mut out_input),
            1
        );
    }
    assert_eq!(again, 555);
    // The next game frame continues play instead of reopening the pause menu.
    // Same continuation as RESUME: a second shell on the same paused world, pause
    // Escape -> RESUME -> rl_shell_resume. The worlds differ only by the original
    // list14 remnants (slot-name scan 0x496fe8.., list item state 0x4e2f44..,
    // mouse zone 0x5099ac) and the save tail's HUD refresh latch 0x4e05a2.
    let mut shell2 = 0;
    let mut escape = input;
    escape.keys = 8;
    let (mut via_resume, mut resume_input) = (0u64, FrameInput::default());
    unsafe {
        assert_eq!(
            rl_shell_create(
                1,
                paused,
                descriptors.as_ptr(),
                11,
                desc(&presentation.surface),
                &mut shell2
            ),
            0
        );
        assert_eq!(rl_shell_input(shell2, 0, &escape, &mut event), 0);
        assert_eq!(event.kind, 5);
        assert_eq!(
            rl_shell_resume(
                shell2,
                event.generation,
                &carried,
                &mut via_resume,
                &mut resume_input
            ),
            0,
            "{}",
            last_error()
        );
    }
    assert_eq!(resume_input, carried);
    let other = with_world(via_resume, |w| Ok(w.project()?)).unwrap();
    let allowed = |p: u32| {
        (0x496fe8..0x497128).contains(&p)
            || p == 0x4e05a2
            || (0x4e2f44..0x4e2f6c).contains(&p)
            || p == 0x5099ac
    };
    let mut latch = false;
    for (address, bytes) in resumed.blocks() {
        let theirs = other.read(address, bytes.len()).unwrap();
        for (i, (a, b)) in bytes.iter().zip(theirs).enumerate() {
            let p = address + i as u32;
            if a != b {
                assert!(allowed(p), "unexpected difference at {p:#x}: {a} vs {b}");
                latch |= p == 0x4e05a2;
            }
        }
    }
    assert!(latch || other.u8(0x4e05a2).unwrap() == 1);
    assert_eq!(rl_shell_free(shell2), 0);
    assert_eq!(super::super::rl_liberar_mundo(via_resume), 0);
    assert_eq!(rl_shell_free(shell), 0);
    assert_eq!(super::super::rl_liberar_mundo(out), 0);
    assert_eq!(super::super::rl_liberar_mundo(paused), 0);
}
fn last_error() -> String {
    super::super::LAST_ERROR.with(|e| e.borrow().clone())
}
