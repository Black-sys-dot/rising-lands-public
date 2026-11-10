//! AffGame/terrain/AffSprite ordering, including the original unstable sorts.
use crate::render::{dw, i16, w, Context, Error, Result};
pub(crate) const FACADES: [u32; 19] = [
    0x2cb, 0x2ed, 0x2fb, 0x30d, 0x326, 0x343, 0x35e, 0x37a, 0x388, 0x390, 0x398, 0x3b0, 0x3ce,
    0x3e1, 0x3f8, 0x406, 0x40e, 0x42a, 0x436,
];
impl Context<'_> {
    pub fn compose(&mut self, hud: bool) -> Result<u32> {
        if self.u8(0x4e2f78)? == 0x16 {
            return self.compose_event();
        }
        if self.u8(0x4e2f78)? != 0x12 {
            return self.stop(0x4176a8, "AffGame mode: only 0x12 and 0x16 are composed");
        }
        let mut count = self.u16(0x4e17ae)?.wrapping_add(1) & 65535;
        self.put16("fps_contador_u16", 0x4e17ae, count)?;
        let tick = self.u32(0x4e17b4)?;
        let tick = self.tick.unwrap_or(if tick == 0 { 1500 } else { tick });
        self.put32("fps_tick_u32", 0x4e17b4, tick)?;
        if tick.wrapping_sub(self.u32(0x4e17b0)?) >= 1000 {
            self.put32("fps_ultimo_tick_u32", 0x4e17b0, tick)?;
            if count > 99 {
                count = 99;
                self.put16("fps_contador_u16", 0x4e17ae, count)?;
            }
            self.put(
                "fps_digitos",
                0x496394,
                &[(count / 10 + 48) as u8, (count % 10 + 48) as u8, 0],
            )?;
            self.put16("fps_contador_u16", 0x4e17ae, 0)?;
        }
        let text = format!("{}\0", self.u8(0x49a05b)?).into_bytes();
        let mut page = self.read(0x4963b4, 4)?;
        page[..text.len()].copy_from_slice(&text);
        self.put("paginas_texto", 0x4963b4, &page)?;
        let event = self.u8(0x4e1b0e)? != 0;
        if self.u8(0x4e05a2)? == 0 {
            let divisor = self.u8(0x497284)?;
            if divisor == 0 {
                return self.stop(0x417921, "draw divisor 0 faults (DIV)");
            }
            if self.u32(0x508f7c)? % divisor != 0 {
                return Ok(1);
            }
        }
        if self.u8(0x508f92)? != 0 && self.u16(0x49a018)? == 800 {
            // 0x41794b..0x4179c9: FilledBox with colour 2 and no clip on the
            // video and system surfaces; this renderer keeps a single surface.
            let width = (self.u16(0x508724)? as usize * 32).min(800);
            let height = (self.u16(0x508726)? as usize * 32).min(600);
            for line in 0..height {
                self.surface[line * 800..line * 800 + width].fill(2);
            }
        }
        if self.u32(0x4e17a8)? == 0 || self.u32(0x4e17a4)? == 0 {
            self.put32("pitch_u32", 0x4e17a0, 800)?;
            self.put32("dd_ponteiro_u32", 0x4e17a4, 0x08000000)?;
        }
        self.put32(
            "dd_travas_u32",
            0x4e17a8,
            self.u32(0x4e17a8)?.wrapping_add(1),
        )?;
        self.put32("superficie_u32", 0x508f70, self.u32(0x4e17a4)?)?;
        self.camera = self.u8(0x508041)?;
        if self.camera > 3 {
            return Err(Error::Invalid("camera must be 0..3"));
        }
        self.clip = [
            self.u16(0x49a020)? as i32,
            self.u16(0x49a01c)? as i32,
            self.u16(0x49a022)? as i32,
            self.u16(0x49a01e)? as i32,
        ];
        if self.u8(0x49a041)? != 0 {
            self.restore_background()?;
        }
        let ox = self.u16(0x508720)?;
        let oy = self.u16(0x508722)?;
        let width = self.u16(0x508724)?;
        let height = self.u16(0x508726)?;
        let margin = self.u8(0x508f90)? as u8 as i8 as i32;
        if self.u16(0x49a018)? != 800
            || self.u32(0x4e17a0)? != 800
            || width * 32 != 800
            || self.u16(0x49a01a)? != 600
            || ox > 255
            || oy > 255
            || !(1..=32).contains(&height)
            || margin < 0
            || margin * 2 > width.min(height) as i32
            || ox + width > 256
            || oy + height > 256
        {
            return self.stop(
                0x41068e,
                "AffTerrain viewport outside the reconstructed interior domain",
            );
        }
        let terrain = self.m.read(self.u32(0x507d70)?, 131072)?;
        let objects = self.m.read(self.u32(0x507d74)?, 65536)?;
        let aux = self.m.read(self.u32(0x507dac)?, 65536)?;
        let discovery = self.m.read(self.u32(0x507d78 + 4 * self.camera)?, 65536)?;
        let light = self.m.read(self.u32(0x507d88 + 4 * self.camera)?, 65536)?;
        for row in margin..height as i32 - margin {
            for col in margin..width as i32 - margin {
                let cell = ((oy as i32 + row) * 256 + ox as i32 + col) as usize;
                let raw = w(terrain, cell * 2);
                let source = if (702..=1081).contains(&raw) && light[cell] & 16 != 0 {
                    349 + (cell as u32 % 256) % 8 + 8 * ((cell as u32 / 256) % 8)
                } else {
                    raw
                };
                self.block(col * 32, row * 32, source, discovery[cell], light[cell])?;
                self.object(
                    col * 32,
                    row * 32,
                    objects[cell],
                    aux[cell],
                    discovery[cell],
                    light[cell],
                    false,
                )?;
            }
        }
        let units = self.m.read(self.u32(0x4e19a0)?, 220800)?;
        let projectiles = self.m.read(self.u32(0x5080f0)?, 5600)?;
        let mut selected = Vec::new();
        for i in 0..800 {
            let r = &units[i * 276..(i + 1) * 276];
            let cell = w(r, 8) as usize;
            if r[2] == 0 || discovery[cell] & 31 != 0 || light[cell] & 16 != 0 {
                continue;
            }
            let x = cell as u32 % 256;
            let y = cell as u32 / 256;
            if x >= ox && y >= oy && x < ox + width + 2 && y < oy + height + 10 {
                selected.push(i);
            }
        }
        for p in 0..selected.len() {
            let mut c = p;
            for other in p + 1..selected.len() {
                let a = &units[selected[c] * 276..];
                let b = &units[selected[other] * 276..];
                if w(b, 8) < w(a, 8) || (w(b, 8) == w(a, 8) && b[0xab] < a[0xab]) {
                    c = other;
                }
            }
            selected.swap(p, c);
        }
        let cell_of = |i: usize| {
            let r = &projectiles[i * 56..];
            ((w(r, 14) / 32) * 256 + w(r, 12) / 32) & 65535
        };
        let mut projs = Vec::new();
        for i in 0..100 {
            let r = &projectiles[i * 56..];
            let key = cell_of(i) as usize;
            let x = w(r, 12) / 32;
            let y = w(r, 14) / 32;
            if r[2] != 0
                && discovery[key] & 31 == 0
                && light[key] & 31 == 0
                && x >= ox
                && y >= oy
                && x < ox + width + 2
                && y < oy + height + 10
            {
                projs.push(i);
            }
        }
        for p in 0..projs.len() {
            let mut c = p;
            for other in p + 1..projs.len() {
                if cell_of(projs[other]) <= cell_of(projs[c]) {
                    c = other;
                }
            }
            projs.swap(p, c);
        }
        let mut cursor = 0;
        for row in 0..height + 5 {
            for col in 0..width + 8 {
                // AffSprite 416b19/416ac6 carries across physical rows, then
                // 416b5a truncates the packed index. Do not wrap each axis.
                let cell = ((oy + row) * 256 + ox + col) as u16 as usize;
                let (x, y) = (col as i32 * 32, row as i32 * 32);
                if objects[cell] != 0 && discovery[cell] & 31 == 0 {
                    self.object(
                        x,
                        y,
                        objects[cell],
                        aux[cell],
                        discovery[cell],
                        light[cell],
                        true,
                    )?;
                }
                self.building(
                    x,
                    y,
                    cell as u32,
                    w(terrain, cell * 2),
                    objects[cell],
                    discovery[cell],
                )?;
                while cursor < selected.len() && units[selected[cursor] * 276 + 0xab] != 0 {
                    cursor += 1;
                }
                while cursor < selected.len()
                    && w(units, selected[cursor] * 276 + 8) == cell as u32
                    && units[selected[cursor] * 276 + 0xab] == 0
                {
                    let i = selected[cursor];
                    self.unit(x, y, i as u32, &units[i * 276..(i + 1) * 276])?;
                    cursor += 1;
                }
            }
        }
        cursor = 0;
        let mut pc = 0;
        for row in 0..height + 10 {
            for col in 0..width + 8 {
                let cell = ((oy + row) * 256 + ox + col) as u16 as u32;
                while pc < projs.len() && cell_of(projs[pc]) == cell {
                    let i = projs[pc];
                    self.projectile(
                        &projectiles[i * 56..(i + 1) * 56],
                        ox,
                        oy,
                        light[cell as usize],
                    )?;
                    pc += 1;
                }
                while cursor < selected.len() && units[selected[cursor] * 276 + 0xab] == 0 {
                    cursor += 1;
                }
                while cursor < selected.len()
                    && w(units, selected[cursor] * 276 + 8) == cell
                    && units[selected[cursor] * 276 + 0xab] != 0
                {
                    let i = selected[cursor];
                    self.unit(
                        col as i32 * 32,
                        row as i32 * 32,
                        i as u32,
                        &units[i * 276..(i + 1) * 276],
                    )?;
                    cursor += 1;
                }
            }
        }
        if self.u8(0x4e19b0)? != 0 {
            let x0 = self.u16(0x49a024)? as u16 as i16 as i32;
            let y0 = self.u16(0x49a026)? as u16 as i16 as i32;
            let x1 = (self.u16(0x49a028)? as u16 as i16 as i32).min(width as i32 * 32);
            let y1 = (self.u16(0x49a02a)? as u16 as i16 as i32).min(height as i32 * 32);
            self.outline(x0, y0, x1, y1, 1, true)?;
        }
        if hud {
            self.hud(event)?;
        }
        Ok(0)
    }
    fn block(
        &mut self,
        x: i32,
        y: i32,
        source: u32,
        discovery: u8,
        illumination: u8,
    ) -> Result<()> {
        let mut id = source;
        if discovery & 16 != 0 {
            id = 1081;
        } else {
            let fog = discovery & 15;
            let light = if illumination & 16 != 0 {
                15
            } else {
                illumination & 15
            };
            if fog != 0 || light != 0 {
                let key = [source as u8, (source >> 8) as u8, light, fog];
                if let Some(slot) = self
                    .cache
                    .keys
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .position(|k| *k == key)
                {
                    id = 1090 + slot as u32;
                } else {
                    if !(1..=1081).contains(&source) {
                        return self.stop(0x40fd95,format!("GetNumBob source {source}: miss requires verified full-literal block"));
                    }
                    let q = self.assets.sprite(source)?;
                    if !raw_block(q) {
                        return self.stop(0x40fd95, "GetNumBob source outside raw 32x33 domain");
                    }
                    let slot = self.cache.cursor as usize;
                    self.cache.keys[slot * 4..slot * 4 + 4].copy_from_slice(&key);
                    self.cache.cursor = ((slot + 1) % 256) as u16;
                    let start = slot * 1056;
                    self.cache.raw[start..start + 1056].copy_from_slice(&q.raw);
                    const MASKS: [u32; 16] = [
                        0, 1085, 1084, 1088, 1083, 1081, 1087, 1081, 1082, 1089, 1081, 1081, 1086,
                        1081, 1081, 1081,
                    ];
                    for (mask, shadow) in [(light, true), (fog, false)] {
                        if mask == 0 {
                            continue;
                        }
                        let q = self.assets.sprite(MASKS[mask as usize])?;
                        for row in 0..32 {
                            for col in 0..32 {
                                let i = row * 32 + col;
                                let p = start + row * 33 + 1 + col;
                                if q.mask[i] != 0 {
                                    self.cache.raw[p] = if shadow {
                                        self.assets.shadow[self.cache.raw[p] as usize]
                                    } else {
                                        q.pixels[i]
                                    };
                                }
                            }
                        }
                    }
                    id = 1090 + slot as u32;
                }
            }
        }
        if x < 0 || y < 0 || x > 768 || y > 568 {
            return Err(Error::Invalid("unclipped AffBloc does not fit"));
        }
        let raw = if (1090..=1345).contains(&id) {
            let start = (id - 1090) as usize * 1056;
            &self.cache.raw[start..start + 1056]
        } else {
            if !(1..=1081).contains(&id) {
                return self.stop(
                    0x45c72a,
                    format!("AffBloc engine ID {id} outside full-block domain"),
                );
            }
            let q = self.assets.sprite(id)?;
            if !raw_block(q) {
                return self.stop(0x45c72a, "AffBloc source is not a raw block");
            }
            &q.raw
        };
        for row in 0..32 {
            let dest = (y as usize + row) * 800 + x as usize;
            self.surface[dest..dest + 32].copy_from_slice(&raw[row * 33 + 1..row * 33 + 33]);
        }
        Ok(())
    }
    // Original render inputs stay explicit rather than being inferred from a cell.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn object(
        &mut self,
        x: i32,
        y: i32,
        code: u8,
        aux: u8,
        discovery: u8,
        light: u8,
        sprites: bool,
    ) -> Result<()> {
        // AffVegetTerrain has no ground layer for explosions. AffVeget's
        // dispatch at 0x415c79..0x415e71 uses the unsigned aux byte modulo 32.
        if matches!(code, 56..=61 | 63..=65) {
            if !sprites || discovery & 31 != 0 {
                return Ok(());
            }
            let layers: &[(u32, i32, i32)] = match code {
                56 | 59 => &[(0x1ff5, 11, 4)],
                57 => &[(0x2015, 11, -11)],
                58 => &[(0x2035, 11, -18)],
                60 => &[(0x2015, 11, -11), (0x1ff5, 28, 15)],
                61 => &[(0x1ff5, 0, 0), (0x2035, 11, -18), (0x2015, 28, 8)],
                // Intact dispatch 415eb0/415ee6/415f1c. These are the
                // three smoke objects emitted by type3 dismantling.
                63 => &[(0x2075, -10, 0)],
                64 => &[(0x2095, -20, 0)],
                65 => &[(0x20b5, -24, -16)],
                _ => unreachable!(),
            };
            for &(base, dx, dy) in layers {
                self.bob(
                    (x + dx) as i16 as i32,
                    (y + dy) as i16 as i32,
                    base + u32::from(aux % 32),
                    true,
                    None,
                )?;
            }
            return Ok(());
        }
        if code == 27 {
            // AffVegetTerrain0x41059a..0x410122 has no ground layer for27, and
            // AffVeget0x415af0 returns for undiscovered cells before dispatch.
            if !sprites || discovery & 31 != 0 {
                return Ok(());
            }
            // AffVeget0x415b3a..0x415bd7: one permutation byte (modulo2) lifts
            // the lit flame by aux-rem; both layers are PasteEclairCmp.
            let cursor = self.u8(0x4e1d18)?;
            let rem = self.u8(0x4e1c18 + cursor)? % 2;
            self.put("rng_u8", 0x4e1d18, &[(cursor as u8).wrapping_add(1)])?;
            let first = (0x23c5 + u32::from(aux)) & 65535;
            self.shadow(i16(x), i16(y - (i32::from(aux) - rem as i32)), first, true)?;
            self.shadow(i16(x), i16(y), 0x23c5 + (u32::from(aux) + 3) % 16, true)?;
            return Ok(());
        }
        if sprites {
            if !matches!(code,1..=11|55..=65) {
                return Ok(());
            }
            if !(1..=11).contains(&code) {
                return self.stop(0x415acf, format!("AffVeget handler for object code {code}"));
            }
        } else {
            if matches!(code, 0 | 253 | 254) || discovery & 31 != 0 {
                return Ok(());
            }
        }
        let address = if sprites { 0x415acf } else { 0x410085 };
        if (1..=11).contains(&code) {
            if aux > 3 {
                return self.stop(address, "tree phase outside 0..3");
            }
            if !sprites || discovery & 31 != 0 {
                return Ok(());
            }
            for &(base, dx, dy, shadow) in TREES[code as usize - 1] {
                let id = base + aux as u32 + if !shadow && light & 16 != 0 { 92 } else { 0 };
                if shadow {
                    self.shadow(x + dx, y + dy, id, false)?;
                } else {
                    self.bob(x + dx, y + dy, id, true, None)?;
                }
            }
            return Ok(());
        }
        if sprites {
            return Ok(());
        }
        let lit = light & 16 == 0;
        // Intact410345/410396/4105b9/4103e2: unsigned aux selects the raw frame.
        // The same silhouette remaps the destination only on unlit terrain.
        if matches!(code, 23..=26) {
            let id = 0x2385 + 16 * u32::from(code - 23) + u32::from(aux);
            self.bob(x, y, id, true, None)?;
            if !lit {
                self.shadow(x, y, id, false)?;
            }
            return Ok(());
        }
        if matches!(code,12..=22|28..=52) {
            let (dx, dy) = if (12..=22).contains(&code) {
                LOW[(code - 12) as usize]
            } else {
                (0, 0)
            };
            let id =
                if code <= 22 { 0x2941 } else { 0x293c } + code as u32 + if lit { 0 } else { 64 };
            self.bob(x + dx, y + dy, id, true, None)?;
        } else if matches!(code, 54 | 66..=68) {
            if code != 54 && aux > 31 {
                return self.stop(address, "sequential object phase outside 0..31");
            }
            let id = 0x2a47
                + if code == 54 {
                    0
                } else {
                    (code as u32 - 66) * 16 + aux as u32 / 2
                };
            let dx = if matches!(code, 54 | 66) { -4 } else { 0 };
            self.bob(x + dx, y, id, true, None)?;
            if !lit {
                self.shadow(x + dx, y, id, false)?;
            }
        } else {
            return self.stop(
                address,
                format!("AffVegetTerrain object code {code} outside validated catalog"),
            );
        }
        Ok(())
    }
    pub fn inhabited(&self, index: u32) -> Result<bool> {
        let a = 0x49a0c8 + index * 276;
        let kind = self.u8(a + 8)?;
        let count = self.u16(a + 14)?;
        let units = self.u32(0x4e19a0)?;
        let mut colonist = false;
        for slot in 0..count {
            if self.u8(a + 0x4e + slot)? != 1 {
                continue;
            }
            let index = self.u16(a + 0x12 + slot * 2)?;
            let t = self.u8(units + index * 276 + 1)?;
            if (kind == 2 || kind == 21) && !matches!(t, 3 | 11..=17 | 25) {
                return Ok(true);
            }
            if t == 9 {
                colonist = true;
            }
        }
        Ok(kind != 21 && colonist)
    }
    pub(crate) fn building(
        &mut self,
        x: i32,
        y: i32,
        cell: u32,
        tile: u32,
        code: u8,
        discovery: u8,
    ) -> Result<()> {
        let region = self.u16(self.u32(0x4e0f38)? + cell * 2)?;
        let index = if region & 0x8000 != 0 {
            (region & 0x7fe0) >> 5
        } else {
            65535
        };
        let record = if index != 65535 {
            if index >= 1000 {
                return self.stop(0x407e1e, "structure index beyond pool");
            }
            Some(self.read(0x49a0c8 + index * 276, 276)?)
        } else {
            None
        };
        if let Some(r) = &record {
            if r[5] != 3 && r[4] as u32 != self.camera && r[0x110 + self.camera as usize] == 0 {
                return Ok(());
            }
        }
        if discovery & 16 != 0 {
            return Ok(());
        }
        let [l, t, r, b] = self.clip;
        let mut saved = Vec::new();
        for v in [l, r, t, b] {
            saved.extend_from_slice(&(v as u16).to_le_bytes());
        }
        self.put("recorte_salvo", 0x49a02c, &saved)?;
        if code == 254 {
            let Some(record) = &record else {
                // Native lookup returns ffff and exits before clip restoration.
                return Ok(());
            };
            self.wall_facade(x, y, record)?;
        } else if code == 253 {
            let Some(record) = &record else {
                // Native lookup returns ffff and exits before clip restoration.
                return Ok(());
            };
            self.gate_facade(x, y, record)?;
        } else if tile == 0x2cb {
            if let Some(r) = &record {
                self.type1_facade(x, y, cell, index, r)?;
            }
        } else if tile == 0x2ed {
            if let Some(r) = &record {
                let inhabited = self.inhabited(index)?;
                if !matches!(r[5], 3..=5) || !inhabited {
                    self.construction_facade(x, y, cell, index, r)?;
                } else {
                    if r[4] > 4 {
                        return Err(Error::Invalid("facade owner"));
                    }
                    let a = self.u32(0x508f7c)?;
                    let b = self.u32(0x508f80)?;
                    let c = self.u32(0x508f84)?;
                    for (id, dx, dy, owner) in [
                        (7032, 0, 0, true),
                        (7049 + ((c.wrapping_add(cell)) & 7), 74, 5, true),
                        (7033 + ((a.wrapping_add(cell)) & 7), 89, 95, false),
                        (7041 + ((b.wrapping_add(cell)) & 7), 151, 131, false),
                        (7059 + (a & 7), 48, 64, true),
                        (7067 + (a & 7), 144, 74, true),
                        (7057 + ((a.wrapping_add(cell)) & 1), 18, 63, false),
                    ] {
                        self.bob(
                            x + dx - 150,
                            y + dy - 93,
                            id,
                            true,
                            if owner { Some(r[4] as u32) } else { None },
                        )?;
                    }
                    // 408ab3: a damage word continues to AffFlamme type 2 (40d5e2)
                    // at the adjusted facade origin (no collapse jitter when inhabited).
                    if w(r, 6) != 0 {
                        self.aff_flamme(x - 150, y - 93, cell, index, 2)?;
                    }
                }
            }
        } else if tile == 0x2fb {
            let Some(record) = &record else {
                // Native408e47 returns before clip restoration for no record.
                return Ok(());
            };
            self.facade2fb(x, y, cell, index, record)?;
        } else if tile == 0x30d {
            if let Some(record) = &record {
                self.construction_facade(x, y, cell, index, record)?;
            }
        } else if tile == 0x37a {
            if let Some(record) = &record {
                self.facade37a(x, y, cell, index, record)?;
            }
        } else if tile == 0x3f8 {
            let Some(record) = &record else {
                // Native40d8d4 returns before the common clip restoration.
                return Ok(());
            };
            self.facade3f8(x, y, cell, index, record)?;
        } else if tile == 0x398 {
            if let Some(record) = &record {
                self.stage_objects_facade398(x, y, cell, index, record)?;
            }
        } else if tile == 0x42a {
            let Some(record) = &record else {
                // Native40e231 returns before the common clip restoration.
                return Ok(());
            };
            self.facade42a(x, y, cell, index, record)?;
        } else if tile == 0x35e {
            let Some(record) = &record else {
                // Native40abf5 returns before the common clip restoration.
                return Ok(());
            };
            self.facade35e(x, y, cell, index, record)?;
        } else if tile == 0x388 {
            let Some(record) = &record else {
                // Native40b2eb returns before the common clip restoration.
                return Ok(());
            };
            self.facade388(x, y, cell, index, record)?;
        } else if tile == 0x3e1 {
            let Some(record) = &record else {
                // Native40d20a returns before the common clip restoration.
                return Ok(());
            };
            self.facade3e1(x, y, cell, index, record)?;
        } else if tile == 0x390 {
            let Some(record) = &record else {
                // Native40be52 returns before the common clip restoration.
                return Ok(());
            };
            self.facade390(x, y, cell, index, record)?;
        } else if tile == 0x3b0 {
            let Some(record) = &record else {
                // Native40c5f4 returns before the common clip restoration.
                return Ok(());
            };
            self.facade3b0(x, y, cell, index, record)?;
        } else if tile == 0x343 {
            let Some(record) = &record else {
                // Native40a3bc returns before the common clip restoration.
                return Ok(());
            };
            self.facade343(x, y, cell, index, record)?;
        } else if tile == 0x326 {
            let Some(record) = &record else {
                // Native409dbd returns before the common clip restoration.
                return Ok(());
            };
            self.facade326(x, y, cell, index, record)?;
        } else if tile == 0x3ce {
            let Some(record) = &record else {
                // Native40cca5 returns before the common clip restoration.
                return Ok(());
            };
            self.facade3ce(x, y, cell, index, record)?;
        } else if tile == 0x406 {
            let Some(record) = &record else {
                // Native40ddbe returns before the common clip restoration.
                return Ok(());
            };
            self.facade406(x, y, cell, index, record)?;
        } else if tile == 0x436 {
            let Some(record) = &record else {
                // Native40e9b3 returns before the common clip restoration.
                return Ok(());
            };
            self.facade436(x, y, cell, index, record)?;
        } else if tile == 0x40e {
            let Some(record) = &record else {
                // Native 40ee3e returns before the common clip restoration.
                return Ok(());
            };
            self.facade40e(x, y, cell, index, record)?;
        } else if FACADES.contains(&tile) {
            return self.stop(0x40f23b, format!("building facade terrain word 0x{tile:x}"));
        }
        if tile == 0x356 {
            // 0x40f360: AffVehiculeAtelier on the workshop door word.
            self.vehicle_atelier(x, y, cell)?;
        }
        self.clip = [l, t, r, b];
        let mut clip = Vec::new();
        for v in [t, b, l, r] {
            clip.extend_from_slice(&(v as u16).to_le_bytes());
        }
        self.put("recorte", 0x49a01c, &clip)?;
        if let Some(r) = record {
            let inhabited = self.inhabited(index)?;
            if inhabited && code != 254 && r[5] != 3 {
                if r[4] >= 5 {
                    return self.stop(0x40f3c2, "owner outside selection words");
                }
                let selected = self.u16(0x495be0 + 2 * r[4] as u32)?;
                let frame = selected == index && r[4] as u32 == self.camera;
                let gauge = r[4] as u32 == self.camera
                    && (self.u16(0x496ac4)? == index || selected == index);
                if frame || gauge {
                    // 0x40f36a: the tail runs with ECX = the index set by the facade
                    // handler, so the frame and gauge are drawn once, on the facade
                    // cell only, for every type.
                    if !FACADES.contains(&tile) {
                        return Ok(());
                    }
                    self.structure_overlay(&r, cell, tile, frame, gauge)?;
                }
            } else if r[8] == 17 && r[4] as u32 == self.camera && self.u16(0x496ac4)? == index {
                // 0x40f444: an uninhabited own type-17 under the cursor gets the
                // gauge from every cell of its footprint with this cell's x, y
                // (the facade handler moves them to its origin on that cell).
                let (gx, gy) = if FACADES.contains(&tile) {
                    (i16(x - 6), i16(y - 33))
                } else {
                    (x, y)
                };
                return self.gauge_at(&r, gx, gy);
            }
        }
        Ok(())
    }
    pub fn projectile(&mut self, r: &[u8], ox: u32, oy: u32, light: u8) -> Result<()> {
        if r[2] == 0 || light & 31 != 0 {
            return Ok(());
        }
        let kind = r[0] as u32;
        if kind == 0 || kind == 28 || kind == 33 || kind > 34 {
            return Ok(());
        }
        let phase = r[24] as u32;
        let dir = r[34] as i32;
        let x = w(r, 12) as i32 - ox as i32 * 32;
        let y = w(r, 14) as i32 - oy as i32 * 32;
        let displacement = (dw(r, 36) >> 8) as i32;
        let mut layers = Vec::new();
        if let Some(&(base, dx, dy, raised)) = SIMPLE.get(kind as usize).and_then(|x| x.as_ref()) {
            layers.push((
                base + phase,
                x + dx,
                y + dy - if raised { displacement } else { 0 },
                0,
            ));
        } else {
            match kind {
                9..=13 => {
                    layers.push((0x2a87 + kind, x - 1, y - 1, 1));
                    layers.push((0x2a87 + kind, x - 1, y - 1 - displacement, 0));
                }
                24 | 25 => {
                    let (base, dx, dy) = if kind == 24 {
                        (0x2155, -75, -52)
                    } else {
                        (0x2255, -91, -59)
                    };
                    layers.push((
                        base + phase + ((dir - 1) / 2) as u32 * 32,
                        x + dx,
                        y + dy,
                        0,
                    ));
                }
                27 => layers.push((
                    ((0x1eb5i32 + phase as i32 + (dir - 1) * 8) & 65535) as u32,
                    x - 68,
                    y - 49,
                    0,
                )),
                29 | 30 => {
                    let (base, offset) = if kind == 29 {
                        (0x2c1d, 8)
                    } else {
                        (0x2bbd, 33)
                    };
                    for step in [0, 32, 64] {
                        layers.push((base + phase + step, x - offset, y - offset, 2));
                    }
                }
                31 | 32 => {
                    let op = if kind == 31 { 1 } else { 2 };
                    layers.push((0x20b5 + phase, x - 24, y - 16, op));
                    layers.push((0x2075 + phase, x, y, op));
                }
                _ => {}
            }
        }
        for (id, x, y, op) in layers {
            let id = id & 65535;
            if id == 0 || id > 12156 {
                return self.stop(
                    0x46ff70,
                    "AffProjectile raw state selects ID outside catalog",
                );
            }
            let (x, y) = (super::render::i16(x), super::render::i16(y));
            if op == 0 {
                self.bob(x, y, id, true, None)?;
            } else {
                self.shadow(x, y, id, op == 2)?;
            }
        }
        Ok(())
    }
}
fn raw_block(q: &crate::render_assets::Sprite) -> bool {
    q.width == 32 && q.height == 32 && q.raw.len() == 1056 && (0..32).all(|y| q.raw[y * 33] == 224)
}
const LOW: [(i32, i32); 11] = [
    (0, 0),
    (16, 16),
    (0, 0),
    (18, 11),
    (18, 0),
    (18, 25),
    (0, 25),
    (0, 0),
    (0, 0),
    (-3, -5),
    (-1, -1),
];
const TREES: [&[(u32, i32, i32, bool)]; 11] = [
    &[(10525, 2, -27, false), (10553, 9, 13, true)],
    &[(10521, -4, -36, false), (10549, 6, 21, true)],
    &[(10517, -3, -83, false), (10545, 16, 6, true)],
    &[
        (10517, 3, -89, false),
        (10545, 22, 0, true),
        (10525, -3, -19, false),
        (10553, 4, 21, true),
    ],
    &[
        (10517, -4, -91, false),
        (10545, 15, -2, true),
        (10525, -7, -24, false),
        (10553, 0, 16, true),
        (10521, 15, -34, false),
        (10549, 25, 23, true),
    ],
    &[(10537, -4, -25, false), (10565, 9, 16, true)],
    &[(10533, -7, -43, false), (10561, 14, 14, true)],
    &[(10529, -11, -65, false), (10557, 12, 13, true)],
    &[
        (10529, -15, -75, false),
        (10557, 8, 3, true),
        (10537, 7, -17, false),
        (10565, 20, 24, true),
    ],
    &[
        (10529, -7, -74, false),
        (10557, 16, 4, true),
        (10533, -18, -40, false),
        (10561, 3, 17, true),
        (10537, 11, -17, false),
        (10565, 24, 24, true),
    ],
    &[(10541, -5, -36, false), (10569, 14, 14, true)],
];
const SIMPLE: [Option<(u32, i32, i32, bool)>; 35] = [
    None,
    Some((10876, -9, -7, false)),
    Some((10880, -9, -7, true)),
    Some((10884, -9, -7, true)),
    Some((10888, -9, -7, true)),
    Some((10892, -9, -7, true)),
    Some((8309, 0, 0, false)),
    Some((8341, -20, 0, false)),
    Some((8373, -24, -16, false)),
    None,
    None,
    None,
    None,
    None,
    Some((10906, 0, 0, false)),
    Some((10938, 0, 0, false)),
    Some((10954, 0, 0, false)),
    Some((10880, -9, -7, false)),
    Some((8469, -37, -38, false)),
    Some((8501, -33, -31, false)),
    Some((9053, -5, -5, false)),
    Some((9045, -6, -6, false)),
    Some((9061, -3, -3, false)),
    Some((9069, -4, -4, false)),
    None,
    None,
    Some((10970, 0, 0, false)),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some((8405, -81, -54, false)),
];
