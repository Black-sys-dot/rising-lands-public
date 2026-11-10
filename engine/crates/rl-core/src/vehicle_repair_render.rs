//! Intact AffVehiculeReparer 0x4029d0..0x402b19 (vehicle under repair in the inhabited
//! type-14 workshop, mode 4) with PorteEstOuverte 0x420c19 and AfficheSpriteVehicule
//! 0x4024ac..0x402721. Pure drawing: shadow and owner-palette sprites under a clip that
//! RestaureClipping 0x40f4e8 puts back from the saved clip words; no RNG or world write.
use crate::render::{i16, Context, Result};

impl Context<'_> {
    /// EAX = x, EDX = y (the caller's unadjusted facade origin), EBX = index.
    pub(crate) fn vehicle_repair(&mut self, x: i32, y: i32, index: u32) -> Result<()> {
        // The caller passes its stack words, sign-extended (0x40ad88..0x40ad93).
        let (x, y) = (i16(x), i16(y));
        let record = 0x49a0c8 + 276 * (index & 0xffff);
        // PorteEstOuverte: door open when phase byte +0x76 == 7 and +0x75 != 0.
        let open = self.u8(record + 0x76)? == 7 && self.u8(record + 0x75)? != 0;
        let progress = self.u16(record + 0x7e)? as u16 as i16;
        if !open && progress == 0 {
            return Ok(());
        }
        let kind = self.u8(record + 0x7b)?;
        // 402a5c..402ab1: jump table 0x4029bc on +0x7b - 11; any other value
        // reaches AfficheSpriteVehicule with uninitialised locals.
        let (dx, dy) = match kind {
            11 => (-76, 50),
            12 => (-84, 40),
            13 => (-85, 48),
            14 => (-72, 50),
            15 => (-83, 24),
            _ => return self.stop(0x402a65, "AffVehiculeReparer vehicle type outside 11..15"),
        };
        let left = i16(x - 0x3d).max(0);
        let top = i16(y + 6).max(0);
        let width = self.u16(0x508724)? as i32 * 32;
        let height = self.u16(0x508726)? as i32 * 32;
        self.set_clip(left, top, width, height)?;
        let frame = if progress < 2 { 2 } else { 6 };
        self.vehicle_sprite(kind, i16(x + dx), i16(y + dy), frame, index, false)?;
        // RestaureClipping: left/right/top/bottom from the saved words 0x49a02c..0x49a032.
        let saved = [
            self.u16(0x49a02c)? as i32,
            self.u16(0x49a030)? as i32,
            self.u16(0x49a02e)? as i32,
            self.u16(0x49a032)? as i32,
        ];
        self.set_clip(saved[0], saved[1], saved[2], saved[3])
    }
    /// AffVehiculeAtelier 0x402864 (called by the common AffBatiments tail 0x40f360 for
    /// every cell): on terrain word 0x356, the type-5 workshop ChercheCaseBatiment finds
    /// at the cell, with its door open (PorteEstOuverte 0x420c19), draws the vehicle
    /// under construction: clip from (x - 65, y - 102) while the progress word +0x7e is
    /// below 2, AfficheSpriteVehicule frame 5 with the bump flag at the per-type offset
    /// from (x - 154, y - 202) (table 0x402850), then RestaureClipping when +0x7e is 0.
    pub(crate) fn vehicle_atelier(&mut self, x: i32, y: i32, cell: u32) -> Result<()> {
        let (x, y) = (i16(x), i16(y));
        let mut found = None;
        for s in 0..1000u32 {
            let r = 0x49a0c8 + 276 * s;
            if self.u8(r + 5)? == 0 {
                continue;
            }
            let origin = self.u16(r + 2)?;
            let kind = self.u8(r + 8)?;
            let (w, h) = (
                self.u16(0x495d8c + 52 * kind)?,
                self.u16(0x495d8e + 52 * kind)?,
            );
            if (0..h).any(|dy| (0..w).any(|dx| origin + (dy << 8) + dx == cell)) {
                found = Some(s);
                break;
            }
        }
        let Some(s) = found else {
            return Ok(());
        };
        let record = 0x49a0c8 + 276 * s;
        if self.u8(record + 8)? != 5 {
            return Ok(());
        }
        if !(self.u8(record + 0x76)? == 7 && self.u8(record + 0x75)? != 0) {
            return Ok(());
        }
        let left = i16(x - 0x41).max(0);
        let top = i16(y - 0x66).max(0);
        let progress = self.u16(record + 0x7e)? as u16 as i16;
        if progress < 2 {
            let width = self.u16(0x508724)? as i32 * 32;
            let height = self.u16(0x508726)? as i32 * 32;
            self.set_clip(left, top, width, height)?;
        }
        let kind = self.u8(record + 0x7b)?;
        let (dx, dy) = match kind {
            11 => (0x2e, 0x20),
            12 => (0x14, 0x14),
            13 => (0x20, 0x20),
            14 => (0x22, 0x22),
            15 => (0x1e, 0x10),
            _ => {
                return self.stop(
                    0x4025b2,
                    "AffVehiculeAtelier vehicle type outside 11..15 (uninitialised sprite)",
                )
            }
        };
        self.vehicle_sprite(kind, i16(x - 0x9a + dx), i16(y - 0xca + dy), 5, s, true)?;
        if progress == 0 {
            let saved = [
                self.u16(0x49a02c)? as i32,
                self.u16(0x49a030)? as i32,
                self.u16(0x49a02e)? as i32,
                self.u16(0x49a032)? as i32,
            ];
            self.set_clip(saved[0], saved[1], saved[2], saved[3])?;
        }
        Ok(())
    }
    /// AfficheSpriteVehicule(AL = type 11..15, EDX = x, EBX = y, CL = frame,
    /// stack index, grey = 0, bump flag, owner +4). With the bump flag set and the
    /// progress word +0x7e above 1 the frame advances by one; both callers pass
    /// grey = 0, so the grey pass never runs.
    fn vehicle_sprite(
        &mut self,
        kind: u32,
        x: i32,
        y: i32,
        frame: u32,
        index: u32,
        bump: bool,
    ) -> Result<()> {
        let record = 0x49a0c8 + 276 * (index & 0xffff);
        let frame = if bump && self.u16(record + 0x7e)? as u16 > 1 {
            (frame + 1) & 0xff
        } else {
            frame
        };
        let (shadow, body) = match kind {
            11 => (0x109a, 0x1062),
            12 => (0x1272, 0x1212),
            13 => (0x1112, 0x10d2),
            14 => (0x1302, 0x12d2),
            _ => (0x11b2, 0x1152),
        };
        let owner = self.u8(record + 4)?;
        if owner > 4 {
            return self.stop(
                0x45c705,
                "AfficheSpriteVehicule owner palette table outside players",
            );
        }
        let phase = self.u32(0x508f80)? % 3;
        let px = i16(x + self.u16(record + 0x80)? as i32);
        let py = i16(y + self.u16(record + 0x82)? as i32);
        self.shadow(px, py, (shadow + 4 * frame + phase) & 0xffff, false)?;
        self.bob(
            px,
            py,
            (body + 4 * frame + phase) & 0xffff,
            true,
            Some(owner),
        )?;
        if kind == 12 {
            self.bob(px, py, 0x1245, true, Some(owner))?;
        }
        if kind == 15 {
            self.bob(px, py, 0x1189, true, Some(owner))?;
        }
        Ok(())
    }
}
