//! Raw ABI declarations, laid out exactly as `engine/include`.
//!
//! The Windows host links the `rising_engine` rlib produced by `rl-ffi`, so the
//! declarations below resolve to the same symbols a C host would load from the
//! DLL. Handles are opaque integers; every pointer is only valid for one call.

#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_void};

pub const ABI_VERSION: u32 = 0x0001_0006;
pub const RENDER_CONTRACT: u32 = 0x1_0009;
pub const SAVE_FORMAT_VERSION: u32 = 1;
pub const SHELL_VERSION: u32 = 1;
pub const GAME_HUD_BACKGROUND_VERSION: u32 = 1;

pub const RL_OK: i32 = 0;
pub const RL_UNSUPPORTED_PATH: i32 = 5;
pub const RL_BUFFER_TOO_SMALL: i32 = 6;
pub const RL_SAVE_INVALID: i32 = 12;
pub const RL_SAVE_BUSY: i32 = 16;
pub const RL_IO_ERROR: i32 = 15;

pub const RL_EXECUCAO_JOGO: u32 = 1;
pub const RL_DIFICULDADE_NORMAL: u32 = 1;

pub const RL_SHELL_MAIN: u32 = 2;
pub const RL_SHELL_BRIEFING: u32 = 17;
pub const RL_SHELL_RESULT: u32 = 19;
pub const RL_SHELL_OBJECTIVES: u32 = 23;
pub const RL_SHELL_CREDITS_SCREEN: u32 = 10;
pub const RL_SHELL_PAUSE: u32 = 16;
pub const RL_SHELL_LOAD_MENU: u32 = 13;
pub const RL_SHELL_LOAD_LIST: u32 = 12;
pub const RL_SHELL_SAVE_LIST: u32 = 14;
pub const RL_SHELL_FINAL: u32 = 24;

pub const RL_SHELL_NONE: u32 = 0;
pub const RL_SHELL_SHOW: u32 = 1;
pub const RL_SHELL_RESET_CAMPAIGN: u32 = 2;
pub const RL_SHELL_START_MISSION: u32 = 3;
pub const RL_SHELL_MISSION_NEXT: u32 = 4;
pub const RL_SHELL_RESUME: u32 = 5;
pub const RL_SHELL_SAVE_LOAD: u32 = 6;
pub const RL_SHELL_INTRODUCTION: u32 = 7;
pub const RL_SHELL_CREDITS: u32 = 8;
pub const RL_SHELL_MULTIPLAYER: u32 = 9;
pub const RL_SHELL_QUIT: u32 = 10;
pub const RL_SHELL_DEFEAT_MOVIE: u32 = 11;
pub const RL_SHELL_LOAD_SLOT: u32 = 12;
pub const RL_SHELL_SAVE_SLOT: u32 = 13;
pub const RL_SHELL_CAMPAIGN_MOVIE: u32 = 14;

pub const RL_SHELL_ENTER: u32 = 1;
pub const RL_SHELL_UP: u32 = 2;
pub const RL_SHELL_DOWN: u32 = 4;
pub const RL_SHELL_ESCAPE: u32 = 8;

pub const RL_SHELL_MOVIE_UNAVAILABLE: u32 = 0;
pub const RL_SHELL_MOVIE_FINISHED: u32 = 1;

pub const RL_SHELL_CREDITS_ENDED: u32 = 0;
pub const RL_SHELL_CREDITS_DRAW: u32 = 1;
pub const RL_SHELL_CREDITS_HOLD: u32 = 2;

pub const RL_INPUT_CURSOR: u32 = 1;
pub const RL_INPUT_DISPLAY_TEXT_BOUNDARY: u32 = 2;
pub const RL_INPUT_DISPLAY_SOUND_BOUNDARY: u32 = 4;

