//! AffBatiments tail 0x40f36a..0x40f48c for every inhabited structure type:
//! BatimentPoseCadre 0x4022c8 (the selection frame, per-type corner sprite
//! sizes through BobGetTaille) and AffJaugeBatiment 0x4020ec (gauge sprite,
//! BatimentGetEtat/BatimentGetDegat bar, FilledBox) with the generic
//! AffContenanceBatiment 0x401a70 bag slots. Listings: intact RISING.EXE
//! 0x4020ec..0x40226f, 0x4022c8..0x402496, 0x401a70..0x401cc5 (jump tables
//! 0x402094 and 0x402270). Types 2 (mode 5), 3 and 4 keep their earlier
//! ports; this module serves the other types.
use crate::render::{i16, w, Context, Result};

/// BatimentPoseCadre 0x402270 jump table: corner frame sprite per type.
/// Type 9 jumps to 0x4023a0 with the caller's ECX (the structure index) as
/// the sprite id; type 12 returns at 0x4022f3 without a frame.
const FRAME_SPRITES: [u32; 22] = [
    0x1ba3, 0x1b75, 0x1b77, 0x1bb5, 0x1c78, 0x1c09, 0x1cb5, 0x1ca8, 0x1c7a, 0, 0x1cb4, 0x1cbf, 0,
    0x1bf7, 0x1c3d, 0x1c94, 0x1c52, 0x1c6f, 0x1cae, 0x1c64, 0x1c94, 0x1c6e,
];
/// AffJaugeBatiment 0x402094 jump table: y offset of the gauge per type
/// (type 11 also moves x by 16; type 12 draws nothing).
const GAUGE_DY: [i32; 22] = [
    0x52, 0x7d, 0x5a, 0x22, 0xe, 0x53, 0x42, 0x23, 0x23, 0, -6, 6, 0, 0x4e, 0x32, 0x35, 0x32, 6,
    0x14, 0x13, 0x35, 0x13,
];
/// The facade origin the AffBatiments locals (di, si) hold at the tail,
/// relative to the facade cell, as each ported facade handler sets it.
/// Types 5, 8 and 13: the handlers subtract these at 0x40a3f1, 0x40c629 and
/// 0x409df2. None: types 9 and 12, walls and gates, draw no facade block.
const FACADE_ORIGIN: [Option<(i32, i32)>; 22] = [
    Some((27, 93)),
    Some((94, 154)),
    Some((150, 93)),
    Some((57, 47)),
    Some((47, 44)),
    Some((122, 106)),
    Some((48, 97)),
    Some((52, 76)),
    Some((157, 62)),
    None,
    Some((25, 37)),
    Some((44, 45)),
    None,
    Some((91, 99)),
    Some((74, 47)),
    Some((61, 65)),
    Some((84, 73)),
    Some((6, 33)),
    Some((28, 68)),
    Some((47, 46)),
    Some((61, 65)),
    Some((44, 45)),
];

/// The AffBatiments locals (di, si) each facade handler leaves for the tail,
/// relative to the facade cell: the origin belongs to the terrain word's body,
/// whatever the record's type (0x3ce serves types 15 and 20 alike).
pub(crate) fn facade_origin(tile: u32) -> Option<(i32, i32)> {
    let kind = match tile {
        0x2fb => 0,
        0x2cb => 1,
        0x2ed => 2,
        0x30d => 3,
        0x398 => 4,
        0x343 => 5,
        0x42a => 6,
        0x3e1 => 7,
        0x3b0 => 8,
        0x406 => 10,
        0x436 => 11,
        0x326 => 13,
        0x35e => 14,
        0x3ce => 15,
        0x37a => 16,
        0x390 => 17,
        0x3f8 => 18,
        0x388 => 19,
        0x40e => 21,
        _ => return None,
    };
    FACADE_ORIGIN[kind]
}

