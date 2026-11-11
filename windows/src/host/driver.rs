//! Host orchestration ported from `macos_live_driver.c`.
//!
//! This module owns the engine call order from `docs/WINDOWS_CLIENT.md` §3:
//! version checks, mission/menu startup, renderer creation, the load tail, the
//! per-frame 2/3 input phases, presentation capture, saves and the prepared
//! shell. It talks to the engine only through the `rl-ffi` C ABI.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::abi::*;
use crate::host::events::{music_requests, sound_events, EventFlcState, MusicRequest, SoundEvent};
use crate::host::input::{InputError, Keyboard, MouseQueue, QueuedInput, VK_CONTROL};
use crate::host::save;

const PRESENTATION_SIZE: usize = PIXELS + 1024 + 270336 + 4;
const MAX_FILE: u64 = 128 * 1024 * 1024;
const UNSUPPORTED_SCREEN: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Menu,
    Mission,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub frames: u64,
    pub generation: u64,
    pub counter: u32,
    pub phase_index: u32,
    pub disposition: u32,
    pub stopped: bool,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CampaignEnd {
    pub outcome: u32,
    pub stage: u8,
    pub has_next: bool,
    pub next: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveKind {
    Slot,
    StageStart,
}

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn file(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    let path = root.join(name);
    let metadata = fs::metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    if metadata.len() > MAX_FILE {
        return Err(format!("{}: file too large", path.display()));
    }
    fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
}

fn file_if_exists(root: &Path, name: &str) -> Option<Vec<u8>> {
    let path = root.join(name);
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE {
        return None;
    }
    fs::read(path).ok()
}

fn null_terminated(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

pub struct Driver {
    root: PathBuf,
    events_directory: Vec<u8>,
    events_bytes: Vec<u8>,
    world: u64,
    renderer: u64,
    shell: u64,
    shell_generation: u64,
    shell_mode: u32,
    shell_input: RlShellInput,
    shell_request: RlShellEvent,
    mission: String,
    difficulty: u32,
    menu_world: bool,
    /// Stage number of a pending prepared campaign start; 0 when not fresh.
    new_campaign: u32,
    slots: [[u8; 33]; 10],
    stage_presentation: Option<Vec<u8>>,
    pause_presentation: Option<Vec<u8>>,
    pause_palette: [u8; PALETTE_BYTES],
    stage_cursor: (i32, i32),
    pixels: Vec<u8>,
    palette: [u8; PALETTE_BYTES],
    game_background: Vec<u8>,
    game_background_ready: bool,
    input: RlFrameInput,
    phase2: RlFrameInput,
    keyboard: Keyboard,
    mouse: MouseQueue,
    cursor: (i32, i32),
    frame_messages: Vec<RlInputMessage>,
    frame_keys: Vec<RlInputKey>,
    input_next_phase: u32,
    status: Status,
    save_error: String,
    sound: Vec<SoundEvent>,
    declared_frames: u64,
    shell_held_keys: BTreeSet<u32>,
    shell_mouse_down: bool,
    shell_pending_move: Option<(u32, u32)>,
}

impl Drop for Driver {
    fn drop(&mut self) {
        // SAFETY: handles are owned by this driver and freed once.
        unsafe {
            if self.shell != 0 {
                rl_shell_free(self.shell);
                self.shell = 0;
            }
            if self.renderer != 0 {
                rl_render_liberar(self.renderer);
                self.renderer = 0;
            }
            if self.world != 0 {
                rl_liberar_mundo(self.world);
                self.world = 0;
            }
        }
    }
}

impl Driver {
    pub fn create(
        root: &Path,
        selection: Selection,
        mission: &str,
        difficulty: u32,
        events_directory: &str,
    ) -> Result<Self, String> {
        Self::check_versions(selection)?;
        if selection == Selection::Mission
            && (mission.is_empty() || mission.contains('/') || mission.contains('\\'))
        {
            return Err("Nome de missão inválido".into());
        }
        let events_bytes = file(root, "EVENTS.INI")?;
        if events_bytes.is_empty() || events_bytes.len() > 1024 * 1024 {
            return Err("EVENTS.INI inválido".into());
        }
        let mut driver = Self {
            root: root.to_path_buf(),
            events_directory: events_directory.as_bytes().to_vec(),
            events_bytes,
            world: 0,
            renderer: 0,
            shell: 0,
            shell_generation: 0,
            shell_mode: 0,
            shell_input: RlShellInput {
                version: 1,
                ..Default::default()
            },
            shell_request: RlShellEvent::default(),
            mission: mission.to_owned(),
            difficulty,
            menu_world: false,
            new_campaign: 0,
            slots: [[0; 33]; 10],
            stage_presentation: None,
            pause_presentation: None,
            pause_palette: [0; PALETTE_BYTES],
            stage_cursor: (400, 300),
            pixels: vec![0; PIXELS],
            palette: [0; PALETTE_BYTES],
            game_background: vec![0; PIXELS],
            game_background_ready: false,
            input: Self::base_input(),
            phase2: Self::phase2_input(),
            keyboard: Keyboard::default(),
            mouse: MouseQueue::default(),
            cursor: (0, 0),
            frame_messages: Vec::with_capacity(4096),
            frame_keys: Vec::with_capacity(256),
            input_next_phase: 0,
            status: Status {
                phase_index: u32::MAX,
                ..Default::default()
            },
            save_error: String::new(),
            sound: Vec::new(),
            declared_frames: 0,
            shell_held_keys: BTreeSet::new(),
            shell_mouse_down: false,
            shell_pending_move: None,
        };
        driver.cursor = (driver.input.cursor_x, driver.input.cursor_y);
        match selection {
            Selection::Mission => {
                driver.start_mission()?;
                driver.renderer = driver.new_renderer(driver.world, None)?;
                if driver.renderer == 0 {
                    return Err("renderer create".into());
                }
            }
            Selection::Menu => {
                driver.start_menu()?;
                driver.shell_open(RL_SHELL_MAIN)?;
            }
        }
        driver.load_palette()?;
        Ok(driver)
    }

    fn check_versions(selection: Selection) -> Result<(), String> {
        // SAFETY: version queries take no arguments.
        unsafe {
            let version = rl_versao();
            if (version >> 16) != 1 || (version & 0xffff) < 6 {
                return Err(format!("ABI 1.6 required (got 0x{version:08x})"));
            }
            if rl_render_contrato() != RENDER_CONTRACT {
                return Err("Graphics 1.9 required".into());
            }
            if rl_salvar_versao() != SAVE_FORMAT_VERSION {
                return Err("Save extension v1 required".into());
            }
            if rl_game_hud_background_version() != GAME_HUD_BACKGROUND_VERSION {
                return Err("Gameplay background extension v1 required".into());
            }
            if rl_shell_version() != SHELL_VERSION {
                return Err("Prepared shell v1 required".into());
            }
            if selection == Selection::Mission && rl_missao_versao() != 1 {
                return Err("Mission extension v1 required".into());
            }
        }
        Ok(())
    }

    fn base_input() -> RlFrameInput {
        RlFrameInput {
            cursor_x: 0,
            cursor_y: 0,
            offscreen: 0x6007_6000,
            registers: RlFrameRegisters {
                eax: 1,
                esp: 0x0200_7ec0,
                ..Default::default()
            },
            saved: [0; 3],
            msg: [0x0300_0000, 0, 0, 0],
        }
    }

    fn phase2_input() -> RlFrameInput {
        RlFrameInput {
            cursor_x: 0,
            cursor_y: 0,
            offscreen: 0x6007_6000,
            registers: RlFrameRegisters {
                ebx: 3,
                edx: 0x0200_7fe8,
                esp: 0x0200_7fe8,
                ..Default::default()
            },
            saved: [0; 3],
            msg: [0x0300_0000, 0, 0, 0],
        }
    }

    fn events_host(&self) -> RlEventsHostV1 {
        RlEventsHostV1 {
            size: std::mem::size_of::<RlEventsHostV1>() as u32,
            version: 1,
            directory: self.events_directory.as_ptr(),
            directory_length: self.events_directory.len() as u32,
            events: self.events_bytes.as_ptr(),
            events_length: self.events_bytes.len() as u32,
        }
    }

    fn start_mission(&mut self) -> Result<(), String> {
        const NAMES: [&str; 10] = [
            "RISING.EXE",
            "RISING.INI",
            "RISING.RUL",
            "RISING.SBL",
            "TABLEAU.COL",
            "COULEUR.MAP",
            "PLATGAME.PIE",
            "MONSTRE.INI",
            "TEXTES.TXT",
            "MONDE01.ETA",
        ];
        let mut owned = Vec::with_capacity(10);
        for (index, default_name) in NAMES.iter().enumerate() {
            let name = if index == 9 {
                self.mission.as_str()
            } else {
                *default_name
            };
            owned.push(file(&self.root, name)?);
        }
        let blocks: Vec<RlBloco> = owned
            .iter()
            .map(|bytes| RlBloco {
                address: 0,
                length: bytes.len() as u32,
                data: bytes.as_ptr(),
            })
            .collect();
        let mut config = RlMissaoInicio::default();
        config.input = self.input;
        config.input.cursor_x = 400;
        config.input.cursor_y = 300;
        let host = self.events_host();
        let mut world = 0u64;
        let mut input = RlFrameInput::default();
        // SAFETY: every pointer references memory that outlives the call; all
        // spans are length-delimited and the outputs are distinct writable locals.
        let code = unsafe {
            rl_missao_iniciar_eventos_v1(
                blocks.as_ptr(),
                blocks.len() as u32,
                self.mission.as_ptr(),
                self.mission.len() as u32,
                &config,
                &host,
                &mut world,
                &mut input,
            )
        };
        if code != RL_OK {
            return Err(self.failure("MissionLoad", code));
        }
        self.world = world;
        self.input = input;
        self.cursor = (400, 300);
        // SAFETY: `world` is the fresh handle published above.
        let code = unsafe { rl_aplicar_dificuldade(self.world, self.difficulty) };
        if code != RL_OK {
            return Err(self.failure("Dificuldade", code));
        }
        self.load_events(false)?;
        Ok(())
    }

    fn start_menu(&mut self) -> Result<(), String> {
        const NAMES: [&str; 9] = [
            "RISING.EXE",
            "RISING.INI",
            "RISING.RUL",
            "RISING.SBL",
            "TABLEAU.COL",
            "COULEUR.MAP",
            "PLATGAME.PIE",
            "MONSTRE.INI",
            "TEXTES.TXT",
        ];
        let mut owned = Vec::with_capacity(9);
        for name in NAMES {
            owned.push(file(&self.root, name)?);
        }
        let blocks: Vec<RlBloco> = owned
            .iter()
            .map(|bytes| RlBloco {
                address: 0,
                length: bytes.len() as u32,
                data: bytes.as_ptr(),
            })
            .collect();
        let mut config = RlMissaoInicio::default();
        config.input = self.input;
        config.input.cursor_x = 400;
        config.input.cursor_y = 300;
        let host = self.events_host();
        let mut world = 0u64;
        let mut input = RlFrameInput::default();
        // SAFETY: same contract as `start_mission`; the menu ABI takes nine blocks.
        let code = unsafe {
            rl_missao_iniciar_menu_eventos_v1(
                blocks.as_ptr(),
                blocks.len() as u32,
                &config,
                &host,
                &mut world,
                &mut input,
            )
        };
        if code != RL_OK {
            return Err(self.failure("MenuInit", code));
        }
        self.world = world;
        self.input = input;
        self.cursor = (input.cursor_x, input.cursor_y);
        self.menu_world = true;
        Ok(())
    }

    fn load_events(&mut self, inherited: bool) -> Result<(), String> {
        if !self.events_bytes.is_empty() {
            let host = self.events_host();
            // SAFETY: the host descriptor copies its buffers during the call.
            let code = unsafe { rl_eventos_reassociar_v1(self.world, &host) };
            if code != RL_OK {
                return Err(self.failure("EVENTS host", code));
            }
        } else if inherited {
            return Err("EVENTS.INI ausente nos dados nativos".into());
        }
        self.declare_flc_events(self.world)
    }

    fn declare_flc_events(&mut self, world: u64) -> Result<(), String> {
        let mut names: BTreeSet<String> = BTreeSet::new();
        if let Ok(bytes) = file(&self.root, "EVENTS.INI") {
            let mut section = false;
            for line in bytes.split(|&byte| byte == b'\n' || byte == b'\r') {
                let text = String::from_utf8_lossy(line);
                let trimmed = text.trim();
                if trimmed.starts_with('[') {
                    section = trimmed.eq_ignore_ascii_case("[EVENEMENT]");
                    continue;
                }
                if !section {
                    continue;
                }
                if let Some((_, value)) = trimmed.split_once('=') {
                    let value = value.trim().to_ascii_uppercase();
                    if !(1..=15).contains(&value.len())
                        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    {
                        return Err("EVENTS.INI: nome de evento inválido".into());
                    }
                    names.insert(value);
                }
            }
        }
        if names.is_empty() {
            return Ok(());
        }
        let events_folder = self.root.parent().unwrap_or(&self.root).join("events");
        let mut table = Vec::with_capacity(names.len());
        for name in &names {
            let mut entry = RlEventFlcV1::default();
            entry.name[..name.len()].copy_from_slice(name.as_bytes());
            if let Some(bytes) = file_if_exists(&events_folder, &format!("{name}.FLC")) {
                if bytes.len() >= 128 {
                    entry.available = 1;
                    entry.header.copy_from_slice(&bytes[..128]);
                }
            }
            table.push(entry);
        }
        // SAFETY: the table is a contiguous, correctly sized array copied by the ABI.
        let code = unsafe { rl_eventos_flc_v1(world, table.as_ptr(), table.len() as u32) };
        if code != RL_OK {
            let detail = last_error();
            if detail.contains("already declared") {
                return Ok(());
            }
            return Err(format!("Tabela de eventos FLC recusada: {detail}"));
        }
        Ok(())
    }

    pub fn advance(&mut self) -> Result<(), String> {
        if self.shell != 0 {
            return Err("shell is active".into());
        }
        let mut screen = 0u32;
        // SAFETY: `screen` is a writable local.
        let code = unsafe { rl_missao_tela(self.world, &mut screen) };
        if code != RL_OK {
            return Err(self.failure("tela da campanha", code));
        }
        if screen == RL_SHELL_PAUSE || screen == RL_SHELL_RESULT {
            return Ok(());
        }
        if self.status.phase_index == u32::MAX {
            return self.first_phase();
        }
        let (_, _, _) = self.render_generation()?;
        self.render_current()?;
        if self.drain_saved_input()? {
            return Ok(());
        }
        self.human_phase(2)?;
        self.human_phase(3)?;
        let (_, _, counter) = self.render_generation()?;
        self.status.counter = counter;
        Ok(())
    }

    fn first_phase(&mut self) -> Result<(), String> {
        let mut keys: Vec<RlInputKey> = self
            .keyboard
            .held_keys(false)
            .into_iter()
            .map(|(vk, value)| RlInputKey { vk, value })
            .collect();
        let declared = RlDeclaredInput {
            size: std::mem::size_of::<RlDeclaredInput>() as u32,
            flags: 6,
            cursor_x: 0,
            cursor_y: 0,
            messages: std::ptr::null(),
            message_count: 0,
            keys: keys.as_mut_ptr(),
            key_count: keys.len() as u32,
        };
        // SAFETY: `keys` and `declared` outlive the call; `input` is writable.
        let code = unsafe { rl_load_tail_entrada_v1(self.world, &mut self.input, &declared) };
        if code != RL_OK {
            return Err(self.failure("load_tail", code));
        }
        self.input.registers = self.phase2.registers;
        self.first_game_draw()?;
        // SAFETY: the world is live and `input` is the exact carried context.
        let code = unsafe { rl_autosalvar_inicio_fase(self.world, &self.input) };
        if code != RL_OK {
            return Err(self.failure("captura automática no início da fase", code));
        }
        let presentation = self.capture_presentation()?;
        self.stage_presentation = Some(presentation);
        self.stage_cursor = self.cursor;
        self.status.phase_index = 0;
        Ok(())
    }

    fn first_game_draw(&mut self) -> Result<(), String> {
        let (generation, _, _) = self.render_generation()?;
        self.render_frame(generation)?;
        self.status.generation = generation;
        Ok(())
    }

    fn render_generation(&self) -> Result<(u64, u32, u32), String> {
        let mut generation = 0u64;
        let mut phase = 0u32;
        let mut counter = 0u32;
        // SAFETY: three distinct writable locals.
        let code = unsafe {
            rl_render_geracao(self.world, &mut generation, &mut phase, &mut counter)
        };
        if code != RL_OK {
            return Err(self.failure("render generation", code));
        }
        Ok((generation, phase, counter))
    }

    fn render_current(&mut self) -> Result<(), String> {
        let (generation, phase, counter) = self.render_generation()?;
        self.status.counter = counter;
        // A world resumed from the original pause menu has no frame phase yet:
        // its retained picture is kept and the next complete frame draws.
        if phase != 0 {
            self.render_frame(generation)?;
        }
        self.status.generation = generation;
        Ok(())
    }

    fn render_frame(&mut self, generation: u64) -> Result<(), String> {
        let mut pixels = vec![0u8; PIXELS];
        let mut palette = [0u8; PALETTE_BYTES];
        let mut result = RlRenderResult::default();
        // SAFETY: both buffers are writable for their exact capacities, and the
        // result is a distinct writable local; the renderer is live.
        let code = unsafe {
            rl_render_quadro(
                self.renderer,
                generation,
                1,
                pixels.as_mut_ptr(),
                pixels.len() as u32,
                palette.as_mut_ptr(),
                palette.len() as u32,
                &mut result,
            )
        };
        if code != RL_OK {
            return Err(self.failure("Rust render", code));
        }
        if result.abi_status != 0
            || result.generation != generation
            || result.width != 800
            || result.height != 600
            || result.stride != 800
            || result.required_pixels != 480000
            || result.required_palette != 768
            || result.palette_generation != 1
            || result.disposition > 2
        {
            self.stop("NativeRenderResultContractMismatch");
            return Err(self.status.message.clone());
        }
        self.status.disposition = result.disposition;
        if result.disposition == 2 {
            let callee = null_terminated(&result.stop_callee);
            let reason = null_terminated(&result.stop_reason);
            self.stop(&format!(
                "{callee} at 0x{:08x}: {reason}",
                result.stop_address
            ));
            return Err(self.status.message.clone());
        }
        self.pixels = pixels;
        self.palette = palette;
        Ok(())
    }

    fn drain_saved_input(&mut self) -> Result<bool, String> {
        let mut pending = 0u32;
        // SAFETY: writable local.
        let code = unsafe { rl_entrada_quantidade_pendente(self.world, &mut pending) };
        if code != RL_OK {
            return Err(self.failure("consulta de entradas salvas", code));
        }
        if pending == 0 {
            return Ok(false);
        }
        loop {
            // SAFETY: `input` is a writable carried context.
            let code = unsafe { rl_entrada_passo_pendente(self.world, &mut self.input) };
            if code != RL_OK {
                return Err(self.failure("entrada salva", code));
            }
            let (_, phase, counter) = self.render_generation()?;
            if phase == 2 {
                self.cursor = (self.input.cursor_x, self.input.cursor_y);
            }
            self.poll_sound(phase);
            if phase == 3 {
                self.status.frames += 1;
                self.declared_frames += 1;
                self.status.counter = counter;
                self.status.phase_index = (2 * self.status.frames) as u32;
                self.input_next_phase = 2;
                return Ok(true);
            }
            // SAFETY: writable local.
            let code = unsafe { rl_entrada_quantidade_pendente(self.world, &mut pending) };
            if code != RL_OK {
                return Err(self.failure("consulta de entradas salvas", code));
            }
            if pending == 0 {
                return Err(self.failure(
                    "SavedInputMissingPhase3",
                    RL_UNSUPPORTED_PATH,
                ));
            }
        }
    }

    fn human_phase(&mut self, phase: u32) -> Result<(), String> {
        if phase == 3 {
            let declared = RlDeclaredInput {
                size: std::mem::size_of::<RlDeclaredInput>() as u32,
                flags: 6,
                ..Default::default()
            };
            return self.input_phase(3, declared);
        }
        if phase != 2 || self.input_next_phase == 3 {
            return Err("invalid human phase".into());
        }
        let frame = self.status.frames + 1;
        let mut cursor = self.cursor;
        let messages = self.mouse.take_due(frame, &mut cursor);
        let keys: Vec<RlInputKey> = self
            .keyboard
            .held_keys(true)
            .into_iter()
            .map(|(vk, value)| RlInputKey { vk, value })
            .collect();
        self.frame_messages.clear();
        self.frame_messages.extend_from_slice(&messages);
        self.frame_keys.clear();
        self.frame_keys.extend_from_slice(&keys);
        let declared = RlDeclaredInput {
            size: std::mem::size_of::<RlDeclaredInput>() as u32,
            flags: 7,
            cursor_x: cursor.0,
            cursor_y: cursor.1,
            messages: self.frame_messages.as_ptr(),
            message_count: messages.len() as u32,
            keys: self.frame_keys.as_ptr(),
            key_count: keys.len() as u32,
        };
        self.input_phase(2, declared)
    }

    fn input_phase(&mut self, phase: u32, declared: RlDeclaredInput) -> Result<(), String> {
        let expected = if self.input_next_phase == 0 {
            2
        } else {
            self.input_next_phase
        };
        if phase != expected {
            return Err(format!("input phase {phase} out of order"));
        }
        // SAFETY: `declared` points into `self.frame_*` buffers that outlive the call.
        let code = unsafe {
            rl_passo_quadro_entrada(self.world, phase, &mut self.input, &declared)
        };
        if code != RL_OK {
            return Err(self.failure(
                if phase == 2 {
                    "entrada entre quadros"
                } else {
                    "entrada CalculGame"
                },
                code,
            ));
        }
        if phase == 2 {
            self.cursor = (self.input.cursor_x, self.input.cursor_y);
        }
        self.status.phase_index = (2 * self.status.frames + if phase == 2 { 1 } else { 2 }) as u32;
        self.input_next_phase = if phase == 2 { 3 } else { 2 };
        self.poll_sound(phase);
        if phase == 3 {
            self.status.frames += 1;
            self.declared_frames += 1;
            let (_, _, counter) = self.render_generation()?;
            self.status.counter = counter;
        }
        Ok(())
    }

    fn poll_sound(&mut self, phase: u32) {
        let mut capacity = 1024usize;
        let mut events: Vec<RlPlayerInputEvent>;
        loop {
            events = vec![RlPlayerInputEvent::default(); capacity];
            let mut required = 0u32;
            // SAFETY: the buffer is writable for `capacity` records.
            let code = unsafe {
                rl_eventos_entrada(
                    self.world,
                    events.as_mut_ptr(),
                    capacity as u32,
                    &mut required,
                )
            };
            if code == RL_BUFFER_TOO_SMALL && required as usize > capacity && required <= 65536 {
                capacity = required as usize;
                continue;
            }
            if code != RL_OK {
                return;
            }
            events.truncate(required as usize);
            let frame = (self.status.frames + 1) as u32;
            self.sound.extend(sound_events(&events, frame, phase));
            return;
        }
    }

    pub fn poll_music(&mut self) -> Vec<MusicRequest> {
        let mut capacity = 1024usize;
        loop {
            let mut events = vec![RlFrameEvent::default(); capacity];
            let mut required = 0u32;
            // SAFETY: the buffer is writable for `capacity` records.
            let code = unsafe {
                rl_eventos_quadro(self.world, events.as_mut_ptr(), capacity as u32, &mut required)
            };
            if code == RL_BUFFER_TOO_SMALL && required as usize > capacity && required <= 65536 {
                capacity = required as usize;
                continue;
            }
            if code != RL_OK {
                return Vec::new();
            }
            events.truncate(required as usize);
            return music_requests(&events);
        }
    }

    pub fn drain_sound(&mut self) -> Vec<SoundEvent> {
        std::mem::take(&mut self.sound)
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn world(&self) -> u64 {
        self.world
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn palette(&self) -> &[u8] {
        &self.palette
    }

    pub fn mission(&self) -> &str {
        &self.mission
    }

    pub fn difficulty(&self) -> u32 {
        self.difficulty
    }

    pub fn set_difficulty(&mut self, difficulty: u32) -> Result<(), String> {
        if difficulty > 2 {
            return Err("invalid difficulty".into());
        }
        self.difficulty = difficulty;
        Ok(())
    }

    pub fn menu_world(&self) -> bool {
        self.menu_world
    }

    pub fn save_error(&self) -> &str {
        &self.save_error
    }

    pub fn pending_input(&self) -> usize {
        self.mouse.pending_count()
    }

    pub fn left_held(&self) -> bool {
        self.mouse.left_held
    }

    pub fn frames(&self) -> u64 {
        self.status.frames
    }

    fn stop(&mut self, message: &str) {
        self.status.stopped = true;
        self.status.message = format!(
            "{message} · quadro {} · ABI {}",
            self.status.frames + 1,
            last_error()
        );
    }

    fn failure(&mut self, context: &str, code: i32) -> String {
        self.status.stopped = true;
        let detail = last_error();
        self.status.message = format!(
            "{context} · quadro {} · ABI {code}: {detail}",
            self.status.frames + 1
        );
        self.status.message.clone()
    }

    fn fail_value<T>(&mut self, context: &str, code: i32) -> Result<T, String> {
        Err(self.failure(context, code))
    }

    // ----- rendering resources ------------------------------------------------

    fn game_background(&mut self) -> Result<(), String> {
        if self.game_background_ready {
            return Ok(());
        }
        let pie = file(&self.root, "PLATGAME.PIE")?;
        let mut surface = vec![0u8; PIXELS];
        // SAFETY: `pie` is readable for its length and `surface` is writable.
        let code = unsafe {
            rl_game_hud_background_initialize(
                GAME_HUD_BACKGROUND_VERSION,
                pie.as_ptr(),
                pie.len() as u32,
                surface.as_mut_ptr(),
                surface.len() as u32,
            )
        };
        if code != RL_OK {
            self.status.stopped = true;
            return self.fail_value("fundo original do jogo", code);
        }
        self.game_background = surface;
        self.game_background_ready = true;
        Ok(())
    }

    fn render_assets(&self, count: usize) -> Result<Vec<Vec<u8>>, String> {
        let mut assets = Vec::with_capacity(count);
        for index in 0..count {
            assets.push(file(&self.root, &format!("asset{index}.bin"))?);
        }
        Ok(assets)
    }

    fn new_renderer(&mut self, world: u64, presentation: Option<&[u8]>) -> Result<u64, String> {
        let assets = self.render_assets(7)?;
        let mut descriptors: Vec<RlRenderBytes> = assets
            .iter()
            .map(|bytes| RlRenderBytes {
                data: bytes.as_ptr(),
                length: bytes.len() as u32,
            })
            .collect();
        let mut renderer = 0u64;
        match presentation {
            Some(presentation) if presentation.len() >= PRESENTATION_SIZE => {
                let surface = RlRenderBytes {
                    data: presentation.as_ptr(),
                    length: PIXELS as u32,
                };
                // SAFETY: the seven bank descriptors and the surface outlive the call.
                let code = unsafe {
                    rl_render_criar_cd(
                        world,
                        descriptors.as_mut_ptr(),
                        descriptors.len() as u32,
                        &surface,
                        &mut renderer,
                    )
                };
                if code != RL_OK {
                    self.status.stopped = true;
                    return self.fail_value("renderer create", code);
                }
                let cache = RlRenderCache {
                    keys: RlRenderBytes {
                        data: presentation[PIXELS..].as_ptr(),
                        length: 1024,
                    },
                    payloads: RlRenderBytes {
                        data: presentation[PIXELS + 1024..].as_ptr(),
                        length: 270336,
                    },
                    cursor: word(&presentation[PIXELS + 1024 + 270336..]),
                };
                // SAFETY: both spans point into the retained presentation.
                let code = unsafe { rl_render_reset(renderer, &surface, &cache) };
                if code != RL_OK {
                    // SAFETY: the renderer was published above and is not reused.
                    unsafe { rl_render_liberar(renderer) };
                    return self.fail_value("renderer reset", code);
                }
                let mut pixels = vec![0u8; PIXELS];
                pixels[..].copy_from_slice(&presentation[..PIXELS]);
                self.pixels = pixels;
            }
            _ => {
                self.game_background()?;
                let surface = RlRenderBytes {
                    data: self.game_background.as_ptr(),
                    length: PIXELS as u32,
                };
                // SAFETY: descriptors point into `assets`/`self.game_background`.
                let code = unsafe {
                    rl_render_criar_cd(
                        world,
                        descriptors.as_mut_ptr(),
                        descriptors.len() as u32,
                        &surface,
                        &mut renderer,
                    )
                };
                if code != RL_OK {
                    self.status.stopped = true;
                    return self.fail_value("renderer create", code);
                }
                self.pixels.copy_from_slice(&self.game_background);
            }
        }
        Ok(renderer)
    }

    fn capture_presentation(&mut self) -> Result<Vec<u8>, String> {
        let mut presentation = vec![0u8; PRESENTATION_SIZE];
        let mut keys = 0u32;
        let mut payloads = 0u32;
        let mut cursor = 0u32;
        // SAFETY: both buffers are writable for their fixed capacities.
        let code = unsafe {
            rl_render_cache(
                self.renderer,
                presentation[PIXELS..].as_mut_ptr(),
                1024,
                presentation[PIXELS + 1024..].as_mut_ptr(),
                270336,
                &mut keys,
                &mut payloads,
                &mut cursor,
            )
        };
        if code != RL_OK {
            return self.fail_value("apresentação da partida", code);
        }
        if keys != 1024 || payloads != 270336 {
            return Err("presentation cache size".into());
        }
        presentation[..PIXELS].copy_from_slice(&self.pixels);
        presentation[PIXELS + 1024 + 270336..].copy_from_slice(&cursor.to_le_bytes());
        Ok(presentation)
    }

    fn load_palette(&mut self) -> Result<(), String> {
        // SAFETY: the palette is writable for 768 bytes.
        let code = unsafe {
            rl_mundo_paleta_jogo(self.world, self.palette.as_mut_ptr(), self.palette.len() as u32)
        };
        if code != RL_OK {
            return self.fail_value("paleta do jogo", code);
        }
        Ok(())
    }

    // ----- host input ---------------------------------------------------------

    pub fn mouse(
        &mut self,
        message: u32,
        x: i32,
        y: i32,
        wparam: u32,
    ) -> Result<(), String> {
        if self.shell != 0 {
            return self.shell_mouse(message, x, y);
        }
        let frame = self.status.frames + 1;
        match self.mouse.mouse(message, x, y, wparam, frame) {
            Ok(_) => Ok(()),
            Err(InputError::InvalidMouseEvent) => Err("InvalidMacMouseEvent".into()),
            Err(error) => Err(format!("{error:?}")),
        }
    }

    pub fn key(&mut self, vk: u32, down: bool, control: bool) -> Result<(), String> {
        if self.shell != 0 {
            return self.shell_key(vk, down);
        }
        let frame = self.status.frames + 1;
        let control = control || self.keyboard.is_held(VK_CONTROL);
        match self.keyboard.key(vk, down, frame, control) {
            Ok(queued) => match self.mouse.schedule(queued) {
                Ok(()) => Ok(()),
                Err(error) => Err(format!("{error:?}")),
            },
            Err(InputError::UnsupportedKey) => Err("UnsupportedMacVirtualKey".into()),
            Err(error) => Err(format!("{error:?}")),
        }
    }

    pub fn release_input(&mut self) {
        if let Some(queued) = self.mouse.release_at_last(self.status.frames + 1) {
            let _ = self.mouse.schedule(queued);
        }
        self.keyboard.clear();
    }

    // ----- saves --------------------------------------------------------------

    fn serialize(&mut self, kind: SaveKind) -> Result<Vec<u8>, i32> {
        let mut capacity = 4 * 1024 * 1024usize;
        for _ in 0..3 {
            let mut bytes = vec![0u8; capacity];
            let mut required = 0u32;
            // SAFETY: the buffer is writable for its capacity.
            let code = unsafe {
                match kind {
                    SaveKind::Slot => rl_salvar_bytes(
                        self.world,
                        &self.input,
                        bytes.as_mut_ptr(),
                        capacity as u32,
                        &mut required,
                    ),
                    SaveKind::StageStart => rl_autosalvar_bytes(
                        self.world,
                        bytes.as_mut_ptr(),
                        capacity as u32,
                        &mut required,
                    ),
                }
            };
            if code == RL_OK {
                bytes.truncate(required as usize);
                self.save_error.clear();
                return Ok(bytes);
            }
            if code != RL_BUFFER_TOO_SMALL || required as usize <= capacity {
                self.save_error = last_error();
                return Err(code);
            }
            capacity = required as usize + required as usize / 8 + 65536;
        }
        Err(RL_SAVE_INVALID)
    }

    /// Captures the exact ABI slot bytes and writes them durably. The previous
    /// slot survives any failure before the rename.
    pub fn save_slot(&mut self, path: &Path, kind: SaveKind) -> Result<(), i32> {
        if kind == SaveKind::Slot {
            let mut screen = 0u32;
            // SAFETY: writable local.
            let code = unsafe { rl_missao_tela(self.world, &mut screen) };
            if code == RL_OK && screen != RL_SHELL_PAUSE {
                if let Err(error) = self.first_game_draw() {
                    self.save_error = error;
                    return Err(RL_SAVE_BUSY);
                }
            }
        }
        let bytes = self.serialize(kind)?;
        if let Err(error) = save::write_durable(path, &bytes) {
            self.save_error = format!("Não foi possível salvar · {}: {error}", path.display());
            return Err(RL_IO_ERROR);
        }
        self.save_error.clear();
        Ok(())
    }

    /// Loads a slot into a fresh world and rebinds a renderer with the CD banks.
    pub fn load_slot(&mut self, path: &Path) -> Result<(), i32> {
        let path_bytes = path.to_string_lossy().into_owned().into_bytes();
        let mut world = 0u64;
        let mut input = RlFrameInput::default();
        // SAFETY: the path span and outputs are valid for this call.
        let code = unsafe {
            rl_carregar_arquivo(
                path_bytes.as_ptr(),
                path_bytes.len() as u32,
                &mut world,
                &mut input,
            )
        };
        if code != RL_OK {
            self.save_error = last_error();
            return Err(code);
        }
        let mut mode = 0u32;
        // SAFETY: writable local.
        let code = unsafe { rl_execucao_modo(world, &mut mode) };
        if code == RL_OK {
            let mut generation = 0u64;
            let mut phase = 0u32;
            let mut frame = 0u32;
            // SAFETY: distinct writable locals.
            let code = unsafe { rl_render_geracao(world, &mut generation, &mut phase, &mut frame) };
            if code != RL_OK
                || mode != RL_EXECUCAO_JOGO
                || (phase != 1 && phase != 3)
            {
                // SAFETY: `world` was published by the load and is not retained.
                unsafe { rl_liberar_mundo(world) };
                self.save_error = "A partida precisa estar no modo Jogar e em um quadro completo.".into();
                return Err(RL_SAVE_BUSY);
            }
        }
        if !self.events_bytes.is_empty() {
            let host = self.events_host();
            // SAFETY: the descriptor is copied; the world is a fresh candidate.
            unsafe { rl_eventos_reassociar_v1(world, &host) };
        }
        if let Err(error) = self.declare_flc_events(world) {
            self.save_error = error;
            // SAFETY: candidate world is not retained.
            unsafe { rl_liberar_mundo(world) };
            return Err(RL_SAVE_BUSY);
        }
        let renderer = match self.new_renderer(world, None) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.save_error = error;
                // SAFETY: candidate world is not retained.
                unsafe { rl_liberar_mundo(world) };
                return Err(RL_SAVE_BUSY);
            }
        };
        let mut pixels = vec![0u8; PIXELS];
        let mut palette = [0u8; PALETTE_BYTES];
        let mut generation = 0u64;
        let mut phase = 0u32;
        let mut frame = 0u32;
        let mut result = RlRenderResult::default();
        // SAFETY: outputs are distinct writable locals.
        let code = unsafe { rl_render_geracao(world, &mut generation, &mut phase, &mut frame) };
        let code = if code == RL_OK {
            // SAFETY: buffers and result are writable and disjoint.
            unsafe {
                rl_render_quadro(
                    renderer,
                    generation,
                    1,
                    pixels.as_mut_ptr(),
                    pixels.len() as u32,
                    palette.as_mut_ptr(),
                    palette.len() as u32,
                    &mut result,
                )
            }
        } else {
            code
        };
        if code != RL_OK || result.disposition == 2 {
            self.save_error = last_error();
            // SAFETY: both handles were published above and are candidates only.
            unsafe {
                rl_render_liberar(renderer);
                rl_liberar_mundo(world);
            }
            return Err(RL_SAVE_BUSY);
        }
        // Publish the complete candidate only now.
        // SAFETY: the previous handles are owned by this driver.
        unsafe {
            if self.renderer != 0 {
                rl_render_liberar(self.renderer);
            }
            if self.world != 0 {
                rl_liberar_mundo(self.world);
            }
        }
        self.world = world;
        self.renderer = renderer;
        self.input = input;
        self.pixels = pixels;
        self.palette = palette;
        self.status = Status {
            frames: frame as u64,
            counter: frame,
            generation,
            phase_index: frame * 2,
            disposition: result.disposition,
            stopped: false,
            message: String::new(),
        };
        self.input_next_phase = 2;
        self.keyboard.clear();
        self.mouse.clear();
        self.cursor = (input.cursor_x, input.cursor_y);
        self.menu_world = false;
        self.new_campaign = 0;
        self.pause_presentation = None;
        self.save_error.clear();
        Ok(())
    }

    pub fn save_bytes_for_test(&mut self, kind: SaveKind) -> Result<Vec<u8>, String> {
        self.serialize(kind).map_err(|code| {
            if self.save_error.is_empty() {
                format!("save failed: {code}")
            } else {
                self.save_error.clone()
            }
        })
    }

    // ----- campaign -----------------------------------------------------------

    pub fn campaign_screen(&mut self) -> Result<u32, String> {
        let mut screen = 0u32;
        // SAFETY: writable local.
        let code = unsafe { rl_missao_tela(self.world, &mut screen) };
        if code != RL_OK {
            return self.fail_value("tela da campanha", code);
        }
        Ok(screen)
    }

    pub fn campaign_end(&mut self) -> Result<CampaignEnd, String> {
        let bytes = self.world_bytes()?;
        let end = campaign_byte(&bytes, 0x004e_05a0)?;
        let victory = campaign_byte(&bytes, 0x004e_1d84)?;
        let stage = campaign_byte(&bytes, 0x0049_6fe4)?;
        let mut result = CampaignEnd {
            outcome: if end == 0 {
                0
            } else if victory != 0 {
                1
            } else {
                2
            },
            stage,
            has_next: false,
            next: String::new(),
        };
        if result.outcome == 1 {
            let (_, name) = self.next_path()?;
            result.next = name.clone();
            result.has_next = file_if_exists(&self.root, &name).is_some();
            if !result.has_next && stage != 25 {
                self.save_error = format!("Arquivo da próxima fase ausente: {name}");
                return Err(self.save_error.clone());
            }
        }
        Ok(result)
    }

    fn world_bytes(&mut self) -> Result<Vec<u8>, String> {
        let mut required = 0u32;
        // SAFETY: documented size query.
        let code = unsafe { rl_projetar_tudo(self.world, std::ptr::null_mut(), 0, &mut required) };
        if code != RL_BUFFER_TOO_SMALL {
            return self.fail_value("projeção do mundo", code);
        }
        let mut bytes = vec![0u8; required as usize];
        // SAFETY: `bytes` is writable for `required` bytes.
        let code = unsafe {
            rl_projetar_tudo(self.world, bytes.as_mut_ptr(), required, &mut required)
        };
        if code != RL_OK {
            return self.fail_value("projeção do mundo", code);
        }
        Ok(bytes)
    }

    fn next_path(&mut self) -> Result<(Vec<u8>, String), String> {
        let mut path = vec![0u8; 257];
        let mut required = 0u32;
        // SAFETY: the buffer is writable for 257 bytes.
        let code =
            unsafe { rl_missao_nome_proxima(self.world, path.as_mut_ptr(), 256, &mut required) };
        if code != RL_OK || required == 0 || required >= 256 {
            return self.fail_value("Próxima fase", code);
        }
        path.truncate(required as usize);
        let name = path_name(&path)?;
        Ok((path, name))
    }

    pub fn continue_campaign(&mut self, difficulty: u32) -> Result<(), String> {
        self.continue_stage(difficulty, false)
    }

    fn continue_stage(&mut self, difficulty: u32, prepared: bool) -> Result<(), String> {
        let fresh = prepared && self.new_campaign != 0;
        let (path, name) = if fresh {
            self.new_campaign_path()?
        } else {
            let end = self.campaign_end()?;
            if end.outcome != 1 || !end.has_next {
                return Err("Não há vitória com uma próxima fase disponível.".into());
            }
            let mut queued = 0u32;
            // SAFETY: writable local.
            let code = unsafe { rl_entrada_quantidade_pendente(self.world, &mut queued) };
            if code != RL_OK {
                return self.fail_value("Entradas da fase anterior", code);
            }
            if queued != 0 {
                return Err("Há entradas salvas pendentes na fase anterior.".into());
            }
            self.next_path()?
        };
        let eta = file(&self.root, &name)?;
        let monsters = file(&self.root, "MONSTRE.INI")?;
        let mut candidate = self.candidate(&name, difficulty);
        let mut new_world = 0u64;
        let mut new_input = RlFrameInput::default();
        let code = if prepared {
            let carried = RlFrameInput {
                cursor_x: self.shell_input.mouse_x as i32,
                cursor_y: self.shell_input.mouse_y as i32,
                ..self.input
            };
            candidate.cursor = (carried.cursor_x, carried.cursor_y);
            candidate.keyboard.clear();
            let name_span = shell_bytes(&path);
            let eta_span = shell_bytes(&eta);
            let monster_span = shell_bytes(&monsters);
            // SAFETY: all spans outlive the call; outputs are distinct locals.
            unsafe {
                rl_shell_campaign_load_dificuldade(
                    self.shell,
                    self.shell_generation,
                    name_span,
                    eta_span,
                    monster_span,
                    &carried,
                    difficulty,
                    &mut new_world,
                    &mut new_input,
                )
            }
        } else {
            // SAFETY: spans and input outlive the call; outputs are distinct locals.
            let code = unsafe {
                rl_missao_continuar(
                    self.world,
                    path.as_ptr(),
                    path.len() as u32,
                    eta.as_ptr(),
                    eta.len() as u32,
                    monsters.as_ptr(),
                    monsters.len() as u32,
                    &self.input,
                    &mut new_world,
                    &mut new_input,
                )
            };
            if code == RL_OK && difficulty != RL_DIFICULDADE_NORMAL {
                // SAFETY: the fresh stage handle is live.
                let applied = unsafe { rl_aplicar_dificuldade(new_world, difficulty) };
                if applied != RL_OK {
                    // SAFETY: candidate handle is discarded on failure.
                    unsafe { rl_liberar_mundo(new_world) };
                    return self.fail_value("Dificuldade", applied);
                }
            }
            code
        };
        if code != RL_OK {
            if new_world != 0 {
                // SAFETY: candidate world is discarded.
                unsafe { rl_liberar_mundo(new_world) };
            }
            return self.fail_value("Não foi possível continuar a campanha", code);
        }
        if self.menu_world {
            // The menu world carries no play execution mode.
            // SAFETY: fresh stage handle.
            let code = unsafe { rl_execucao_definir(new_world, RL_EXECUCAO_JOGO) };
            if code != RL_OK {
                // SAFETY: candidate world is discarded.
                unsafe { rl_liberar_mundo(new_world) };
                return self.fail_value("execution mode", code);
            }
        }
        candidate.world = new_world;
        candidate.input = new_input;
        candidate.cursor = (new_input.cursor_x, new_input.cursor_y);
        candidate.mission = name.clone();
        candidate.difficulty = difficulty;
        if let Err(error) = candidate.load_events(true) {
            return Err(error);
        }
        let renderer = match candidate.new_renderer(new_world, None) {
            Ok(renderer) => renderer,
            Err(error) => {
                return Err(error);
            }
        };
        candidate.renderer = renderer;
        if let Err(error) = candidate.advance() {
            return Err(error);
        }
        if let Err(error) = candidate.render_current() {
            return Err(error);
        }
        let presentation = match candidate.capture_presentation() {
            Ok(presentation) => presentation,
            Err(error) => return Err(error),
        };
        candidate.stage_presentation = Some(presentation);
        candidate.status.generation = candidate.status.generation.max(0);
        self.absorb(candidate);
        self.save_error.clear();
        Ok(())
    }

    fn candidate(&self, mission: &str, difficulty: u32) -> Driver {
        Driver {
            root: self.root.clone(),
            events_directory: self.events_directory.clone(),
            events_bytes: self.events_bytes.clone(),
            world: 0,
            renderer: 0,
            shell: 0,
            shell_generation: 0,
            shell_mode: 0,
            shell_input: RlShellInput {
                version: 1,
                ..Default::default()
            },
            shell_request: RlShellEvent::default(),
            mission: mission.to_owned(),
            difficulty,
            menu_world: false,
            new_campaign: 0,
            slots: self.slots,
            stage_presentation: None,
            pause_presentation: None,
            pause_palette: [0; PALETTE_BYTES],
            stage_cursor: (400, 300),
            pixels: vec![0; PIXELS],
            palette: [0; PALETTE_BYTES],
            game_background: vec![0; PIXELS],
            game_background_ready: false,
            input: self.input,
            phase2: self.phase2,
            keyboard: self.keyboard.clone(),
            mouse: MouseQueue::default(),
            cursor: self.cursor,
            frame_messages: Vec::with_capacity(4096),
            frame_keys: Vec::with_capacity(256),
            input_next_phase: 2,
            status: Status {
                phase_index: u32::MAX,
                ..Default::default()
            },
            save_error: String::new(),
            sound: Vec::new(),
            declared_frames: 0,
            shell_held_keys: BTreeSet::new(),
            shell_mouse_down: false,
            shell_pending_move: None,
        }
    }

    fn absorb(&mut self, mut candidate: Driver) {
        // SAFETY: both sets of handles are owned; the old ones are freed once.
        unsafe {
            if self.shell != 0 {
                rl_shell_free(self.shell);
            }
            if self.renderer != 0 {
                rl_render_liberar(self.renderer);
            }
            if self.world != 0 {
                rl_liberar_mundo(self.world);
            }
        }
        self.world = std::mem::take(&mut candidate.world);
        self.renderer = std::mem::take(&mut candidate.renderer);
        self.shell = 0;
        self.shell_mode = 0;
        self.shell_request = RlShellEvent::default();
        self.shell_generation = 0;
        self.mission = std::mem::take(&mut candidate.mission);
        self.difficulty = candidate.difficulty;
        self.menu_world = false;
        self.new_campaign = 0;
        self.input = candidate.input;
        self.phase2 = candidate.phase2;
        self.input_next_phase = 2;
        self.cursor = candidate.cursor;
        self.pixels = std::mem::take(&mut candidate.pixels);
        self.palette = candidate.palette;
        self.game_background = std::mem::take(&mut candidate.game_background);
        self.game_background_ready = candidate.game_background_ready;
        self.stage_presentation = candidate.stage_presentation.take();
        self.pause_presentation = None;
        self.status = candidate.status.clone();
        self.keyboard = candidate.keyboard.clone();
        self.mouse = MouseQueue::default();
        self.declared_frames = 0;
    }

    fn new_campaign_path(&mut self) -> Result<(Vec<u8>, String), String> {
        let number = self.new_campaign_stage();
        if !(1..=25).contains(&number) {
            return Err("Nova campanha inválida".into());
        }
        let bytes = self.world_bytes()?;
        let prefix = project_string(&bytes, 0x0049_4b27, 31)?;
        let suffix = project_string(&bytes, 0x0049_4b36, 15)?;
        let root = project_string(&bytes, 0x0050_9594, 200)?;
        if prefix.len() != 14 {
            return Err("Caminho da nova campanha inválido".into());
        }
        let mut prefix = prefix;
        prefix[12] = b'0' + (number / 10) as u8;
        prefix[13] = b'0' + (number % 10) as u8;
        let mut path = root;
        path.extend_from_slice(&prefix);
        path.extend_from_slice(&suffix);
        let name = path_name(&path)?;
        Ok((path, name))
    }

    fn new_campaign_stage(&self) -> u32 {
        self.new_campaign
    }

    // ----- prepared shell -----------------------------------------------------

    pub fn shell_mode(&self) -> u32 {
        self.shell_mode
    }

    pub fn shell_state(&self) -> Option<RlShellInput> {
        if self.shell == 0 {
            return None;
        }
        let mut input = self.shell_input;
        input.keys = 0;
        input.mouse_b0 = 0;
        Some(input)
    }

    pub fn shell_begin(&mut self) -> Result<(), String> {
        if self.shell != 0 || self.status.stopped {
            return Err("shell already open".into());
        }
        let screen = self.campaign_screen()?;
        if screen != RL_SHELL_RESULT && screen != RL_SHELL_PAUSE {
            return Err("shell screen unavailable".into());
        }
        if screen == RL_SHELL_PAUSE {
            let presentation = self.capture_presentation()?;
            self.pause_presentation = Some(presentation);
            self.pause_palette = self.palette;
        }
        self.shell_open(screen).map_err(|error| {
            self.save_error = error.clone();
            error
        })
    }

    fn shell_assets(&mut self) -> Result<Vec<Vec<u8>>, String> {
        let mut owned = Vec::with_capacity(11);
        for index in 0..9 {
            if index == 7 {
                let mut shadow = vec![0u8; 256];
                let mut light = vec![0u8; 256];
                // SAFETY: both buffers are writable for 256 bytes.
                let code = unsafe {
                    rl_mundo_tabelas_sombra_luz(
                        self.world,
                        shadow.as_mut_ptr(),
                        light.as_mut_ptr(),
                    )
                };
                if code != RL_OK {
                    return self.fail_value("tabelas de sombra e luz", code);
                }
                owned.push(shadow);
                owned.push(light);
                continue;
            }
            let name = if index < 7 {
                format!("asset{index}.bin")
            } else {
                String::new()
            };
            if !name.is_empty() {
                owned.push(file(&self.root, &name)?);
            }
        }
        owned.push(file(&self.root, "MENU.CMP")?);
        owned.push(file(&self.root, "INVENTE.CMP")?);
        Ok(owned)
    }

    fn shell_open(&mut self, mode: u32) -> Result<(), String> {
        let assets = self.shell_assets()?;
        let descriptors: Vec<RlShellBytes> = assets
            .iter()
            .map(|bytes| shell_bytes(bytes))
            .collect();
        let surface = shell_bytes(&self.pixels);
        let mut shell = 0u64;
        // SAFETY: all 11 descriptors and the surface outlive the call.
        let code = unsafe {
            rl_shell_create(
                SHELL_VERSION,
                self.world,
                descriptors.as_ptr(),
                descriptors.len() as u32,
                surface,
                &mut shell,
            )
        };
        if code != RL_OK {
            return self.fail_value("Menu original indisponível", code);
        }
        let result = (|| {
            let bytes = self.world_bytes()?;
            let mut input = RlShellInput {
                version: 1,
                ..Default::default()
            };
            input.clock_7c = project_word(&bytes, 0x0050_8f7c, 4)?;
            input.animation_84 = project_word(&bytes, 0x0050_8f84, 4)?;
            input.mouse_x = project_word(&bytes, 0x0050_99a8, 2)?;
            input.mouse_y = project_word(&bytes, 0x0050_99aa, 2)?;
            self.shell = shell;
            self.shell_mode = mode;
            self.shell_generation = 0;
            self.shell_input = input;
            self.shell_request = RlShellEvent::default();
            self.shell_paint()
        })();
        if result.is_err() {
            // SAFETY: the session is discarded.
            unsafe { rl_shell_free(shell) };
            self.shell = 0;
            self.shell_mode = 0;
        }
        result
    }

    fn shell_paint(&mut self) -> Result<(), String> {
        let mut plan = RlShellPlan::default();
        // SAFETY: writable local.
        let code = unsafe { rl_shell_plan(self.shell, self.shell_mode, &mut plan) };
        if code != RL_OK {
            return self.fail_value("Plano da tela original", code);
        }
        let background = null_terminated(&plan.background).to_ascii_lowercase();
        let name = match background.as_str() {
            "\\ecranfin.lzs" => "ECRANFIN.LZS",
            "\\pagemis2.pie" => "PAGEMIS2.PIE",
            "\\pagemis.pie" => "PAGEMIS.PIE",
            "\\titre.pie" => "TITRE.PIE",
            "\\credits.pie" => "CREDITS.PIE",
            _ => return Err("Fundo original ausente ou inválido".into()),
        };
        let file_bytes = file(&self.root, name)?;
        let mut pixels = vec![0u8; PIXELS];
        // SAFETY: the file span is readable and the surface is writable.
        let code = unsafe {
            rl_shell_imagem_cd(
                file_bytes.as_ptr(),
                file_bytes.len() as u32,
                pixels.as_mut_ptr(),
                pixels.len() as u32,
            )
        };
        if code != RL_OK {
            return self.fail_value("Fundo original ausente ou inválido", code);
        }
        // SAFETY: the surface outlives the call; the generation is a writable local.
        let code = unsafe {
            rl_shell_paint_prepared(
                self.shell,
                self.shell_generation,
                shell_bytes(&pixels),
                &mut self.shell_generation,
            )
        };
        if code != RL_OK {
            return self.fail_value("Pintura da tela original", code);
        }
        self.shell_copy_picture()
    }

    fn shell_copy_picture(&mut self) -> Result<(), String> {
        let mut pixels = vec![0u8; PIXELS];
        let mut palette = [0u8; PALETTE_BYTES];
        // SAFETY: both outputs are writable for their fixed capacities.
        let code = unsafe {
            rl_shell_copy(
                self.shell,
                self.shell_generation,
                pixels.as_mut_ptr(),
                pixels.len() as u32,
                palette.as_mut_ptr(),
                palette.len() as u32,
            )
        };
        if code != RL_OK {
            return self.fail_value("Cópia da tela original", code);
        }
        self.pixels = pixels;
        self.palette = palette;
        Ok(())
    }

    pub fn shell_input(&mut self, input: RlShellInput) -> Result<RlShellEvent, String> {
        if self.shell == 0 || self.shell_request.kind != RL_SHELL_NONE {
            return Err("shell request pending".into());
        }
        let mut event = RlShellEvent::default();
        // SAFETY: input and event are valid for this call.
        let code = unsafe {
            rl_shell_input(self.shell, self.shell_generation, &input, &mut event)
        };
        if code != RL_OK {
            return self.fail_value("Entrada na tela original", code);
        }
        self.shell_generation = event.generation;
        self.shell_input = input;
        self.shell_request = event;
        Ok(event)
    }

    pub fn shell_draw(&mut self, animation: u32) -> Result<(), String> {
        if self.shell == 0 || self.shell_request.kind != RL_SHELL_NONE {
            return Err("shell request pending".into());
        }
        // SAFETY: the generation output is a writable local.
        let code = unsafe {
            rl_shell_draw(
                self.shell,
                self.shell_generation,
                animation,
                &mut self.shell_generation,
            )
        };
        if code != RL_OK {
            return self.fail_value("Desenho da tela original", code);
        }
        self.shell_input.animation_84 = animation;
        self.shell_copy_picture()
    }

    pub fn shell_slots(&mut self, table: &[u8; 330]) -> Result<(), String> {
        if self.shell == 0 {
            return Err("shell not open".into());
        }
        for index in 0..10 {
            if table[33 * index] > 1 {
                return Err("invalid slot table".into());
            }
        }
        self.slots = [[0; 33]; 10];
        for index in 0..10 {
            self.slots[index].copy_from_slice(&table[33 * index..33 * index + 33]);
        }
        Ok(())
    }

    pub fn shell_show(&mut self) -> Result<(), String> {
        let kind = self.shell_request.kind;
        if kind != RL_SHELL_SHOW && kind != RL_SHELL_SAVE_LOAD && kind != RL_SHELL_CREDITS {
            return Err("no SHOW request".into());
        }
        let mode = if kind == RL_SHELL_CREDITS {
            RL_SHELL_CREDITS_SCREEN
        } else {
            self.shell_request.mode
        };
        let from = self.shell_mode;
        let mut briefing_stage = 0u32;
        let mut entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        if is_list(mode) {
            entries = self.slot_files()?;
        } else if mode == RL_SHELL_BRIEFING || mode == RL_SHELL_OBJECTIVES {
            briefing_stage = self.current_stage()?;
            let (path, text) = self.briefing_file(briefing_stage)?;
            if let Some(text) = text {
                entries.push((path, text));
            }
        } else if mode == RL_SHELL_CREDITS_SCREEN {
            let (path, header) = self.credits_file()?;
            if !header.is_empty() {
                entries.push((path, header));
            }
        }
        let files: Vec<RlShellFile> = entries
            .iter()
            .map(|(path, bytes)| RlShellFile {
                path: shell_bytes(path),
                bytes: shell_bytes(bytes),
            })
            .collect();
        // SAFETY: every descriptor references buffers retained for this scope.
        let code = unsafe {
            rl_shell_menu_init(
                self.shell,
                self.shell_generation,
                files.as_ptr(),
                files.len() as u32,
                &mut self.shell_generation,
            )
        };
        if code != RL_OK {
            return self.fail_value("Tela original", code);
        }
        // Pause "Reiniciar a missão": the briefing's START_MISSION reloads this stage.
        if mode == RL_SHELL_BRIEFING && from == RL_SHELL_PAUSE {
            self.new_campaign = briefing_stage;
        }
        self.shell_entered(mode)
    }

    pub fn shell_request(&mut self) -> Result<(), String> {
        let kind = self.shell_request.kind;
        let mode = self.shell_request.mode;
        if kind == RL_SHELL_RESUME || kind == RL_SHELL_SAVE_SLOT {
            return self.shell_resume();
        }
        if kind == RL_SHELL_RESET_CAMPAIGN && mode == RL_SHELL_BRIEFING {
            let (path, text) = self.briefing_file(1)?;
            let mut files = Vec::new();
            let mut owned = Vec::new();
            if let Some(text) = text {
                owned.push(text);
            }
            if let Some(text) = owned.last() {
                files.push(RlShellFile {
                    path: shell_bytes(&path),
                    bytes: shell_bytes(text),
                });
            }
            // SAFETY: the descriptors reference retained buffers.
            let code = unsafe {
                rl_shell_campaign_reset(
                    self.shell,
                    self.shell_generation,
                    files.as_ptr(),
                    files.len() as u32,
                    &mut self.shell_generation,
                )
            };
            if code != RL_OK {
                return self.fail_value("Novo jogo original", code);
            }
            self.new_campaign = 1;
            return self.shell_entered(RL_SHELL_BRIEFING);
        }
        if kind == RL_SHELL_RESET_CAMPAIGN && mode == RL_SHELL_LOAD_MENU {
            let entries = self.slot_files()?;
            let files: Vec<RlShellFile> = entries
                .iter()
                .map(|(path, bytes)| RlShellFile {
                    path: shell_bytes(path),
                    bytes: shell_bytes(bytes),
                })
                .collect();
            // SAFETY: descriptors reference retained buffers.
            let code = unsafe {
                rl_shell_campaign_reset(
                    self.shell,
                    self.shell_generation,
                    files.as_ptr(),
                    files.len() as u32,
                    &mut self.shell_generation,
                )
            };
            if code != RL_OK {
                return self.fail_value("Carregar jogo original", code);
            }
            return self.shell_entered(RL_SHELL_LOAD_MENU);
        }
        self.save_error = format!("Pedido original {kind}/{mode} ainda não disponível");
        Err(RL_UNSUPPORTED_PATH.to_string())
    }

    pub fn shell_prepare(&mut self) -> Result<(), String> {
        if self.shell == 0 || self.shell_request.kind != RL_SHELL_MISSION_NEXT {
            return Err("no MISSION_NEXT request".into());
        }
        let (path, name) = self.next_path()?;
        let eta = file_if_exists(&self.root, &name);
        let text_name = format!(
            "MONDE{}{}.TXT",
            name.as_bytes().get(5).copied().unwrap_or(b'0') as char,
            name.as_bytes().get(6).copied().unwrap_or(b'0') as char
        );
        let text = file_if_exists(&self.root, &text_name);
        let stage = self.current_stage()?;
        let final_stage = eta.is_none() && stage == 25;
        if !final_stage && eta.is_none() {
            self.save_error = format!("Falha ao ler dados originais do briefing: {name} / {text_name}");
            return Err(self.save_error.clone());
        }
        let bytes = self.world_bytes()?;
        let root = project_string(&bytes, 0x0050_9594, 256)?;
        let relative = project_string(&bytes, 0x0049_8430, 19)?;
        if relative.len() != 18
            || &relative[..12] != b"\\World\\monde"
            || &relative[14..] != b".TXT"
        {
            return Err("Caminho do briefing inválido".into());
        }
        let mut text_path = root;
        text_path.extend_from_slice(&relative);
        let eta_bytes = eta.unwrap_or_default();
        let text_bytes = text.unwrap_or_default();
        let mut files = Vec::new();
        let mut paths = vec![path.clone(), text_path.clone()];
        let mut owned = vec![eta_bytes.clone(), text_bytes.clone()];
        if final_stage {
            files.push(RlShellFile {
                path: shell_bytes(&paths[1]),
                bytes: shell_bytes(&owned[1]),
            });
        } else {
            files.push(RlShellFile {
                path: shell_bytes(&paths[0]),
                bytes: shell_bytes(&owned[0]),
            });
            if !text_bytes.is_empty() {
                files.push(RlShellFile {
                    path: shell_bytes(&paths[1]),
                    bytes: shell_bytes(&owned[1]),
                });
            }
        }
        // SAFETY: all spans are retained for this call.
        let code = unsafe {
            rl_shell_campaign_prepare(
                self.shell,
                self.shell_generation,
                shell_bytes(&path),
                files.as_ptr(),
                files.len() as u32,
                &mut self.shell_generation,
            )
        };
        if code != RL_OK {
            return self.fail_value("Preparação do briefing original", code);
        }
        self.shell_mode = if final_stage { RL_SHELL_FINAL } else { 17 };
        self.shell_request = RlShellEvent::default();
        self.shell_paint()
    }

    pub fn shell_load(&mut self) -> Result<(), String> {
        if self.shell == 0 || self.shell_request.kind != RL_SHELL_START_MISSION {
            return Err("no START_MISSION request".into());
        }
        self.continue_stage(self.difficulty, true)
    }

    pub fn shell_movie_finish(&mut self, completion: u32) -> Result<(), String> {
        let kind = self.shell_request.kind;
        if kind != RL_SHELL_DEFEAT_MOVIE
            && kind != RL_SHELL_CAMPAIGN_MOVIE
            && kind != RL_SHELL_INTRODUCTION
        {
            return Err("no movie request".into());
        }
        // SAFETY: the generation output is a writable local.
        let code = unsafe {
            rl_shell_defeat_finish(
                self.shell,
                self.shell_generation,
                completion,
                &mut self.shell_generation,
            )
        };
        if code != RL_OK {
            return self.fail_value("Retorno original após o filme", code);
        }
        self.shell_mode = RL_SHELL_MAIN;
        self.shell_request = RlShellEvent::default();
        self.shell_input.keys = 0;
        self.shell_input.mouse_ae = 0;
        self.shell_input.mouse_af = 0;
        self.shell_input.mouse_b0 = 0;
        self.shell_paint()
    }

    pub fn shell_credits_frame(&mut self, elapsed_ms: u32) -> Result<RlShellCredits, String> {
        if self.shell == 0 || self.shell_mode != RL_SHELL_CREDITS_SCREEN {
            return Err("credits screen not open".into());
        }
        let mut credits = RlShellCredits::default();
        // SAFETY: writable local.
        let code = unsafe {
            rl_shell_credits_frame(
                self.shell,
                self.shell_generation,
                elapsed_ms,
                &mut credits,
            )
        };
        if code != RL_OK {
            return self.fail_value("Créditos originais", code);
        }
        self.shell_generation = credits.generation;
        if credits.state == RL_SHELL_CREDITS_ENDED {
            // SAFETY: documented completion; the generation is a writable local.
            let code = unsafe {
                rl_shell_menu_init(
                    self.shell,
                    self.shell_generation,
                    std::ptr::null(),
                    0,
                    &mut self.shell_generation,
                )
            };
            if code != RL_OK {
                return self.fail_value("Retorno original dos créditos", code);
            }
            self.shell_entered(RL_SHELL_MAIN)?;
        }
        Ok(credits)
    }

    pub fn shell_credits_present(
        &mut self,
        frame: &RlShellCredits,
        pixels: &[u8],
        palette: &[u8],
    ) -> Result<(), String> {
        if frame.state != RL_SHELL_CREDITS_DRAW
            || frame.x + frame.width > 800
            || frame.y + frame.height > 600
            || palette.len() < PALETTE_BYTES
        {
            return Err("invalid credits frame".into());
        }
        for row in 0..frame.height as usize {
            let source = row * frame.width as usize;
            let destination =
                (frame.y as usize + row) * 800 + frame.x as usize;
            self.pixels[destination..destination + frame.width as usize]
                .copy_from_slice(&pixels[source..source + frame.width as usize]);
        }
        self.palette.copy_from_slice(&palette[..PALETTE_BYTES]);
        Ok(())
    }

    pub fn shell_save_slot(&mut self, path: &Path) -> (i32, Result<(), String>) {
        let shown = (self.pixels.clone(), self.palette);
        if let Some(pause) = self.pause_presentation.clone() {
            self.pixels.copy_from_slice(&pause[..PIXELS]);
            self.palette = self.pause_palette;
        }
        let saved = match self.save_slot(path, SaveKind::Slot) {
            Ok(()) => 0,
            Err(code) => code,
        };
        let error = self.save_error.clone();
        match self.shell_resume() {
            Ok(()) => {
                if saved != 0 {
                    self.save_error = error;
                }
                (saved, Ok(()))
            }
            Err(failure) => {
                self.pixels.copy_from_slice(&shown.0);
                self.palette = shown.1;
                self.save_error = failure.clone();
                (saved, Err(failure))
            }
        }
    }

    pub fn shell_click_clock(mode: u32, clock: u32, press: bool) -> u32 {
        if mode != RL_SHELL_BRIEFING && mode != RL_SHELL_OBJECTIVES {
            return clock;
        }
        if press {
            if clock % 10 != 0 {
                clock + (10 - clock % 10)
            } else {
                clock
            }
        } else if clock % 10 != 0 {
            clock
        } else {
            clock + 1
        }
    }

    fn shell_resume(&mut self) -> Result<(), String> {
        let pause = self
            .pause_presentation
            .clone()
            .ok_or_else(|| "Retorno do menu de pausa sem a apresentação do jogo".to_string())?;
        let mut world = 0u64;
        let mut input = RlFrameInput::default();
        // SAFETY: the carried input and outputs are valid for this call.
        let code = unsafe {
            rl_shell_resume(
                self.shell,
                self.shell_generation,
                &self.input,
                &mut world,
                &mut input,
            )
        };
        if code != RL_OK {
            return self.fail_value("Retorno do menu de pausa", code);
        }
        if !self.events_bytes.is_empty() {
            let host = self.events_host();
            // SAFETY: descriptor copied; candidate world is fresh.
            unsafe { rl_eventos_reassociar_v1(world, &host) };
        }
        if let Err(error) = self.declare_flc_events(world) {
            self.save_error = error;
            // SAFETY: candidate world discarded.
            unsafe { rl_liberar_mundo(world) };
            return Err(self.save_error.clone());
        }
        let renderer = match self.new_renderer(world, Some(&pause)) {
            Ok(renderer) => renderer,
            Err(error) => {
                // SAFETY: candidate world discarded.
                unsafe { rl_liberar_mundo(world) };
                return Err(error);
            }
        };
        // SAFETY: the session and previous handles are owned and replaced once.
        unsafe {
            rl_shell_free(self.shell);
            if self.renderer != 0 {
                rl_render_liberar(self.renderer);
            }
            if self.world != 0 {
                rl_liberar_mundo(self.world);
            }
        }
        self.shell = 0;
        self.shell_mode = 0;
        self.shell_request = RlShellEvent::default();
        self.world = world;
        self.renderer = renderer;
        self.input = input;
        self.pixels.copy_from_slice(&pause[..PIXELS]);
        self.palette = self.pause_palette;
        self.pause_presentation = None;
        self.keyboard.clear();
        self.mouse.clear();
        Ok(())
    }

    fn shell_entered(&mut self, mode: u32) -> Result<(), String> {
        self.shell_mode = mode;
        self.shell_request = RlShellEvent::default();
        self.shell_input.keys = 0;
        self.shell_input.mouse_ae = 0;
        self.shell_input.mouse_af = 0;
        self.shell_input.mouse_b0 = 0;
        self.shell_paint()
    }

    fn shell_key(&mut self, vk: u32, down: bool) -> Result<(), String> {
        if self.shell_request.kind != RL_SHELL_NONE {
            return Ok(());
        }
        if down {
            self.shell_held_keys.insert(vk);
            return Ok(());
        }
        if !self.shell_held_keys.remove(&vk) {
            return Ok(());
        }
        let keys = match vk {
            VK_RETURN => RL_SHELL_ENTER,
            38 => RL_SHELL_UP,
            40 => RL_SHELL_DOWN,
            27 => RL_SHELL_ESCAPE,
            _ => 0,
        };
        if keys != 0 {
            self.shell_send_keys(keys)?;
        }
        Ok(())
    }

    fn shell_mouse(&mut self, message: u32, x: i32, y: i32) -> Result<(), String> {
        if self.shell_request.kind != RL_SHELL_NONE {
            return Ok(());
        }
        let mut input = self
            .shell_state()
            .ok_or_else(|| "shell not open".to_string())?;
        input.mouse_x = x.max(0) as u32;
        input.mouse_y = y.max(0) as u32;
        input.mouse_af = 0;
        input.mouse_b0 = 0;
        let press = message == 0x201 || message == 0x203;
        if press {
            self.shell_mouse_down = true;
        }
        input.mouse_ae = u32::from(self.shell_mouse_down);
        if message == 0x202 {
            input.mouse_af = u32::from(self.shell_mouse_down);
            self.shell_mouse_down = false;
        }
        if press || input.mouse_ae != 0 {
            input.clock_7c = Self::shell_click_clock(
                self.shell_mode,
                input.clock_7c,
                press,
            );
        }
        let _ = self.shell_input(input)?;
        Ok(())
    }

    fn shell_send_keys(&mut self, keys: u32) -> Result<(), String> {
        let mut input = self
            .shell_state()
            .ok_or_else(|| "shell not open".to_string())?;
        input.keys = keys;
        input.mouse_ae = 0;
        input.mouse_af = 0;
        let _ = self.shell_input(input)?;
        Ok(())
    }

    /// Present host saves in the original slot VFS order: the exact paths
    /// stored behind 0x497128, each with a `SAV3` header and its caption.
    fn slot_files(&mut self) -> Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
        let bytes = self.world_bytes()?;
        let mut entries = Vec::new();
        for index in 0..10 {
            if self.slots[index][0] == 0 {
                continue;
            }
            let pointer = project_word(&bytes, 0x0049_7128 + 4 * index as u32, 4)?;
            let mut path = Vec::new();
            for offset in 0..255 {
                let byte = campaign_byte(&bytes, pointer + offset)?;
                if byte == 0 {
                    break;
                }
                path.push(byte);
            }
            let mut header = vec![0u8; 36];
            header[..4].copy_from_slice(b"SAV3");
            header[4..].copy_from_slice(&self.slots[index][1..]);
            entries.push((path, header));
        }
        Ok(entries)
    }

    fn current_stage(&mut self) -> Result<u32, String> {
        let bytes = self.world_bytes()?;
        let stage = campaign_byte(&bytes, 0x0049_6fe4)? as u32;
        if !(1..=25).contains(&stage) {
            return Err("invalid stage number".into());
        }
        Ok(stage)
    }

    fn briefing_file(&mut self, number: u32) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
        let bytes = self.world_bytes()?;
        let mut path = project_string(&bytes, 0x0050_9594, 256)?;
        let relative = project_string(&bytes, 0x0049_8430, 19)?;
        if relative.len() != 18
            || &relative[..12] != b"\\World\\monde"
            || &relative[14..] != b".TXT"
        {
            return Err("Caminho do briefing inválido".into());
        }
        let mut relative = relative;
        relative[12] = b'0' + (number / 10) as u8;
        relative[13] = b'0' + (number % 10) as u8;
        path.extend_from_slice(&relative);
        let name = format!("MONDE{number:02}.TXT");
        let text = file_if_exists(&self.root, &name);
        if text.is_none() {
            // An empty briefing is valid; a missing file is not an error.
            return Ok((path, None));
        }
        Ok((path, text))
    }

    fn credits_file(&mut self) -> Result<(Vec<u8>, Vec<u8>), String> {
        let bytes = self.world_bytes()?;
        let mut path = project_string(&bytes, 0x0050_9594, 256)?;
        path.extend_from_slice(b"\\Events\\CREDITS.FLC");
        let events_folder = self.root.parent().unwrap_or(&self.root).join("events");
        let header = file_if_exists(&events_folder, "CREDITS.FLC")
            .map(|bytes| bytes[..bytes.len().min(128)].to_vec())
            .unwrap_or_default();
        Ok((path, header))
    }

    pub fn shell_mouse_down(&self) -> bool {
        self.shell_mouse_down
    }

    pub fn shell_set_mouse_down(&mut self, down: bool) {
        self.shell_mouse_down = down;
    }

    pub fn frame_event_flc_state(&mut self) -> EventFlcState {
        let mut state = RlEventFlcStateV1::default();
        if self.shell != 0 {
            return EventFlcState::default();
        }
        // SAFETY: writable local.
        let code = unsafe { rl_evento_flc_estado_v1(self.world, &mut state) };
        if code == RL_OK {
            state.into()
        } else {
            EventFlcState::default()
        }
    }

    pub fn finish_flc_event(&mut self) -> i32 {
        // SAFETY: the world is live; the call is idempotent under its contract.
        unsafe { rl_evento_flc_terminar_v1(self.world) }
    }

    pub fn declared_frames(&self) -> u64 {
        self.declared_frames
    }
}

