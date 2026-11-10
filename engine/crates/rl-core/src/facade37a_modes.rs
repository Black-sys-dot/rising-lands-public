//! Intact facade37a (AffBatiments terrain 0x37a, type 16) outside the finished modes 4/5:
//! the mode-3 collapse clip with its three permutation jitters (0x40b88a..0x40b9a8) and
//! the construction modes 0..2, 6, 7 (0x40bb83..0x40be08), which never read +6.
use crate::render::{i16, w, Context, Result};

impl Context<'_> {
    /// Mode 3 of an uninhabited facade37a: bottom = y + 113 + jitter (at most the last
    /// view row), top = max(0, y + 200 + jitter - +0x84), clip (0, top, width, bottom),
    /// x += jitter. Returns the adjusted origin.
    pub(crate) fn facade37a_collapse(&mut self, x: i32, y: i32, r: &[u8]) -> Result<(i32, i32)> {
        let width = self.u16(0x508724)? as i32 * 32;
        let height = self.u16(0x508726)? as i32 * 32;
        let bottom = i16(y + 113 + self.permutation_third()?).min(height - 1);
        let top = i16(y + 200 + self.permutation_third()? - w(r, 0x84) as i32).max(0);
        self.set_clip(0, top, width, bottom)?;
        let x = i16(x + self.permutation_third()?);
        Ok((x, top))
    }
    /// Construction modes at the adjusted origin (x - 84, y - 73): scaffold 0x1c52 under
    /// the health clip, then (from half health) base 0x1c53 and parts 0x1c54 (+2, +28)
    /// and 0x1c5c (+86, +37), greyed when uninhabited.
    pub(crate) fn facade37a_construction(
        &mut self,
        x: i32,
        y: i32,
        index: u32,
        r: &[u8],
    ) -> Result<()> {
        let maximum = self.u16(0x495d90 + 52 * u32::from(r[8]))?;
        let state = (200 * w(r, 10) / maximum) as u16 as i32;
        let width = self.u16(0x508724)? as i32 * 32;
        let height = self.u16(0x508726)? as i32 * 32;
        if state < 100 {
            self.set_clip(
                x.max(0),
                i16(y + (100 - state) * 113 / 100).max(0),
                width - 1,
                height - 1,
            )?;
            return self.bob(x, y, 0x1c52, true, None);
        }
        let offset = (200 - state) * 113 / 100;
        self.set_clip(0, 0, width - 1, i16(y + offset).min(height - 1))?;
        self.bob(x, y, 0x1c52, true, None)?;
        self.set_clip(x.max(0), i16(y + offset).max(0), width - 1, height - 1)?;
        let parts = [
            (0x1c53, x, y),
            (0x1c54, i16(x + 2), i16(y + 28)),
            (0x1c5c, i16(x + 86), i16(y + 37)),
        ];
        for (id, px, py) in parts {
            self.bob(px, py, id, true, None)?;
        }
        if !self.inhabited(index)? {
            for (id, px, py) in parts {
                self.observe("PasteGrisCmp", 0x45bcb0, &[px, py, id as i32, 1]);
                self.remap_sprite(px, py, id, 0x4e24c4)?;
            }
        }
        Ok(())
    }
    /// The shared collapse jitter: permutation byte at [0x4e1d18] through the table
    /// 0x4e1c18, modulo 3, cursor incremented (published like the other facades).
    fn permutation_third(&mut self) -> Result<i32> {
        let cursor = self.u8(0x4e1d18)?;
        let value = self.u8(0x4e1c18 + cursor)? % 3;
        self.put("rng_u8", 0x4e1d18, &[(cursor as u8).wrapping_add(1)])?;
        Ok(value as i32)
    }
}
