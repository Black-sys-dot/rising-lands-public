//! Intact AffVegetTerrain object254 and AffBatiments terrain0x398 bodies.
//! Runtime dispatch remains with the renderer owner. No scene domain is widened.
use crate::{
    render::{i16, w, Context, Error, Observation, Renderer, Result},
    FrameRegisters, MemoriaJogo,
};
use std::collections::BTreeMap;

/// Completed AffVegetTerrain0x410085 register adapter, restricted to object254.
/// This body never reads auxiliary animation, consumes RNG or draws a sprite.
pub fn ground254(m: &MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let cell = r.ebx & 65535;
    let objects = m.u32(0x507d74)?;
    if m.u8(objects.wrapping_add(cell))? != 254 {
        return Err(Error::Stop(0x410085, "ground254 requires object254".into()));
    }
    let camera = u32::from(m.u8(0x508041)?);
    let discovery = m.u32(0x507d78 + 4 * camera)?;
    r.eax = (cell & 0xff00) | u32::from(m.u8(discovery.wrapping_add(cell))? & 31 == 0);
    r.edx = discovery;
    if r.eax & 255 != 0 {
        // Native makes this read even though both lighting branches return alike.
        let light = m.u32(0x507d88 + 4 * camera)?;
        let _ = m.u8(light.wrapping_add(cell))? & 16;
        r.eax = objects.wrapping_add(cell);
        r.edx = 254;
    }
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Exact effects/pixels and all eight completed return registers of the scoped
/// caller. Flags and transient stack are outside the API. Errors retain their
/// native effect prefix and publish no completed return registers.
pub struct FacadeOutcome {
    pub registers: Option<FrameRegisters>,
    pub effects: Vec<(u32, u8)>,
    pub observations: Vec<Observation>,
    pub error: Option<Error>,
}

/// Full AffBatiments0x407dd3 for terrain0x398/type4. Uses real region, ownership,
/// discovery, bag, health, clipping and selection inputs. No default game data.
pub fn facade398(
    renderer: &mut Renderer,
    m: &MemoriaJogo,
    mut r: FrameRegisters,
) -> Result<FacadeOutcome> {
    let cell = r.ebx & 65535;
    if m.u16(m.u32(0x507d70)?.wrapping_add(2 * cell))? != 0x398 {
        return Err(Error::Stop(
            0x40f23b,
            "facade398 requires terrain0x398".into(),
        ));
    }
    let camera = u32::from(m.u8(0x508041)?);
    if camera > 3 {
        return Err(Error::Stop(0x407e34, "facade398 camera outside0..3".into()));
    }
    let mut c = Context {
        background: &mut renderer.saved_background,
        text_boundaries: &mut renderer.recorded_text_boundaries,
        text_checked: 0,
        tick: renderer.tick,
        m,
        overrides: BTreeMap::new(),
        surface: &mut renderer.surface,
        cache: &mut renderer.cache,
        assets: &renderer.assets,
        observations: Vec::new(),
        audit: renderer.audit,
        clip: [
            i32::from(m.u16(0x49a020)?),
            i32::from(m.u16(0x49a01c)?),
            i32::from(m.u16(0x49a022)?),
            i32::from(m.u16(0x49a01e)?),
        ],
        camera,
    };
    let result = c.stage_objects_caller398(r.eax as i32, r.edx as i32, cell, r.ebx);
    let (registers, error) = match result {
        Ok([eax, ebx, edx]) => {
            r.eax = eax;
            r.ebx = ebx;
            r.edx = edx;
            r.esp = r.esp.wrapping_add(4);
            (Some(r), None)
        }
        Err(e) => (None, Some(e)),
    };
    Ok(FacadeOutcome {
        registers,
        effects: c.overrides.into_iter().collect(),
        observations: c.observations,
        error,
    })
}

impl Context<'_> {
    fn stage_objects_clip(&mut self, clip: [i32; 4]) -> Result<()> {
        let [l, t, r, b] = clip;
        // SetClip0x40f519 truncates each argument to a word.
        for (address, value) in [(0x49a020, l), (0x49a022, r), (0x49a01c, t), (0x49a01e, b)] {
            self.put16("stage_objects_clip", address, value as u32)?;
        }
        self.clip = clip.map(|v| v as u16 as i32);
        Ok(())
    }
    fn stage_objects_jitter(&mut self) -> Result<i32> {
        let cursor = self.u8(0x4e1d18)?;
        let value = self.u8(0x4e1c18 + cursor)? % 3;
        self.put(
            "stage_objects_permutation",
            0x4e1d18,
            &[(cursor as u8).wrapping_add(1)],
        )?;
        Ok(value as i32)
    }
    fn stage_objects_gray(&mut self, x: i32, y: i32, id: u32) -> Result<()> {
        self.observe("PasteGrisCmp", 0x45bcb0, &[x, y, id as i32, 1]);
        // The existing private remap_sprite cannot be called across modules.
        // Keep the native destination remap table explicit; it is not shadow.
        let sprite = self.assets.sprite(id)?;
        let table = self.m.read(self.u32(0x4e24c4)?, 256)?;
        let [l, t, r, b] = self.clip;
        for sy in 0.max(t - y)..(sprite.height as i32).min(b.min(599) - y + 1) {
            // Column 800 (collapse clip) wraps to the next row like PasteBobCmp.
            for sx in 0.max(l - x)..(sprite.width as i32).min(r.min(800) - x + 1) {
                if sprite.mask[sy as usize * sprite.width + sx as usize] != 0 {
                    let p = (y + sy) as usize * 800 + (x + sx) as usize;
                    if p >= self.surface.len() {
                        return self
                            .stop(0x45bcb0, "grey remap column 800 on the last surface row");
                    }
                    self.surface[p] = table[self.surface[p] as usize];
                }
            }
        }
        Ok(())
    }
    /// Reusable body0x40c19c..0x40c5a6, called after AffBatiments' visibility
    /// checks and clip save. Caller restores clipping and handles overlays.
    pub(crate) fn stage_objects_facade398(
        &mut self,
        x: i32,
        y: i32,
        cell: u32,
        index: u32,
        r: &[u8],
    ) -> Result<()> {
        let maximum = self.structure_maximum(r)?;
        if maximum == 0 {
            return self.stop(0x41a93b, "native IDIV fault: zero maximum health");
        }
        // Full quotient is saved, but the construction branch uses its low word.
        let state = (200 * w(r, 10) / maximum) as u16 as i32;
        let mut x = i16(x.wrapping_sub(47));
        let mut y = i16(y.wrapping_sub(44));
        let width = (self.u16(0x508724)? as i32) * 32;
        let height = (self.u16(0x508726)? as i32) * 32;
        if matches!(r[5], 3..=5) {
            if r[5] == 3 && !self.inhabited(index)? {
                let bottom = i16(y + 77 + self.stage_objects_jitter()?).min(height - 1);
                y = i16(y + 200 + self.stage_objects_jitter()? - w(r, 0x84) as i32).max(0);
                self.stage_objects_clip([0, y, width, bottom])?;
                x = i16(x + self.stage_objects_jitter()?);
            }
            if self.inhabited(index)? {
                self.bob(x, y, 0x1c79, true, Some(u32::from(r[4])))?;
            } else {
                self.bob(x, y, 0x1c79, true, None)?;
                self.stage_objects_gray(x, y, 0x1c79)?;
            }
            if w(r, 6) != 0 {
                // Native pushes 4, then calls AffFeuBatiment0x40323c through
                // shared0x40d5e2. Preserve the drawn/collapse prefix on stop.
                return self.aff_flamme(x, y, cell, index, 4);
            }
            return Ok(());
        }
        if state < 100 {
            self.stage_objects_clip([
                x.max(0),
                i16(y + (100 - state) * 77 / 100).max(0),
                width - 1,
                height - 1,
            ])?;
            return self.bob(x, y, 0x1c78, true, None);
        }
        let offset = (200 - state) * 77 / 100;
        self.stage_objects_clip([0, 0, width - 1, i16(y + offset).min(height - 1)])?;
        self.bob(x, y, 0x1c78, true, None)?;
        self.stage_objects_clip([x.max(0), i16(y + offset).max(0), width - 1, height - 1])?;
        self.bob(x, y, 0x1c79, true, None)?;
        if !self.inhabited(index)? {
            self.stage_objects_gray(x, y, 0x1c79)?;
        }
        Ok(())
    }
    fn stage_objects_caller398(&mut self, x: i32, y: i32, cell: u32, ebx: u32) -> Result<[u32; 3]> {
        let region = self.u16(self.u32(0x4e0f38)?.wrapping_add(2 * cell))?;
        let index = if region & 0x8000 != 0 {
            (region & 0x7fe0) >> 5
        } else {
            65535
        };
        let record = if index != 65535 {
            Some(self.read(0x49a0c8 + 276 * index, 276)?)
        } else {
            None
        };
        if let Some(r) = &record {
            if r[5] != 3 && u32::from(r[4]) != self.camera && r[0x110 + self.camera as usize] == 0 {
                // Owner query preserves EDX=index; caller replaces DH with
                // camera before this visibility test at0x407e34.
                return Ok([self.camera, ebx, (index & !0xff00) | (self.camera << 8)]);
            }
        }
        let discovery = self.u32(0x507d78 + 4 * self.camera)?;
        if self.u8(discovery.wrapping_add(cell))? & 16 != 0 {
            return Ok([cell & 0xff00, ebx, discovery]);
        }
        let [l, t, right, bottom] = self.clip;
        for (address, value) in [
            (0x49a02c, l),
            (0x49a02e, right),
            (0x49a030, t),
            (0x49a032, bottom),
        ] {
            self.put16("stage_objects_saved_clip", address, value as u32)?;
        }
        let code = self.u8(self.u32(0x507d74)?.wrapping_add(cell))?;
        if code == self.u8(0x495bf8)? || code == self.u8(0x495bf9)? {
            return self.stop(0x41b54a, "ChercheIndiceBatiment wall/gate dependency");
        }
        let Some(record) = record else {
            return Ok([self.u32(0x4e0f38)?, 65535, 65535]);
        };
        self.stage_objects_facade398(x, y, cell, index, &record)?;
        // AffForgeron0x402864 returns immediately for terrain0x398.
        self.stage_objects_clip([l, t, right, bottom])?;
        let inhabited = self.inhabited(index)?;
        if inhabited && record[5] != 3 {
            let owner = u32::from(record[4]);
            let selected = self.u16(0x495be0 + 2 * owner)?;
            if selected == index && owner == self.camera {
                return self.stop(0x4022c8, "type4 selection frame consumer");
            }
            if owner == self.camera && (self.u16(0x496ac4)? == index || selected == index) {
                return self.stop(0x4020ec, "type4 health gauge consumer");
            }
            // Final owner query0x41a08c; own-owner no-gauge branch then
            // clears EAX through XOR EAX,EDX at0x40f41f.
            let eax = if owner == self.camera {
                owner
            } else {
                ((0x49a0c8 + 276 * index) & !255) | owner
            };
            return Ok([eax, cell, index]);
        }
        // AffForgeron leaves its terrain pointer's upper word in EDX;
        // final type query0x41a4f4 is ANDed with0xff at0x40f45b.
        let edx = if inhabited {
            3
        } else {
            (self.u32(0x507d70)? & !65535) | 0x398
        };
        Ok([4, cell, edx])
    }
}