fn is_list(mode: u32) -> bool {
    mode == RL_SHELL_LOAD_LIST || mode == RL_SHELL_LOAD_MENU || mode == RL_SHELL_SAVE_LIST
}

fn parse_stage(name: &str) -> Option<u32> {
    if name.len() < 7 || !name.starts_with("MONDE") {
        return None;
    }
    name[5..7].parse().ok()
}

fn path_name(path: &[u8]) -> Result<String, String> {
    let text = String::from_utf8_lossy(path);
    let base = text
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    if base.len() < 11 || base.len() > 12 || !base.starts_with("MONDE") || !base.ends_with(".ETA")
    {
        return Err("Caminho da próxima fase inválido".into());
    }
    Ok(base)
}

fn campaign_byte(bytes: &[u8], address: u32) -> Result<u8, String> {
    if bytes.len() < 4 {
        return Err("projeção inválida".into());
    }
    let mut offset = 4usize;
    let count = word(bytes);
    for _ in 0..count {
        if offset + 8 > bytes.len() {
            return Err("projeção inválida".into());
        }
        let base = word(&bytes[offset..]);
        let length = word(&bytes[offset + 4..]);
        offset += 8;
        if offset + length as usize > bytes.len() {
            return Err("projeção inválida".into());
        }
        if address >= base && (address as u64) < base as u64 + length as u64 {
            return Ok(bytes[offset + (address - base) as usize]);
        }
        offset += length as usize;
    }
    Err("endereço fora da projeção".into())
}

