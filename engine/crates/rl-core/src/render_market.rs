//! AffMarche 0x46677d: the market icon, the proposal panel and the settlement
//! gauge, from the original asm. Memory writes go through `put` (they stay within
//! this render); `market::display_tick` applies the ones the logic reads.
use crate::render::{i16, Context, Result, SavedBackground};
use crate::render_assets::Sprite;

const WX: u32 = 0x4e2906;
const WY: u32 = 0x496f80;
const HOME: u32 = 0x4e2928;
const PARTNER: u32 = 0x496f8c;
const PAGE: u32 = 0x496f88;
const ACCEPTED: u32 = 0x4e292c;
const TAB: u32 = 0x496fd0;
const VALUE: u32 = 0x4e2930;
const COUNT_A: u32 = 0x4e2938;
const COUNT_B: u32 = 0x4e2940;
const GIVE_UNITS: u32 = 0x4e29bc;
const WANT_UNITS: u32 = 0x4e2a8c;
const GIVE_GOODS: u32 = 0x4e2cfc;
const WANT_GOODS: u32 = 0x4e2d14;
const AVAIL_UNITS: u32 = 0x4e2d2c;
const AVAIL_GOODS: u32 = 0x4e2dfc;
const ON_SALE: u32 = 0x4e2e14;
const SELECT_KIND: u32 = 0x496f82;
const SELECT_ITEM: u32 = 0x496f83;
const SELECT_PAGE: u32 = 0x496f84;
const INFO_ON: u32 = 0x4e2910;
const INFO_LOCK: u32 = 0x4e2911;
const INFO_KIND: u32 = 0x4e2913;
const INFO_COUNT: u32 = 0x4e2914;
const INFO_ITEM: u32 = 0x4e2915;

fn tradable(kind: u32) -> bool {
    let k = kind as u8;
    if k < 0x12 {
        k < 0x10
    } else {
        k > 0x18
    }
}