impl Context<'_> {
    /// frame: BatimentPoseCadre for the highlighted structure
    /// (`[0x495be0 + 2 * owner] == index`); gauge: AffJaugeBatiment for the
    /// selected or highlighted one. Both receive the facade origin.
    pub fn structure_overlay_generic(
        &mut self,
        r: &[u8],
        cell: u32,
        origin: (i32, i32),
        frame: bool,
        gauge: bool,
    ) -> Result<()> {
        let kind = usize::from(r[8]);
        if kind >= FRAME_SPRITES.len() {
            return self.stop(
                if frame { 0x4022c8 } else { 0x4020ec },
                "structure overlay type outside the jump tables",
            );
        }
        let (dx, dy) = origin;
        let ox = self.u16(0x508720)?;
        let oy = self.u16(0x508722)?;
        let x = (cell as i32 % 256 - ox as i32) * 32 - dx;
        let y = (cell as i32 / 256 - oy as i32) * 32 - dy;
        let owner = u32::from(r[4]);
        if frame && kind != 12 {
            let id = if kind == 9 {
                // 4023a0 reached straight from the table: CX is the caller's
                // structure index (0x40f3da mov eax, ecx).
                self.u16(0x495be0 + 2 * owner)?
            } else {
                FRAME_SPRITES[kind]
            };
            let Ok(sprite) = self.assets.sprite(id) else {
                return self.stop(0x4023ad, "BobGetTaille outside the loaded bob catalog");
            };
            let width = sprite.width as i32;
            let height = sprite.height as i32;
            let phase = (self.u32(0x508f80)? & 7) as i32;
            for (sprite, px, py) in [
                (0x2a79, x + phase - 20, y + phase),
                (0x2a78, x + phase - 20, y + height - phase),
                (0x2a7a, x + width + 3 - phase, y + phase),
                (0x2a77, x + width + 3 - phase, y + height - phase),
            ] {
                self.bob(i16(px), i16(py), sprite, true, Some(owner))?;
            }
        }
        if !gauge || kind == 12 {
            return Ok(());
        }
        self.gauge_at(r, x, y)
    }
    /// AffJaugeBatiment 0x4020ec body from (di, si) = (x, y).
    pub(crate) fn gauge_at(&mut self, r: &[u8], x: i32, y: i32) -> Result<()> {
        let kind = usize::from(r[8]);
        let ox = self.u16(0x508720)?;
        let mut gx = i16(x - 23 + if kind == 11 { 16 } else { 0 });
        let gy = i16(y + GAUGE_DY[kind]);
        if ox == 0 && gx < 0 {
            gx = 0;
        }
        self.bob(i16(gx + 9), gy, 0x2a7b, true, None)?;
        let maximum = self.u16(0x495d90 + 52 * kind as u32)?;
        if maximum == 0 {
            return self.stop(0x41a93b, "structure gauge zero maximum health");
        }
        // BatimentGetEtat 0x41a8e5 and BatimentGetDegat 0x41a940: AX =
        // 200 * word / maximum; the bar height compares as a 16-bit value.
        let health = (200 * w(r, 10) / maximum) as u16;
        let damage = (200 * w(r, 6) / maximum) as u16;
        let height = (i32::from(health) - i32::from(damage)) * 60 / 200;
        let colour = if height as u16 <= 20 {
            145
        } else if height as u16 <= 40 {
            180
        } else {
            48
        };
        self.filled(
            i16(gx + 10),
            i16(gy + 61 - height),
            i16(gx + 14),
            i16(gy + 60),
            colour,
        )?;
        match kind {
            19 => self.contenance_hospi(r, gx, gy),
            8 => self.contenance_hangar(r, gx, gy),
            _ => self.bag_slots(r, gx, gy).map(|_| ()),
        }
    }
    /// Bag entry `slot` of the structure's occupant list: [r + 0x12 + 2 * i]
    /// (BagGet 0x4472de on the bag at r + 0xc), and the unit record.
    fn occupant(&self, r: &[u8], slot: usize) -> Result<u32> {
        if 0x14 + 2 * slot > r.len() {
            return self.stop(0x4472de, "bag entry beyond the structure record");
        }
        let id = w(r, 0x12 + 2 * slot);
        Ok(self.u32(0x4e19a0)?.wrapping_add(276 * id))
    }
    /// AffContenanceHangar 0x401729: a fixed marker, then from 32 px right
    /// and 48 px down one owner-coloured marker per occupant of the
    /// structure's owner, the first occupant of kind 9 skipped (0x40181d),
    /// and empty markers up to half of [0x495be9].
    fn contenance_hangar(&mut self, r: &[u8], gx: i32, gy: i32) -> Result<()> {
        let owner = u32::from(r[4]);
        self.bob(gx, gy, 0x2a3c, true, Some(owner))?;
        let (mut x, y) = (gx + 0x20, i16(gy + 0x30));
        let count = w(r, 0xc) as usize;
        let mut pilot_skipped = false;
        let mut drawn = 0u32;
        for slot in 0..count {
            let unit = self.occupant(r, slot)?;
            if self.u8(unit + 1)? == 9 && !pilot_skipped {
                pilot_skipped = true;
                continue;
            }
            if self.u8(unit)? == owner {
                self.bob(i16(x), y, 0x2a3c, true, Some(owner))?;
                drawn += 1;
                x += 8;
            }
        }
        let empty = self.u8(0x495be9)? / 2;
        while drawn & 0xffff < empty {
            self.bob(i16(x), y, 0x2a3b, true, None)?;
            drawn += 1;
            x += 8;
        }
        Ok(())
    }
    /// AffContenanceHospi 0x40186c. Left column: one marker per occupant of
    /// kind 9 at full health, then empty ones up to the capacity
    /// (0x41f3ed for type 19: table byte minus [0x495be8]). Row 32 px right
    /// and 48 px below it: one marker per wounded occupant, then empty ones
    /// up to [0x495be8]. The 0x4019fa recheck repeats an equal comparison,
    /// so its 0x401a13 tail is dead.
    fn contenance_hospi(&mut self, r: &[u8], gx: i32, gy: i32) -> Result<()> {
        let owner = u32::from(r[4]);
        let count = w(r, 0xc) as usize;
        let (x, mut y) = (gx, gy);
        let mut well = 0u32;
        let mut wounded = Vec::new();
        for slot in 0..count {
            let unit = self.occupant(r, slot)?;
            let kind = self.u8(unit + 1)?;
            let healed = self.u16(unit + 0x9a)? == self.u16(0x4965b6 + 50 * kind)?;
            if !healed {
                wounded.push(());
            } else if kind == 9 {
                self.bob(i16(x), i16(y), 0x2a3c, true, Some(owner))?;
                well += 1;
                y += 8;
            }
        }
        let capacity =
            (self.u8(0x495d9e + 52 * 19)? as u8).wrapping_sub(self.u8(0x495be8)? as u8) as u32;
        while well & 0xffff < capacity {
            self.bob(i16(x), i16(y), 0x2a3b, true, None)?;
            well += 1;
            y += 8;
        }
        let (mut x, y) = (x + 0x20, y + 0x30);
        for _ in &wounded {
            self.bob(i16(x), i16(y), 0x2a3e, true, None)?;
            x += 8;
        }
        let mut sick = wounded.len() as u32;
        let beds = self.u8(0x495be8)?;
        while sick & 0xffff < beds {
            self.bob(i16(x), i16(y), 0x2a3b, true, None)?;
            sick += 1;
            x += 8;
        }
        Ok(())
    }
    /// AffContenanceBatiment 0x401ae2..0x401b76: every bag slot up to the
    /// capacity, the second column of eight 9 px to the left.
    /// AffContenanceBatiment 0x401aff..0x401b74: one slot per bag entry,
    /// eight per column, the ninth starting a new column 9 pixels left.
    /// Returns the (x, y) the loop leaves in ESI/ECX for the type tails
    /// (0x401b89: the CAVE level row starts there).
    pub(crate) fn bag_slots(&mut self, r: &[u8], gx: i32, gy: i32) -> Result<(i32, i32)> {
        let capacity = w(r, 14) as usize;
        if 0x4e + capacity > r.len() {
            return self.stop(0x401a70, "bag capacity beyond the structure record");
        }
        let (mut bx, mut by) = (gx, gy);
        for slot in 0..capacity {
            if slot == 8 {
                bx = i16(bx - 9);
                by = gy;
            }
            let troop = r[0x4e + slot] == 1;
            self.bob(
                bx,
                i16(by),
                if troop { 0x2a3c } else { 0x2a3b },
                true,
                troop.then_some(u32::from(r[4])),
            )?;
            by += 8;
        }
        Ok((bx, by))
    }
}