fn project_word(bytes: &[u8], address: u32, size: u32) -> Result<u32, String> {
    let mut value = 0u32;
    for index in 0..size {
        value |= (campaign_byte(bytes, address + index)? as u32) << (8 * index);
    }
    Ok(value)
}

fn project_string(bytes: &[u8], address: u32, capacity: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for index in 0..capacity {
        let byte = campaign_byte(bytes, address + index)?;
        if byte == 0 {
            return Ok(out);
        }
        out.push(byte);
    }
    Err("string fora da projeção".into())
}

// `shell_held_keys` and `shell_mouse_down` are runtime host state, kept here so
// the shell input path stays serialized with the game worker in one object.
impl Driver {
    pub fn shell_tick(&mut self) -> Result<(), String> {
        if self.shell == 0 || self.shell_request.kind != RL_SHELL_NONE {
            return Ok(());
        }
        let mut input = self
            .shell_state()
            .ok_or_else(|| "shell not open".to_string())?;
        input.clock_7c = input.clock_7c.wrapping_add(1);
        input.animation_84 = input.animation_84.wrapping_add(1);
        input.mouse_ae = u32::from(self.shell_mouse_down);
        input.mouse_af = 0;
        if let Some((x, y)) = self.shell_pending_move.take() {
            input.mouse_x = x;
            input.mouse_y = y;
        }
        let event = self.shell_input(input)?;
        if event.kind == RL_SHELL_NONE && self.shell_mode != RL_SHELL_CREDITS_SCREEN {
            self.shell_draw(input.animation_84)?;
        }
        Ok(())
    }

    pub fn shell_pending_move(&mut self) -> Option<(u32, u32)> {
        self.shell_pending_move.take()
    }

    pub fn shell_set_move(&mut self, x: u32, y: u32) {
        self.shell_pending_move = Some((x, y));
    }
}