impl Context<'_> {
    fn window(&self) -> Result<(i32, i32)> {
        Ok((self.u16(WX)? as i32, self.u16(WY)? as i32))
    }
    fn mouse(&self) -> Result<(i32, i32)> {
        Ok((self.u16(0x5099a8)? as i32, self.u16(0x5099aa)? as i32))
    }
    fn side(&self, p: u32) -> Result<u32> {
        Ok(if self.u8(HOME + p)? == 1 {
            p
        } else {
            self.u8(PARTNER + p)?
        })
    }
    fn set_byte(&mut self, a: u32, v: u32) -> Result<()> {
        self.put("market_byte", a, &[v as u8])
    }
    fn paste(&mut self, x: i32, y: i32, id: u32) -> Result<()> {
        self.bob(i16(x), i16(y), id, false, None)
    }
    fn paste_player(&mut self, x: i32, y: i32, id: u32, owner: u32) -> Result<()> {
        self.bob(i16(x), i16(y), id, false, Some(owner))
    }
    /// PasteEclairCmp and PasteOmbreCmp with their clip argument.
    fn shade(&mut self, x: i32, y: i32, id: u32, light: bool, clipped: bool) -> Result<()> {
        let (x, y) = (i16(x), i16(y));
        self.observe(
            if light {
                "PasteEclairCmp"
            } else {
                "PasteOmbreCmp"
            },
            if light { 0x45ba23 } else { 0x45b796 },
            &[x, y, id as i32, clipped as i32],
        );
        let clip = if clipped {
            self.clip
        } else {
            self.screen_clip()
        };
        self.raster(id, x, y, if light { 2 } else { 1 }, 0, clip)
    }
    /// FilledBox 0x45cacd with flag 0: the screen bounds, not the clip window.
    fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u8) -> Result<()> {
        let saved = self.clip;
        self.clip = self.screen_clip();
        let r = self.filled(x0, y0, x1, y1, color);
        self.clip = saved;
        r
    }
    /// PasteBlocCmpHalf 0x45b164 and PasteBlocCmpHalfEclair 0x45b074: every second
    /// compressed pixel of every second row, written contiguously.
    fn half(&mut self, x: i32, y: i32, id: u32, light: bool) -> Result<()> {
        let (x, y) = (i16(x), i16(y));
        self.observe(
            if light {
                "PasteBlocCmpHalfEclair"
            } else {
                "PasteBlocCmpHalf"
            },
            if light { 0x45b074 } else { 0x45b164 },
            &[x, y, id as i32],
        );
        let assets = self.assets;
        let q: &Sprite = assets.sprite(id)?;
        let width = q.width as i32;
        let mut stream = (0..q.height)
            .flat_map(|r| (0..q.width).map(move |c| (r, c)))
            .filter(|&(r, c)| q.mask[r * q.width + c] != 0)
            .map(|(r, c)| q.pixels[r * q.width + c]);
        let mut dest = y as i64 * 800 + x as i64;
        'rows: for row in 0..q.height as i32 {
            for column in 0..width {
                let Some(v) = stream.next() else {
                    break 'rows;
                };
                if column % 2 == 0 && row % 2 == 0 {
                    if !(0..800 * 600).contains(&dest) {
                        return self.stop(0x45b1e1, "PasteBlocCmpHalf outside surface");
                    }
                    self.surface[dest as usize] = if light { assets.light[v as usize] } else { v };
                    dest += 1;
                }
            }
            if row % 2 == 0 {
                dest += 800 - (width / 2) as i64;
            }
        }
        Ok(())
    }
    /// SetInfoAide 0x428ef1 with DL = 0.
    fn set_info(&mut self, text: u32, number: u32) -> Result<()> {
        let current = self.u32(0x4e060c)?;
        let threshold = self.u16(0x49620a)?;
        if text != current {
            self.put16("info_aide_compteur_u16", 0x4e0610, threshold)?;
        } else {
            let count = self.u16(0x4e0610)?;
            if count < threshold {
                self.put16("info_aide_compteur_u16", 0x4e0610, count + 1)?;
            }
        }
        self.put32("info_aide_u32", 0x4e060c, text)?;
        self.put16("info_aide_nombre_u16", 0x49620c, number)
    }
    /// TextGetMessage 0x478a3b.
    fn message_text(&self, n: u32) -> Result<u32> {
        self.u32(self.u32(0x5099a4)? + 4 * (n & 0xffff))
    }
    /// AffTextEncadre 0x45a218, font 1: a boxed number at the bottom layer.
    fn number_box(&mut self, x: i32, y: i32, number: u32) -> Result<()> {
        let text = number.to_string().into_bytes();
        let mut width = 2;
        for &b in &text {
            width += self.assets.sprite(0x23d5 + b as u32 - 32)?.width as i32;
        }
        let mut height = self.assets.sprite(0x23d5 + 0x6a - 32)?.height as i32;
        let (mut x, mut y) = (0.max(i16(x)), 0.max(i16(y)));
        if y + height + 6 > self.u16(0x49a01a)? as i32 - 8 {
            y = self.u16(0x49a01a)? as i32 - 9 - height - 6;
        }
        if x + width + 6 > self.u16(0x49a018)? as i32 {
            x = self.u16(0x49a018)? as i32 - width - 6;
        }
        width += 6 - i32::from(width % 2 == 0);
        height += 6;
        for (name, a, value) in [
            ("tooltip_x", 0x49a046, x),
            ("tooltip_y", 0x49a048, y),
            ("tooltip_w", 0x49a04a, width),
            ("tooltip_h", 0x49a04c, height),
        ] {
            self.put16(name, a, value as u32)?;
        }
        if x < 0 || y < 0 || x + width > 800 || y + height > 600 {
            return self.stop(0x45a3d5, "AffTextEncadre save outside the screen");
        }
        let mut saved = Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            let p = (y + row) as usize * 800 + x as usize;
            saved.extend_from_slice(&self.surface[p..p + width as usize]);
        }
        let address = self.u32(0x49a03c)?;
        *self.background = Some(SavedBackground {
            address,
            pixels: saved,
        });
        self.outline(x, y, x + width - 1, y + height - 1, 1, false)?;
        self.shaded_box(x + 1, y + 1, x + width - 2, y + height - 2, true, false)?;
        self.afftext(
            &text,
            [x + 3, y + 1, x + width - 2, y + height - 2],
            1,
            [0, 0],
        )
    }
    fn on_sale_count(&self, source: u32) -> Result<u32> {
        let mut n = 0;
        for i in 0..26 {
            if self.u8(ON_SALE + 26 * source + i)? != 0 && tradable(i) {
                n += 1;
            }
        }
        Ok(n)
    }
    /// AffPersonnage 0x46447d and AffPersonnageContact 0x464710.
    fn persons(&mut self, p: u32, base: i32, contact: bool) -> Result<()> {
        if self.u16(TAB + 2 * p)? != 1 {
            return Ok(());
        }
        let (wx, wy) = self.window()?;
        let (ix, iy, scroll, page) = if contact {
            (0x496f3c, 0x496f40, 0x4e290c, 1)
        } else {
            (0x496e94, 0x496e98, 0x4e290e, 0)
        };
        self.paste(wx + self.u16(ix)? as i32, wy + self.u16(iy)? as i32, 0x2af9)?;
        let (mut slot, mut skipped) = (0i32, 0u32);
        let (mut left, mut lit) = (6i32, 0i32);
        let mut gap = 0;
        for i in 0..26u32 {
            if self.u16(AVAIL_UNITS + 52 * p + 2 * i)? != 0 {
                if skipped & 0xffff != self.u16(scroll)? {
                    skipped = skipped.wrapping_add(1);
                } else {
                    let x = left + gap + wx + 0x21;
                    let y = wy + base;
                    let id = if i == 0x19 { 0x29e1 } else { 0x29cf + i };
                    self.paste_player(x, y, id, p)?;
                    if self.u8(SELECT_KIND)? == 1
                        && i == self.u8(SELECT_ITEM)?
                        && self.u8(SELECT_PAGE)? == page
                    {
                        self.shade(left + lit + wx + 0x21, y, 0x29cf + i, true, false)?;
                    }
                    let slot_x = (slot + 1) * 6 + slot * 0x38 + wx;
                    let (mx, my) = self.mouse()?;
                    if mx > slot_x + 0x21 && my > y && mx < slot_x + 0x59 && my < y + 0x30 {
                        self.set_info(0x4965ba + 50 * i, 0xffff)?;
                    }
                    slot += 1;
                    gap += 0x37;
                    lit += 0x38;
                    left += 6;
                }
            }
            if slot > 2 {
                break;
            }
        }
        let mut gap = slot * 0x37;
        let mut left = slot * 6 + 6;
        while slot < 3 {
            let y = wy + base;
            let x = left + gap + wx;
            slot += 1;
            self.fill(x + 0x21, y, x + 0x58, y + 0x2f, 0x2a)?;
            gap += 0x37;
            left += 6;
        }
        Ok(())
    }
    /// AffRessource 0x465569 and AffRessourceContact 0x4656c8.
    fn goods(&mut self, p: u32, base: i32, contact: bool) -> Result<()> {
        if self.u16(TAB + 2 * p)? != 2 {
            return Ok(());
        }
        let (wx, wy) = self.window()?;
        let (ix, iy, page) = if contact {
            (0x496f54, 0x496f58, 1)
        } else {
            (0x496eac, 0x496eb0, 0)
        };
        self.paste(wx + self.u16(ix)? as i32, wy + self.u16(iy)? as i32, 0x2afa)?;
        let mut n = 0;
        for r in 0..3u32 {
            let mine = self.u8(PAGE + p)?;
            if !((self.u16(AVAIL_GOODS + 6 * p + 2 * r)? as i32 > 0 && mine == 0) || mine == 1) {
                continue;
            }
            let id = 0x29f0 + r;
            let y = wy + base;
            let x = n * 8 + wx + 0x27 + n * 0x35;
            self.paste(x, y, id)?;
            if self.u8(SELECT_KIND)? == 2
                && r == self.u8(SELECT_ITEM)?
                && self.u8(SELECT_PAGE)? == page
            {
                self.shade(x, y, id, true, false)?;
            }
            n += 1;
        }
        Ok(())
    }
    /// The number overlay shared by the four miniature lists.
    fn info_number(
        &mut self,
        x: i32,
        y: i32,
        kind: u32,
        item: u32,
        n: u32,
        page: u32,
    ) -> Result<()> {
        if self.u8(INFO_ON)? == 0
            || self.u8(INFO_LOCK)? != 0
            || self.u8(INFO_KIND)? != kind
            || self.u8(INFO_ITEM)? != item
            || page != 0
        {
            return Ok(());
        }
        self.number_box(x, y, n)?;
        let c = (self.u8(INFO_COUNT)? + 1) & 255;
        self.set_byte(INFO_COUNT, c)?;
        if c > 0x1e {
            for a in [INFO_ON, INFO_KIND, INFO_ITEM, INFO_COUNT, INFO_LOCK] {
                self.set_byte(a, 0)?;
            }
        }
        Ok(())
    }
    /// AffMiniEmplacementsDonne 0x463abb and AffMiniEmplacementsVeut 0x463f96.
    fn miniatures(&mut self, h: u32, y0: i32, units: u32, goods: u32, want: bool) -> Result<()> {
        let (wx, wy) = self.window()?;
        let mut slot = 0i32;
        let required = u32::from(want);
        let count_table = if want { WANT_UNITS } else { GIVE_UNITS };
        let mut d = 0;
        for i in 0..26u32 {
            let n = self.u16(units + 2 * i)?;
            if n == 0 {
                continue;
            }
            let x = d + wx + 0x28;
            let y = wy + y0;
            self.half(x, y, if i == 0x19 { 0x29e1 } else { 0x29cf + i }, !want)?;
            let page = (self.u8(PAGE + h)? != required) as u32;
            if self.u8(SELECT_KIND)? == 1 && i == self.u8(SELECT_ITEM)? && page == 0 {
                self.set_byte(SELECT_KIND, 0xff)?;
                self.set_byte(SELECT_ITEM, 0xff)?;
            }
            self.info_number(x, y, 1, i, n, page)?;
            let hover_x = slot * 0x1e + wx;
            let (mx, my) = self.mouse()?;
            if mx > hover_x + 0x28 && my > y && mx < hover_x + 0x4d && my < y + 0x18 {
                let count = self.u16(count_table + 52 * h + 2 * i)?;
                self.set_info(0x4965ba + 50 * i, count)?;
            }
            d += 0x1e;
            slot += 1;
        }
        let mut d = slot * 0x1e;
        for r in 0..3u32 {
            let n = self.u16(goods + 2 * r)?;
            if n == 0 {
                continue;
            }
            let id = 0x29f0 + r;
            let x = d + wx + 0x28;
            let y = wy + y0;
            self.half(x, y, id, false)?;
            let page = (self.u8(PAGE + h)? != required) as u32;
            if self.u8(SELECT_KIND)? == 2 && r == self.u8(SELECT_ITEM)? && page == 0 {
                self.half(x, y, id, true)?;
                self.set_byte(SELECT_KIND, 0xff)?;
                self.set_byte(SELECT_ITEM, 0xff)?;
            }
            self.info_number(x, y, 2, r, n, page)?;
            let hover_x = slot * 0x1e + wx;
            let (mx, my) = self.mouse()?;
            if mx > hover_x + 0x28 && my > y && mx < hover_x + 0x4d && my < y + 0x18 {
                self.set_info(self.message_text(0x28 + r)?, n)?;
            }
            d += 0x1e;
            slot += 1;
        }
        let mut d = slot * 0x1e;
        while slot < 6 {
            let y = wy + y0;
            let x = d + wx;
            self.fill(x + 0x28, y, x + 0x43, y + 0x17, 0x2a)?;
            d += 0x1e;
            slot += 1;
        }
        Ok(())
    }
    /// AffAlliance 0x465a34.
    fn alliance(&mut self, p: u32) -> Result<()> {
        let (wx, wy) = self.window()?;
        let dh = self.u8(PARTNER + p)?;
        let mut n = 0;
        for q in 0..4u32 {
            if q == dh {
                continue;
            }
            let at = self.u32(0x5080a8)? + 20 * dh + 3;
            let allied = dh <= 4 && (self.u32(at)? >> 24) & (1 << q) != 0;
            if !allied {
                continue;
            }
            let d = n * 0x16;
            let frame = self.u16(0x508f80)?;
            self.paste(wx + 0x96 + d, wy + 0x96, 0x2b13)?;
            self.paste_player(wx + 0x96 + d, wy + 0x96, 0x2b13 + (frame & 7), q)?;
            n += 1;
        }
        Ok(())
    }
    /// The first zone under the mouse, using the market's ten buttons as the
    /// panel would set them (AffIconeSousCurseur 0x464f44).
    fn panel_zone(&self) -> Result<u32> {
        let (mx, my) = self.mouse()?;
        let (wx, wy) = (self.u16(WX)?, self.u16(WY)?);
        let mut live = [[0u32; 4]; 10];
        for (e, zone) in live.iter_mut().enumerate() {
            let entry = 0x496e90 + 24 * e as u32;
            let left = self.u16(entry + 4)?.wrapping_add(wx) & 0xffff;
            let top = self.u16(entry + 8)?.wrapping_add(wy) & 0xffff;
            let right = left.wrapping_add(self.u16(entry + 0xc)?) & 0xffff;
            let bottom = top.wrapping_add(self.u16(entry + 0x10)?) & 0xffff;
            *zone = [left, top, right, bottom];
        }
        let (first, last) = (self.u16(0x50a9b4)?, self.u16(0x50a9b6)?);
        let mut z = first;
        while z <= last {
            let r = if (0xc..0x16).contains(&z) {
                live[(z - 0xc) as usize]
            } else {
                let a = 0x5099b4 + 16 * z;
                [
                    self.u32(a)?,
                    self.u32(a + 4)?,
                    self.u32(a + 8)?,
                    self.u32(a + 12)?,
                ]
            };
            if r[0] as i32 <= mx && r[1] as i32 <= my && mx <= r[2] as i32 && my <= r[3] as i32 {
                return Ok(z);
            }
            z += 1;
        }
        Ok(0)
    }
    /// AffIconeSousCurseur 0x464f44.
    fn icon_under_cursor(&mut self) -> Result<()> {
        let zone = self.panel_zone()?.wrapping_sub(0xc) & 0xffff;
        if zone > 8 {
            return Ok(());
        }
        let (wx, wy) = self.window()?;
        let button = |c: &mut Self, xa: u32, ya: u32, id: u32| -> Result<()> {
            let x = wx + c.u16(xa)? as i32;
            let y = wy + c.u16(ya)? as i32;
            c.paste(x, y, id)
        };
        let message = match zone {
            0 | 7 => 0x70,
            1 | 8 => 0x71,
            2 => {
                button(self, 0x496ec4, 0x496ec8, 0x2afd)?;
                0x6c
            }
            3 => {
                if self.u8(0x496ed4)? == 0 {
                    return Ok(());
                }
                button(self, 0x496edc, 0x496ee0, 0x2afb)?;
                0x6a
            }
            4 => {
                if self.u8(0x496eec)? == 0 {
                    return Ok(());
                }
                button(self, 0x496ef4, 0x496ef8, 0x2afc)?;
                0x6b
            }
            5 => {
                if self.u8(0x496f04)? == 0 {
                    return Ok(());
                }
                button(self, 0x496f0c, 0x496f10, 0x2afb)?;
                0x6a
            }
            _ => {
                if self.u8(0x496f1c)? == 0 {
                    return Ok(());
                }
                button(self, 0x496f24, 0x496f28, 0x2afc)?;
                0x6b
            }
        };
        self.set_info(self.message_text(message)?, 0xffff)
    }
    /// AffMarcheProposition 0x465b8c.
    fn proposal(&mut self, p: u32) -> Result<()> {
        let h = self.side(p)?;
        let own = u32::from(self.u16(TAB + 2 * p)? == 1 && self.on_sale_count(p)? > 3);
        let (first, second) = if p == h {
            (0x496ed4, 0x496eec)
        } else {
            (0x496f04, 0x496f1c)
        };
        self.set_byte(first, own)?;
        self.set_byte(second, own)?;
        let partner = self.u8(PARTNER + p)?;
        let other =
            u32::from(self.u16(TAB + 2 * partner)? == 1 && self.on_sale_count(partner)? > 3);
        let (first, second) = if p != h {
            (0x496ed4, 0x496eec)
        } else {
            (0x496f04, 0x496f1c)
        };
        self.set_byte(first, other)?;
        self.set_byte(second, other)?;
        let (wx, wy) = self.window()?;
        self.paste(wx, wy, 0x2af8)?;
        self.persons(h, 0x31, false)?;
        self.goods(h, 0x31, false)?;
        let contact = self.u8(PARTNER + h)?;
        if contact != 0xff {
            self.persons(contact, 0xda, true)?;
            self.goods(contact, 0xda, true)?;
        }
        self.miniatures(h, 0x6f, GIVE_UNITS + 52 * h, GIVE_GOODS + 6 * h, false)?;
        self.miniatures(h, 0xb3, WANT_UNITS + 52 * h, WANT_GOODS + 6 * h, true)?;
        self.set_byte(SELECT_KIND, 0xff)?;
        self.set_byte(SELECT_ITEM, 0xff)?;
        self.alliance(p)?;
        if partner != 0xff {
            let (x, y) = (wx + 0x2c, wy + 0x96);
            self.paste(x, y, 0x2b13)?;
            let frame = self.u16(0x508f80)?;
            self.paste_player(x, y, 0x2b13 + (frame & 7), partner)?;
        }
        let s = self.side(h)?;
        let (gx, gy) = (self.u16(0x496f6c)? as i32, self.u16(0x496f70)? as i32);
        if self.u8(ACCEPTED + s)? != 0 {
            let id = 0x2b00 + self.u16(COUNT_A + 2 * s)?;
            self.paste(wx + gx, wy + gy, id & 0xffff)?;
            if self.u16(VALUE + 2 * h)? >= 0x64 {
                self.paste(wx + gx, wy + gy, 0x2b12)?;
            }
        } else {
            self.paste(wx + gx, wy + gy, 0x2b11)?;
        }
        if self.u8(ACCEPTED + h)? != 0 {
            let v = self.u16(VALUE + 2 * h)? as i32;
            let mut bar = (v.wrapping_mul(84) / 100) & 0xffff;
            if bar > 0x54 {
                bar = 0x54;
            }
            let color = if bar <= 0x1e {
                0x91
            } else if bar <= 0x41 {
                0xb4
            } else {
                0x30
            };
            self.fill(wx + 0xfd, wy + 0xc8 - bar, wx + 0x107, wy + 0xc8, color)?;
        }
        self.icon_under_cursor()
    }
    /// AffMarche 0x46677d for the local player's state.
    pub(crate) fn market(&mut self, p: u32) -> Result<()> {
        let margin = (self.u8(0x508f90)? as u8 as i8 as i32) << 5;
        match self.u8(0x4e2e7c + p)? {
            1 | 2 => {
                let h = self.u16(0x508726)? as i32;
                let frame = self.u16(0x508f7c)? & 0x1f;
                let y = ((h - 1) << 5) - 0xf;
                let (x, y) = if self.u16(0x49a018)? == 0x320 {
                    (margin + 0x2d, y - margin)
                } else {
                    (0x2d, y)
                };
                self.shade(x, y, 0x2f1d + frame, false, false)?;
                self.paste(x, y, 0x2efd + frame)
            }
            3 => self.proposal(p),
            4 => {
                let s = self.side(p)?;
                let (x, y) = (0x2f8 - margin, 0x1c2 - margin);
                self.put16("largura_tela_u16", 0x49a018, 0x320)?;
                self.shade(x, y, 0x1e6c, false, true)?;
                let id = 0x1e4b + self.u16(COUNT_B + 2 * s)?;
                self.bob(i16(x), i16(y), id & 0xffff, true, None)
            }
            _ => Ok(()),
        }
    }
}
