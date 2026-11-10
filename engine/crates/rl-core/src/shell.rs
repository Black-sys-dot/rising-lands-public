//! Engine-side P4 screen state/input/surface composition. App wiring is separate.
//! enter_prepared requires the caller to finish the requested campaign/research
//! operations and provide the real decoded background. It never simulates them.
use crate::{
    render::{Error, Renderer, Result},
    shell_animation::{self, Bank},
    shell_briefing::{self, Files},
    shell_draw,
    shell_input::{self, Keys},
    shell_menu, shell_state, FrameRegisters, MemoriaJogo,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Screen {
    Main = 2,
    SinglePlayer = 3,
    /// Original save-slot lists: load in game12, load from the main menu13, save14.
    LoadList = 12,
    LoadMenu = 13,
    SaveList = 14,
    Pause = 16,
    Briefing = 17,
    Result = 19,
    Objectives = 23,
    /// MenuInit(10) credits: credits.pie and the CREDITS.FLC animation that
    /// AffCredits draws (campaign_flow::credits_frame), left by Escape or a click.
    Credits = 10,
    /// Campaign end after the last stage: MenuInit(24), then the original movie
    /// request and MenuInit(2) at the same site as the defeat branch.
    Final = 24,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequiredCall {
    pub symbol: &'static str,
    pub address: u32,
}
impl Screen {
    /// Required gameplay preparation, in original call order. Presentation has
    /// no RNG draws, so preparing these before its atomic publication preserves
    /// the gameplay RNG sequence. This API does not execute these dependencies.
    pub fn required_game_calls(self) -> &'static [RequiredCall] {
        match self {
            Self::Main => &[
                RequiredCall {
                    symbol: "mpClose",
                    address: 0x46ce60,
                },
                RequiredCall {
                    symbol: "RechercheReset",
                    address: 0x470b46,
                },
                RequiredCall {
                    symbol: "Engin_ResetExperience",
                    address: 0x441470,
                },
                RequiredCall {
                    symbol: "InitPlayer",
                    address: 0x46dcb4,
                },
            ],
            Self::Briefing => &[RequiredCall {
                symbol: "RechercheCalculNew",
                address: 0x47150f,
            }],
            _ => &[],
        }
    }
    pub fn current(m: &MemoriaJogo) -> Result<Self> {
        match m.u8(shell_state::MODE)? {
            2 => Ok(Self::Main),
            3 => Ok(Self::SinglePlayer),
            12 => Ok(Self::LoadList),
            13 => Ok(Self::LoadMenu),
            14 => Ok(Self::SaveList),
            16 => Ok(Self::Pause),
            17 => Ok(Self::Briefing),
            19 => Ok(Self::Result),
            23 => Ok(Self::Objectives),
            24 => Ok(Self::Final),
            10 => Ok(Self::Credits),
            // CalculGame mode 20, the Space wait: no MenuInit call or store
            // enters it (docs/HANDOVER_20261109_CAMPANHA.md, "CalculGame mode 20").
            20 => Err(Error::Stop(
                0x42cd5f,
                "CalculGame mode 20 is unreachable in the intact binary, see handover".into(),
            )),
            _ => Err(Error::Stop(
                0x4682bb,
                "shell screen is outside prepared UI modes".into(),
            )),
        }
    }
    pub fn background(self, mission: u8) -> &'static [u8] {
        match self {
            Self::Main
            | Self::SinglePlayer
            | Self::LoadList
            | Self::LoadMenu
            | Self::SaveList
            | Self::Pause
            | Self::Final => b"\\titre.pie",
            Self::Briefing | Self::Objectives => {
                if mission < 7 {
                    b"\\pagemis2.pie"
                } else {
                    b"\\pagemis.pie"
                }
            }
            Self::Result => b"\\ecranfin.lzs",
            Self::Credits => b"\\credits.pie",
        }
    }
}
pub struct Banks {
    pub menu: Bank,
    pub inventions: Bank,
}
impl Banks {
    pub fn load(menu: &[u8], inventions: &[u8], renderer: &Renderer) -> Result<Self> {
        if menu.get(..2) != Some(&40u16.to_le_bytes())
            || inventions.get(..2) != Some(&72u16.to_le_bytes())
        {
            return Err(Error::Invalid("MENU/INVENTE bank order"));
        }
        Ok(Self {
            menu: Bank::load(menu, &renderer.assets)?,
            inventions: Bank::load(inventions, &renderer.assets)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    None,
    Show(Screen),
    /// MissionReset, player initialization, then mode17 or mode13. Mode17 also
    /// requires original RechercheCalculNew0x47150f before enter_prepared.
    ResetCampaign {
        next_mode: u8,
    },
    StartMission,
    MissionNext,
    ResumeGame,
    SaveLoadMenu {
        mode: u8,
    },
    /// MenuInit(1): CalculGame mode1 then probes this CD-root movie; absent or
    /// finished -> MenuInit(2) at the defeat join 0x42c925.
    Introduction {
        path: Vec<u8>,
    },
    Credits,
    Multiplayer,
    Quit,
    /// Original defeat branch checks this CD-root movie; absent or finished -> main.
    DefeatMovie {
        path: Vec<u8>,
    },
    /// A slot was activated in list12 (in game) or13 (main menu): the original
    /// checks the slot file, runs GameLoad and MenuInit(18). `path` is the exact
    /// original slot filename at0x497128; the host loads its own saved game.
    LoadSlot {
        slot: u8,
        from_menu: bool,
        path: Vec<u8>,
    },
    /// A slot was activated in list14: the original GameSave, then MenuInit(18).
    SaveSlot {
        slot: u8,
        path: Vec<u8>,
    },
    /// CalculGame mode24 (0x42c947) checks this CD-root campaign-end movie;
    /// absent or finished -> MenuInit(2), like the defeat branch.
    CampaignMovie {
        path: Vec<u8>,
    },
}
fn request(m: &MemoriaJogo, mode: u8) -> Result<Request> {
    Ok(match mode {
        2 => Request::Show(Screen::Main),
        3 => Request::Show(Screen::SinglePlayer),
        16 => Request::Show(Screen::Pause),
        17 => Request::Show(Screen::Briefing),
        23 => Request::Show(Screen::Objectives),
        18 => Request::ResumeGame,
        12..=14 => Request::SaveLoadMenu { mode },
        1 => {
            // CalculGame mode1 0x42c8bf: CD root 0x509594 + "\\events\\introfr.avi".
            let mut path = shell_briefing::cstring(m, 0x509594, 256)?;
            path.extend_from_slice(&shell_briefing::cstring(m, 0x494444, 256)?);
            Request::Introduction { path }
        }
        10 => Request::Credits,
        4 => Request::Multiplayer,
        21 => Request::Quit,
        _ => {
            return Err(Error::Stop(
                0x4682bb,
                format!("unported shell request mode{mode}"),
            ))
        }
    })
}
fn registers() -> FrameRegisters {
    FrameRegisters {
        esp: 0x2008000,
        ..FrameRegisters::default()
    }
}

/// background is a complete800x600 original picture, decoded with shell_picture.
/// Memory includes original text table, font state and prepared research.
/// Owned CMP banks replace legacy heap headers/pointer tables in this semantic
/// API. Missing assets/fields return addressed errors.
/// Caller prerequisites: required_game_calls(), campaign reset/player setup
/// for a new campaign, and retained game state for mode23. All such operations
/// must finish before this prepared presentation is entered.
/// No world reset, loading, research effect, video, sound or platform effect is
/// replaced by an empty callback. Requests identify those integration boundaries.
pub fn enter_prepared(
    m: &mut MemoriaJogo,
    renderer: &mut Renderer,
    banks: &Banks,
    screen: Screen,
    background: &[u8],
    files: &mut impl Files,
) -> Result<()> {
    if m.u8(shell_state::MODE)? == screen as u8 {
        return Ok(());
    }
    if matches!(
        screen,
        Screen::LoadList | Screen::LoadMenu | Screen::SaveList | Screen::Final | Screen::Credits
    ) {
        // The slot captions come from GameSaveScanNames over host files:
        // campaign_flow::menu_init prepares and enters these lists.
        return Err(Error::Stop(
            0x469292,
            "save-slot lists and credits are entered through MenuInit preparation".into(),
        ));
    }
    if background.len() != 480000 {
        return Err(Error::Stop(
            0x4799c4,
            "shell background is not a complete original surface".into(),
        ));
    }
    if m.u16(0x49a018)? != 800 || m.u16(0x49a01a)? != 600 {
        return Err(Error::Invalid("shell engine surface must be800x600"));
    }
    let mut next = m.clone();
    let mut pixels = renderer.clone();
    if matches!(screen, Screen::Briefing | Screen::Objectives) {
        next.set_u8(0x4e1d84, 0)?;
        let mission = next.u8(0x496fe4)?;
        shell_briefing::load(&mut next, mission, files)?;
    }
    pixels.surface.copy_from_slice(background);
    if matches!(screen, Screen::Briefing | Screen::Objectives) {
        shell_draw::research_block(&mut pixels, &next, next.u8(0x508041)?, registers())?;
    } else if screen == Screen::Result {
        shell_draw::result_caption_block(&mut pixels, &next, registers())?;
    }
    next.set_u8(shell_state::MODE, screen as u8)?;
    let margin = next.u8(0x508f90)?;
    if margin != 0 {
        next.set_u16(0x4e2f7a, margin as i8 as i16 as u16)?;
        next.set_u8(0x508f90, 0)?;
    }
    if matches!(screen, Screen::Main | Screen::SinglePlayer | Screen::Pause) {
        shell_state::configure(&mut next, screen as u8)?;
        shell_state::zones(
            &mut next,
            FrameRegisters {
                eax: 3,
                ..registers()
            },
        )?;
    } else {
        if matches!(screen, Screen::Briefing | Screen::Objectives) {
            shell_state::zones(
                &mut next,
                FrameRegisters {
                    eax: 2,
                    ..registers()
                },
            )?;
        }
        shell_menu::reset(&mut next)?;
    }
    draw_inner(&mut next, &mut pixels, banks)?;
    pixels.last = None;
    *m = next;
    *renderer = pixels;
    Ok(())
}
fn draw_inner(m: &mut MemoriaJogo, renderer: &mut Renderer, banks: &Banks) -> Result<()> {
    match Screen::current(m)? {
        Screen::Main
        | Screen::SinglePlayer
        | Screen::LoadList
        | Screen::LoadMenu
        | Screen::SaveList
        | Screen::Pause => {
            // AffGame0x417f04 sends every list mode below18 to the0x4176d9 body.
            shell_animation::menu_pixels(renderer, m, &banks.menu)?;
        }
        Screen::Briefing | Screen::Objectives => {
            shell_animation::briefing_pixels(renderer, m, &banks.inventions)?;
        }
        // Original AffGame modes19 and24 return without redrawing.
        Screen::Result | Screen::Final => {}
        // AffGame mode10 is AffCredits, driven by its host clock.
        Screen::Credits => {
            return Err(Error::Stop(
                0x43ed0e,
                "credits frames advance through campaign_flow::credits_frame".into(),
            ))
        }
    }
    Ok(())
}
/// Draw over the persistent surface. Counter0x508f84 comes from the caller's
/// virtual frame clock; this routine never advances either RNG or the clock.
pub fn draw(m: &mut MemoriaJogo, renderer: &mut Renderer, banks: &Banks) -> Result<()> {
    let mut next = renderer.clone();
    m.begin();
    match draw_inner(m, &mut next, banks) {
        Ok(()) => {
            m.commit();
            next.last = None;
            *renderer = next;
            Ok(())
        }
        Err(e) => {
            m.rollback();
            Err(e)
        }
    }
}

/// Reconstruct a prepared screen after entry or renderer replacement. All game
/// preparation and the MenuInit flip have already been published by the world.
/// Dirty/clipping presentation scratch stays local; no effect is consumed twice.
pub fn paint_prepared(
    m: &MemoriaJogo,
    renderer: &mut Renderer,
    banks: &Banks,
    background: &[u8],
) -> Result<()> {
    if background.len() != 480000 {
        return Err(Error::Invalid("shell background size"));
    }
    let mut local = m.clone();
    let mut pixels = renderer.clone();
    pixels.surface.copy_from_slice(background);
    match Screen::current(m)? {
        Screen::Briefing | Screen::Objectives => {
            local.set_u8(shell_briefing::DIRTY, 1)?;
            shell_draw::research_block(&mut pixels, &local, local.u8(0x508041)?, registers())?;
        }
        Screen::Result => {
            shell_draw::result_caption_block(&mut pixels, &local, registers())?;
        }
        _ => {}
    }
    // Credits: MenuInit(10) leaves only credits.pie; AffCredits frames overlay it.
    if Screen::current(m)? != Screen::Credits {
        draw_inner(&mut local, &mut pixels, banks)?;
    }
    pixels.last = None;
    *renderer = pixels;
    Ok(())
}

/// Keys are completed press/release events in native Enter/Up/Down/Escape
/// priority. The app supplies raw original mouse coordinates/button flags.
/// Returned requests must complete before the next screen is entered.
pub fn input(m: &mut MemoriaJogo, keys: Keys) -> Result<Request> {
    m.begin();
    let result = input_inner(m, keys);
    match result {
        Ok(r) => {
            m.commit();
            Ok(r)
        }
        Err(e) => {
            m.rollback();
            Err(e)
        }
    }
}
fn input_inner(m: &mut MemoriaJogo, keys: Keys) -> Result<Request> {
    match Screen::current(m)? {
        Screen::Main
        | Screen::SinglePlayer
        | Screen::LoadList
        | Screen::LoadMenu
        | Screen::SaveList
        | Screen::Pause => {
            let b = shell_input::menu(m, registers(), keys)?;
            match b.address {
                0x42d4e2 => Ok(Request::None),
                0x4682bb => request(m, b.registers.edx as u8),
                0x42c3e4 => {
                    let (_, action) = shell_state::selection(m, b.registers)?;
                    let slot_path = |m: &MemoriaJogo, slot: u8| {
                        shell_briefing::cstring(m, m.u32(0x497128 + 4 * u32::from(slot))?, 256)
                    };
                    match action {
                        shell_state::Action::None => Ok(Request::None),
                        shell_state::Action::Menu(mode) => {
                            if mode == 18 {
                                // 0x42c57f: the sixth pause item sets the HUD latch after
                                // MenuInit(18). MenuInit only ever writes 1 there (ResetMode
                                // 0x40f74c), so setting it before resume_game is the same.
                                // Escape (MenuBefore16 0x42c776) reaches mode 18 without it.
                                m.set_u8(0x4e05a2, 1)?;
                            }
                            request(m, mode)
                        }
                        shell_state::Action::ResetCampaign { next_screen } => {
                            Ok(Request::ResetCampaign {
                                next_mode: next_screen,
                            })
                        }
                        shell_state::Action::LoadSlot { slot, from_menu } => {
                            Ok(Request::LoadSlot {
                                slot,
                                from_menu,
                                path: slot_path(m, slot)?,
                            })
                        }
                        shell_state::Action::SaveSlot { slot } => Ok(Request::SaveSlot {
                            slot,
                            path: slot_path(m, slot)?,
                        }),
                    }
                }
                _ => Err(Error::Stop(
                    b.address,
                    "unhandled menu input boundary".into(),
                )),
            }
        }
        Screen::Briefing | Screen::Objectives => {
            let b = shell_input::briefing(m, registers(), keys)?;
            if b.address == 0x4682bb {
                request(m, b.registers.edx as u8)
            } else if m.u8(shell_state::MODE)? == 0 {
                Ok(Request::StartMission)
            } else {
                Ok(Request::None)
            }
        }
        Screen::Credits => Ok(
            if crate::campaign_flow::credits_input(m, keys.escape).map_err(Error::Memory)? {
                Request::Show(Screen::Main)
            } else {
                Request::None
            },
        ),
        Screen::Final => {
            let mut path = shell_briefing::cstring(m, 0x509594, 256)?;
            path.extend_from_slice(&shell_briefing::cstring(m, 0x49445b, 256)?);
            Ok(Request::CampaignMovie { path })
        }
        Screen::Result => {
            let gate = shell_input::result_gate(m, registers())?;
            if gate.address == 0x42c9d5 {
                let mut path = shell_briefing::cstring(m, 0x509594, 256)?;
                path.extend_from_slice(&shell_briefing::cstring(m, 0x494472, 256)?);
                Ok(Request::DefeatMovie { path })
            } else {
                let b = shell_input::victory(m, registers(), keys)?;
                Ok(if b.address == 0x46c08c {
                    Request::MissionNext
                } else {
                    Request::None
                })
            }
        }
    }
}
