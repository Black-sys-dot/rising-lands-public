use super::*;

unsafe fn saved(handle: u64, input: &FrameInput) -> Vec<u8> {
    let mut required = 0;
    assert_eq!(
        unsafe { rl_salvar_bytes(handle, input, ptr::null_mut(), 0, &mut required) },
        6
    );
    let mut bytes = vec![0xa5; required as usize];
    assert_eq!(
        unsafe { rl_salvar_bytes(handle, input, bytes.as_mut_ptr(), required, &mut required) },
        0
    );
    bytes
}
fn create() -> u64 {
    let mut handle = 0;
    assert_eq!(
        unsafe { rl_criar_mundo(ABI_VERSION, 37, u32::MAX - 7, &mut handle) },
        0
    );
    let seed = 1234u32.to_le_bytes();
    let block = RlBloco {
        address: rl_core::WORLD_SEED,
        length: 4,
        data: seed.as_ptr(),
    };
    assert_eq!(unsafe { rl_carregar_blocos(handle, &block, 1) }, 0);
    assert_eq!(rl_avancar_ticks(handle, 12345), 0);
    handle
}
fn declared() -> player_abi::RlDeclaredInput {
    player_abi::RlDeclaredInput {
        size: std::mem::size_of::<player_abi::RlDeclaredInput>() as u32,
        flags: 1,
        cursor_x: 50,
        cursor_y: 60,
        messages: ptr::null(),
        message_count: 0,
        keys: ptr::null(),
        key_count: 0,
    }
}
#[test]
fn copied_save_load_and_queued_inputs_are_atomic() {
    let handle = create();
    let mut other = 0;
    let input = FrameInput {
        offscreen: 0x12345678,
        msg: [1, 2, 3, 4],
        ..Default::default()
    };
    let mut d = declared();
    assert_eq!(unsafe { rl_entrada_enfileirar(handle, 2, &d) }, 0);
    d.cursor_x = 777; // The accepted answer must already have been copied.
    let mut count = 0;
    assert_eq!(
        unsafe { rl_entrada_quantidade_pendente(handle, &mut count) },
        0
    );
    assert_eq!(count, 1);
    let bytes = unsafe { saved(handle, &input) };
    let (_, resume) = save::decode(&bytes).unwrap();
    assert_eq!(resume.pending[0].answers.cursor, Some((50, 60)));
    let mut output = FrameInput::default();
    assert_eq!(
        unsafe { rl_carregar_bytes(bytes.as_ptr(), bytes.len() as u32, &mut other, &mut output) },
        0
    );
    assert_eq!(output, input);
    assert_eq!(unsafe { saved(other, &output) }, bytes);
    let mut sentinel = [0xa5; 8];
    let mut required = 0;
    assert_eq!(
        unsafe { rl_salvar_bytes(handle, &input, sentinel.as_mut_ptr(), 8, &mut required) },
        6
    );
    assert_eq!(sentinel, [0xa5; 8]);
    assert_eq!(required as usize, bytes.len());
    for (offset, code) in [(8, 14), (32, 13)] {
        let mut bad = bytes.clone();
        bad[offset] ^= 1;
        let before = output;
        let previous_handle = other;
        assert_eq!(
            unsafe { rl_carregar_bytes(bad.as_ptr(), bad.len() as u32, &mut other, &mut output) },
            code
        );
        assert_eq!(other, previous_handle);
        assert_eq!(output, before);
        assert_eq!(unsafe { saved(other, &output) }, bytes);
    }
    // This sparse fixture cannot execute a full frame. Both queue and carried
    // registers must survive its named memory error.
    assert_ne!(unsafe { rl_entrada_passo_pendente(other, &mut output) }, 0);
    assert_eq!(output, input);
    assert_eq!(
        unsafe { rl_entrada_quantidade_pendente(other, &mut count) },
        0
    );
    assert_eq!(count, 1);
    assert_eq!(unsafe { rl_carregar_blocos(other, ptr::null(), 0) }, 0);
    assert_eq!(
        unsafe { rl_entrada_quantidade_pendente(other, &mut count) },
        0
    );
    assert_eq!(count, 0);
    assert_eq!(rl_liberar_mundo(handle), 0);
    assert_eq!(rl_liberar_mundo(other), 0);
    assert_eq!(unsafe { rl_entrada_enfileirar(handle, 2, &d) }, 2);
}
#[test]
fn files_autosave_and_errors_cross_the_abi() {
    let handle = create();
    let mut other = 0;
    let input = FrameInput::default();
    let mut required = 999;
    assert_eq!(
        unsafe { rl_autosalvar_bytes(handle, ptr::null_mut(), 0, &mut required) },
        17
    );
    assert_eq!(required, 999);
    assert_eq!(unsafe { rl_autosalvar_inicio_fase(handle, &input) }, 0);
    let original = unsafe { saved(handle, &input) };
    assert_eq!(rl_avancar_ticks(handle, 999), 0);
    assert_eq!(
        unsafe { rl_autosalvar_bytes(handle, ptr::null_mut(), 0, &mut required) },
        6
    );
    let mut slot = vec![0; required as usize];
    assert_eq!(
        unsafe { rl_autosalvar_bytes(handle, slot.as_mut_ptr(), required, &mut required) },
        0
    );
    assert_eq!(slot, original);
    let path = std::env::temp_dir().join(format!("rl-save-ffi-{}.rls", std::process::id()));
    let path = path.to_str().unwrap();
    let n = path.len() as u32;
    assert_eq!(
        unsafe { rl_autosalvar_arquivo(handle, path.as_ptr(), n) },
        0
    );
    let mut output = input;
    assert_eq!(
        unsafe { rl_carregar_arquivo(path.as_ptr(), n, &mut other, &mut output) },
        0
    );
    assert_eq!(unsafe { saved(other, &output) }, original);
    assert_eq!(
        unsafe { rl_salvar_arquivo(handle, &input, path.as_ptr(), n) },
        0
    );
    let current = unsafe { saved(handle, &input) };
    let previous = other;
    assert_eq!(
        unsafe { rl_carregar_arquivo(path.as_ptr(), n, &mut other, &mut output) },
        0
    );
    assert_eq!(rl_liberar_mundo(previous), 0);
    assert_eq!(unsafe { saved(other, &output) }, current);
    std::fs::remove_file(path).unwrap();
    assert_eq!(
        unsafe { rl_carregar_arquivo(path.as_ptr(), n, &mut other, &mut output) },
        15
    );
    assert_eq!(unsafe { saved(other, &output) }, current);
    assert_eq!(
        unsafe {
            rl_carregar_bytes(
                original.as_ptr(),
                (save::MAX_SAVE_BYTES + 1) as u32,
                &mut other,
                &mut output,
            )
        },
        9
    );
    assert_eq!(
        unsafe { rl_salvar_bytes(handle, ptr::null(), ptr::null_mut(), 0, &mut required) },
        1
    );
    assert_eq!(
        unsafe { rl_carregar_arquivo(b"a\0b".as_ptr(), 3, &mut other, &mut output) },
        1
    );
    assert_eq!(unsafe { rl_entrada_enfileirar(handle, 1, &declared()) }, 1);
    assert_eq!(rl_liberar_mundo(handle), 0);
    assert_eq!(rl_liberar_mundo(other), 0);
}