pub const PIXELS: usize = 800 * 600;
pub const PALETTE_BYTES: usize = 768;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlInputMessage {
    pub message: u32,
    pub wparam: u32,
    pub lparam: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlInputKey {
    pub vk: u32,
    pub value: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlDeclaredInput {
    pub size: u32,
    pub flags: u32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub messages: *const RlInputMessage,
    pub message_count: u32,
    pub keys: *const RlInputKey,
    pub key_count: u32,
}

impl Default for RlDeclaredInput {
    fn default() -> Self {
        Self {
            size: std::mem::size_of::<Self>() as u32,
            flags: 0,
            cursor_x: 0,
            cursor_y: 0,
            messages: std::ptr::null(),
            message_count: 0,
            keys: std::ptr::null(),
            key_count: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct RlFrameRegisters {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct RlFrameInput {
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub offscreen: u32,
    pub registers: RlFrameRegisters,
    pub saved: [u32; 3],
    pub msg: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlBloco {
    pub address: u32,
    pub length: u32,
    pub data: *const u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlMissaoInicio {
    pub version: u32,
    pub allocation_fill: u32,
    pub ai_clock_ms: u32,
    pub thread_id: u32,
    pub ticks_per_second: u32,
    pub epoch_ms: u32,
    pub execution_mode: u32,
    pub reserved: u32,
    pub iterations: [u32; 3],
    pub input: RlFrameInput,
}

impl Default for RlMissaoInicio {
    fn default() -> Self {
        Self {
            version: 1,
            allocation_fill: 0xa7,
            ai_clock_ms: 0x0012_3456,
            thread_id: 1,
            ticks_per_second: 20,
            epoch_ms: 0,
            execution_mode: RL_EXECUCAO_JOGO,
            reserved: 0,
            iterations: [5, 2, 1],
            input: RlFrameInput::default(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlEventsHostV1 {
    pub size: u32,
    pub version: u32,
    pub directory: *const u8,
    pub directory_length: u32,
    pub events: *const u8,
    pub events_length: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlRenderBytes {
    pub data: *const u8,
    pub length: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlRenderCache {
    pub keys: RlRenderBytes,
    pub payloads: RlRenderBytes,
    pub cursor: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlRenderResult {
    pub abi_status: i32,
    pub disposition: u32,
    pub generation: u64,
    pub frame_counter: u32,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub required_pixels: u32,
    pub required_palette: u32,
    pub palette_generation: u64,
    pub cache_generation: u64,
    pub stop_address: u32,
    pub stop_callee: [u8; 48],
    pub stop_reason: [u8; 192],
}

impl Default for RlRenderResult {
    fn default() -> Self {
        // SAFETY: every field of this `#[repr(C)]` plain-data struct is valid at zero.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlFrameEvent {
    pub kind: u32,
    pub id: u32,
    pub site: u32,
    pub args: [u32; 5],
    pub result: u32,
    pub registers: [u32; 4],
    pub stack: [u32; 4],
    pub stack_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlPlayerInputEvent {
    pub kind: u32,
    pub phase: u32,
    pub site: u32,
    pub frame: u32,
    pub values: [u32; 12],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlShellBytes {
    pub data: *const u8,
    pub length: u32,
}

impl Default for RlShellBytes {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            length: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlShellFile {
    pub path: RlShellBytes,
    pub bytes: RlShellBytes,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct RlShellInput {
    pub version: u32,
    pub keys: u32,
    pub mouse_x: u32,
    pub mouse_y: u32,
    pub mouse_ae: u32,
    pub mouse_af: u32,
    pub mouse_b0: u32,
    pub clock_7c: u32,
    pub animation_84: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlShellEvent {
    pub generation: u64,
    pub kind: u32,
    pub mode: u32,
    pub path_length: u32,
    pub reserved: u32,
    pub path: [u8; 512],
}

impl Default for RlShellEvent {
    fn default() -> Self {
        // SAFETY: plain data, all-zero is a valid invalid request.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlShellPlan {
    pub generation: u64,
    pub mode: u32,
    pub same_mode: u32,
    pub required_count: u32,
    pub required_mask: u32,
    pub required_calls: [u32; 4],
    pub background: [u8; 32],
}

impl Default for RlShellPlan {
    fn default() -> Self {
        // SAFETY: plain data.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlShellCredits {
    pub generation: u64,
    pub state: u32,
    pub frame: u32,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RlEventFlcV1 {
    pub name: [u8; 16],
    pub available: u32,
    pub header: [u8; 128],
}

impl Default for RlEventFlcV1 {
    fn default() -> Self {
        // SAFETY: plain data.
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RlEventFlcStateV1 {
    pub running: u32,
    pub modal: u32,
    pub index: u32,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub speed: u32,
}

extern "C" {
    pub fn rl_versao() -> u32;
    pub fn rl_render_contrato() -> u32;
    pub fn rl_salvar_versao() -> u32;
    pub fn rl_game_hud_background_version() -> u32;
    pub fn rl_missao_versao() -> u32;
    pub fn rl_shell_version() -> u32;

    pub fn rl_erro_mensagem(buffer: *mut u8, capacity: u32, required: *mut u32) -> i32;
    pub fn rl_liberar_mundo(world: u64) -> i32;
    pub fn rl_projetar_tudo(
        world: u64,
        buffer: *mut u8,
        capacity: u32,
        required: *mut u32,
    ) -> i32;

    pub fn rl_missao_iniciar_eventos_v1(
        assets: *const RlBloco,
        count: u32,
        name: *const u8,
        name_length: u32,
        config: *const RlMissaoInicio,
        host: *const RlEventsHostV1,
        output: *mut u64,
        input_out: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_missao_iniciar_menu_eventos_v1(
        assets: *const RlBloco,
        count: u32,
        config: *const RlMissaoInicio,
        host: *const RlEventsHostV1,
        output: *mut u64,
        input_out: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_eventos_reassociar_v1(world: u64, host: *const RlEventsHostV1) -> i32;
    pub fn rl_aplicar_dificuldade(world: u64, preset: u32) -> i32;
    pub fn rl_carregar_eventos(world: u64, bytes: *const u8, length: u32) -> i32;
    pub fn rl_execucao_definir(world: u64, mode: u32) -> i32;
    pub fn rl_execucao_modo(world: u64, output: *mut u32) -> i32;
    pub fn rl_load_tail_entrada_v1(
        world: u64,
        input: *mut RlFrameInput,
        declared: *const RlDeclaredInput,
    ) -> i32;
    pub fn rl_passo_quadro_entrada(
        world: u64,
        phase: u32,
        input: *mut RlFrameInput,
        declared: *const RlDeclaredInput,
    ) -> i32;
    pub fn rl_eventos_entrada(
        world: u64,
        buffer: *mut RlPlayerInputEvent,
        capacity: u32,
        required: *mut u32,
    ) -> i32;
    pub fn rl_missao_tela(world: u64, screen: *mut u32) -> i32;
    pub fn rl_missao_nome_proxima(
        world: u64,
        buffer: *mut u8,
        capacity: u32,
        required: *mut u32,
    ) -> i32;
    pub fn rl_missao_continuar(
        world: u64,
        name: *const u8,
        name_length: u32,
        eta: *const u8,
        eta_length: u32,
        monsters: *const u8,
        monsters_length: u32,
        input: *const RlFrameInput,
        output: *mut u64,
        input_out: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_missao_continuar_dificuldade(
        world: u64,
        name: *const u8,
        name_length: u32,
        eta: *const u8,
        eta_length: u32,
        monsters: *const u8,
        monsters_length: u32,
        input: *const RlFrameInput,
        preset: u32,
        output: *mut u64,
        input_out: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_entrada_quantidade_pendente(world: u64, count: *mut u32) -> i32;
    pub fn rl_entrada_passo_pendente(world: u64, input: *mut RlFrameInput) -> i32;

    pub fn rl_game_hud_background_initialize(
        version: u32,
        asset: *const u8,
        length: u32,
        surface: *mut u8,
        capacity: u32,
    ) -> i32;
    pub fn rl_render_criar_cd(
        world: u64,
        banks: *const RlRenderBytes,
        count: u32,
        initial_surface: *const RlRenderBytes,
        output: *mut u64,
    ) -> i32;
    pub fn rl_render_reset_cd(renderer: u64, surface: *const RlRenderBytes) -> i32;
    pub fn rl_render_reset(
        renderer: u64,
        surface: *const RlRenderBytes,
        cache: *const RlRenderCache,
    ) -> i32;
    pub fn rl_render_geracao(
        world: u64,
        generation: *mut u64,
        phase: *mut u32,
        frame: *mut u32,
    ) -> i32;
    pub fn rl_render_quadro(
        renderer: u64,
        expected_generation: u64,
        hud_flags: u32,
        indexed_buffer: *mut u8,
        indexed_capacity: u32,
        palette: *mut u8,
        palette_capacity: u32,
        result: *mut RlRenderResult,
    ) -> i32;
    pub fn rl_render_liberar(renderer: u64) -> i32;
    pub fn rl_render_cache(
        renderer: u64,
        keys: *mut u8,
        key_capacity: u32,
        payloads: *mut u8,
        payload_capacity: u32,
        key_required: *mut u32,
        payload_required: *mut u32,
        cursor: *mut u32,
    ) -> i32;
    pub fn rl_mundo_paleta_jogo(world: u64, palette: *mut u8, capacity: u32) -> i32;
    pub fn rl_mundo_tabelas_sombra_luz(
        world: u64,
        shadow: *mut u8,
        light: *mut u8,
    ) -> i32;
    pub fn rl_shell_imagem_cd(
        file: *const u8,
        length: u32,
        pixels: *mut u8,
        capacity: u32,
    ) -> i32;

    pub fn rl_salvar_bytes(
        world: u64,
        input: *const RlFrameInput,
        buffer: *mut u8,
        capacity: u32,
        required: *mut u32,
    ) -> i32;
    pub fn rl_carregar_arquivo(
        path: *const u8,
        path_length: u32,
        new_world: *mut u64,
        input: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_autosalvar_inicio_fase(world: u64, input: *const RlFrameInput) -> i32;
    pub fn rl_autosalvar_bytes(
        world: u64,
        buffer: *mut u8,
        capacity: u32,
        required: *mut u32,
    ) -> i32;

    pub fn rl_shell_create(
        version: u32,
        world: u64,
        assets: *const RlShellBytes,
        count: u32,
        surface: RlShellBytes,
        output: *mut u64,
    ) -> i32;
    pub fn rl_shell_plan(shell: u64, mode: u32, output: *mut RlShellPlan) -> i32;
    pub fn rl_shell_input(
        shell: u64,
        generation: u64,
        input: *const RlShellInput,
        output: *mut RlShellEvent,
    ) -> i32;
    pub fn rl_shell_draw(
        shell: u64,
        generation: u64,
        animation_84: u32,
        output_generation: *mut u64,
    ) -> i32;
    pub fn rl_shell_copy(
        shell: u64,
        generation: u64,
        pixels: *mut u8,
        pixel_capacity: u32,
        palette: *mut u8,
        palette_capacity: u32,
    ) -> i32;
    pub fn rl_shell_free(shell: u64) -> i32;
    pub fn rl_shell_paint_prepared(
        shell: u64,
        generation: u64,
        background: RlShellBytes,
        output_generation: *mut u64,
    ) -> i32;
    pub fn rl_shell_menu_init(
        shell: u64,
        generation: u64,
        files: *const RlShellFile,
        count: u32,
        output_generation: *mut u64,
    ) -> i32;
    pub fn rl_shell_campaign_reset(
        shell: u64,
        generation: u64,
        files: *const RlShellFile,
        count: u32,
        output_generation: *mut u64,
    ) -> i32;
    pub fn rl_shell_campaign_prepare(
        shell: u64,
        generation: u64,
        name: RlShellBytes,
        files: *const RlShellFile,
        count: u32,
        output_generation: *mut u64,
    ) -> i32;
    pub fn rl_shell_campaign_load_dificuldade(
        shell: u64,
        generation: u64,
        name: RlShellBytes,
        eta: RlShellBytes,
        monsters: RlShellBytes,
        input: *const RlFrameInput,
        preset: u32,
        output: *mut u64,
        input_out: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_shell_resume(
        shell: u64,
        generation: u64,
        input: *const RlFrameInput,
        output: *mut u64,
        input_out: *mut RlFrameInput,
    ) -> i32;
    pub fn rl_shell_defeat_finish(
        shell: u64,
        generation: u64,
        completion: u32,
        generation_out: *mut u64,
    ) -> i32;
    pub fn rl_shell_credits_frame(
        shell: u64,
        generation: u64,
        elapsed_ms: u32,
        output: *mut RlShellCredits,
    ) -> i32;
    pub fn rl_shell_export_world(shell: u64, generation: u64, output: *mut u64) -> i32;

    pub fn rl_eventos_quadro(
        world: u64,
        buffer: *mut RlFrameEvent,
        capacity: u32,
        required: *mut u32,
    ) -> i32;
    pub fn rl_eventos_flc_v1(
        world: u64,
        entries: *const RlEventFlcV1,
        count: u32,
    ) -> i32;
    pub fn rl_evento_flc_estado_v1(world: u64, output: *mut RlEventFlcStateV1) -> i32;
    pub fn rl_evento_flc_terminar_v1(world: u64) -> i32;
}

/// Read the thread-local ABI diagnostic without allocating on the engine side.
pub fn last_error() -> String {
    let mut required = 0u32;
    // SAFETY: a null, zero-capacity query is the documented size query.
    let code = unsafe { rl_erro_mensagem(std::ptr::null_mut(), 0, &mut required) };
    if code != RL_BUFFER_TOO_SMALL || required == 0 || required > 4096 {
        return String::new();
    }
    let mut bytes = vec![0u8; required as usize + 1];
    // SAFETY: `bytes` holds at least `required` writable bytes.
    let code = unsafe { rl_erro_mensagem(bytes.as_mut_ptr(), required, &mut required) };
    if code != RL_OK {
        return String::new();
    }
    let length = bytes.iter().position(|&b| b == 0).unwrap_or(required as usize);
    String::from_utf8_lossy(&bytes[..length]).into_owned()
}

/// A borrowed span used by the shell ABI.
pub fn shell_bytes(bytes: &[u8]) -> RlShellBytes {
    RlShellBytes {
        data: bytes.as_ptr(),
        length: bytes.len() as u32,
    }
}

pub fn c_bytes(bytes: &[u8]) -> *const c_void {
    bytes.as_ptr().cast()
}

pub fn c_char_ptr(text: &str) -> *const c_char {
    text.as_ptr().cast()
}
