//! EvenementAffiche 0x4581bc drawing (AffGame 0x417c8a in mode 18 and the
//! modal mode 22 at 0x4178b3), without the FLC pixels and palette: the host
//! decodes the declared `Events\NAME.FLC` and overlays its frame at the
//! centred rectangle documented in `motor/include/event-flc.md`.
use crate::render::{i16, Context, Result};
impl Context<'_> {
    /// FilledBoxGris 0x45cd91 with its clip argument 0 (whole screen bounds):
    /// each pixel through the grey table [0x4e24c4]. Same body as the scripted
    /// renderer's shaded box, kept here so the event path owns its call site.
    fn event_grey_box(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> Result<()> {
        self.observe("FilledBoxGris", 0x45cd91, &[x0, y0, x1, y1, 0]);
        let r = self.u16(0x49a018)? as i32 - 1;
        let b = self.u16(0x49a01a)? as i32 - 1;
        if x0 > r || y0 > b || x1 < 0 || y1 < 0 {
            return Ok(());
        }
        let (x0, y0, x1, y1) = (x0.max(0), y0.max(0), x1.min(r).min(799), y1.min(b).min(599));
        let table = self.m.read(self.u32(0x4e24c4)?, 256)?.to_vec();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = y as usize * 800 + x as usize;
                self.surface[p] = table[self.surface[p] as usize];
            }
        }
        Ok(())
    }
    /// The drawing of one EvenementAffiche call, from the state at AffGame
    /// entry. Just opened (or last frame reported): grey the view above the
    /// 42-pixel HUD band. Playing: the frame border bob 0x2b1c 16 pixels
    /// outside the centred FLC rectangle; the host then overlays the frame.
    pub(crate) fn event_affiche(&mut self) -> Result<()> {
        if self.u8(0x4e1b0e)? == 0 {
            return Ok(());
        }
        let width = self.u16(0x49a018)? as i32;
        let height = self.u16(0x49a01a)? as i32;
        let handle = self.u32(0x4e1b10)?;
        if self.u8(0x4e1b14)? != 0 || self.u16(0x4e1b0c)? >= self.u16(handle + 6)? {
            return self.event_grey_box(0, 0, i16(width - 1), i16(height - 0x2a));
        }
        let x = (width - self.u16(handle + 8)? as i32) / 2;
        let y = (height - self.u16(handle + 10)? as i32) / 2;
        self.bob(i16(x - 16), i16(y - 16), 0x2b1c, true, None)
    }
    /// AffGame mode 22 (0x4178b3): surface lock, EvenementAffiche, unlock, flip.
    pub(crate) fn compose_event(&mut self) -> Result<u32> {
        if self.u32(0x4e17a8)? == 0 || self.u32(0x4e17a4)? == 0 {
            self.put32("pitch_u32", 0x4e17a0, 800)?;
            self.put32("dd_ponteiro_u32", 0x4e17a4, 0x08000000)?;
        }
        self.put32("superficie_u32", 0x508f70, self.u32(0x4e17a4)?)?;
        self.event_affiche()?;
        Ok(0)
    }
}
