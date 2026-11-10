//! Original gameplay surface initialization, RL-VIS-001.
//!
//! MenuInit's game-entry call at 0x468476 and RestoreAll's calls at
//! 0x474b6a/0x474bb1 load the complete PLATGAME.PIE at (0,0), pitch 800.
//! AffGame subsequently redraws the world and dynamic HUD over those pixels.
//! This provider has no world, RNG, cache, palette or platform side effects.
use crate::{frame::stop, render_assets::sha256, shell_picture, Result};

pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;
pub const PIXELS: usize = WIDTH * HEIGHT;
pub const ORIGINAL_SHA256: [u8; 32] = [
    218, 106, 7, 201, 58, 106, 2, 124, 216, 28, 229, 14, 77, 252, 117, 104, 72, 34, 59, 217, 81,
    244, 166, 14, 112, 186, 177, 233, 188, 60, 32, 167,
];

/// Decode once from the same original asset used by render_assets::Assets.
/// Its palette remains owned by Assets; native LoadPie skips the 768 RGB bytes.
pub struct GameHudBackground {
    pixels: Vec<u8>,
}
impl GameHudBackground {
    pub fn load(original_pie: &[u8]) -> Result<Self> {
        if sha256(original_pie) != ORIGINAL_SHA256 {
            return stop("Gameplay background original asset SHA-256", 0x4799c4);
        }
        let mut pixels = vec![0; PIXELS];
        shell_picture::pie(original_pie, &mut pixels, WIDTH)?;
        Ok(Self { pixels })
    }

    /// At game entry/reset/restore, before terrain, selection, counters and cursor.
    /// This copies the *whole* original picture, including its black world area.
    /// Do not call per frame: that would erase retained HUD/tooltip pixels.
    /// A host resize that only scales the 800x600 framebuffer needs no reset.
    pub fn initialize_surface(&self, surface: &mut [u8]) -> Result<()> {
        if surface.len() != PIXELS {
            return stop("Gameplay background requires 800x600 surface", 0x479adb);
        }
        surface.copy_from_slice(&self.pixels);
        Ok(())
    }
}
