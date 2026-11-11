//! Autodesk FLC/FLI decoder, ported from `NativeFLC.swift`.
//!
//! Host presentation only, as the original's FlicOpen/FlicPlay layer: an 8-bit
//! frame buffer plus a 256-colour palette, advanced one frame chunk at a time.
//! Chunks: COLOR_256 (4), COLOR_64 (11), BLACK (13), BRUN (15), COPY (16),
//! LC (12, FLI delta) and SS2 (7, FLC word delta); others (PSTAMP, ...) skip.

use crate::platform::SURFACE_PIXELS;

#[derive(Clone, Debug)]
pub struct FlcError(pub String);

impl std::fmt::Display for FlcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for FlcError {}

fn failed(message: impl Into<String>) -> FlcError {
    FlcError(message.into())
}

#[derive(Clone, Debug)]
pub struct Flc {
    data: Vec<u8>,
    pub frames: usize,
    pub width: usize,
    pub height: usize,
    /// Milliseconds per frame (FLI speed is in 1/70 s and is converted).
    pub delay_ms: f64,
    pub pixels: Vec<u8>,
    pub palette: [u8; 768],
    pub palette_changed: bool,
    pub shown: usize,
    offset: usize,
    first_frame: usize,
}

impl Flc {
    pub fn new(bytes: Vec<u8>) -> Result<Self, FlcError> {
        if bytes.len() < 128 {
            return Err(failed("FLC header truncated"));
        }
        let u16_at = |offset: usize| -> usize {
            bytes[offset] as usize | (bytes[offset + 1] as usize) << 8
        };
        let u32_at = |offset: usize| -> usize { u16_at(offset) | u16_at(offset + 2) << 16 };
        let magic = u16_at(4);
        if magic != 0xAF12 && magic != 0xAF11 {
            return Err(failed(format!("not an FLC/FLI (magic {magic:04x})")));
        }
        let frames = u16_at(6);
        let width = u16_at(8);
        let height = u16_at(10);
        if frames == 0
            || width == 0
            || height == 0
            || width > 800
            || height > 600
            || (u16_at(12) != 8 && u16_at(12) != 0)
        {
            return Err(failed("unsupported FLC geometry"));
        }
        let speed = u32_at(16);
        let delay_ms = if magic == 0xAF11 {
            speed as f64 * 1000.0 / 70.0
        } else {
            speed as f64
        };
        let frame1 = if magic == 0xAF12 { u32_at(80) } else { 0 };
        let first_frame = if frame1 >= 128 && frame1 < bytes.len() {
            frame1
        } else {
            128
        };
        Ok(Self {
            data: bytes,
            frames,
            width,
            height,
            delay_ms,
            pixels: vec![0; width * height],
            palette: [0; 768],
            palette_changed: false,
            shown: 0,
            offset: first_frame,
            first_frame,
        })
    }

    pub fn from_path(path: &std::path::Path) -> Result<Self, FlcError> {
        Self::new(std::fs::read(path).map_err(|error| failed(error.to_string()))?)
    }

    /// Decodes the next frame chunk (prefix chunks `0xF100` are skipped).
    /// After `frames` chunks, a file's ring frame (if present) returns to frame 1.
    pub fn next_frame(&mut self) -> Result<(), FlcError> {
        self.palette_changed = false;
        loop {
            if self.offset + 16 > self.data.len() {
                return Err(failed(format!(
                    "frame {} beyond end of file",
                    self.shown + 1
                )));
            }
            let size = self.u32(self.offset);
            let kind = self.u16(self.offset + 4);
            if size < 16 || self.offset + size > self.data.len() {
                return Err(failed(format!(
                    "frame {} size {size} invalid",
                    self.shown + 1
                )));
            }
            if kind == 0xF100 {
                self.offset += size;
                continue;
            }
            if kind != 0xF1FA {
                return Err(failed(format!(
                    "frame {} chunk type {kind:04x}",
                    self.shown + 1
                )));
            }
            let count = self.u16(self.offset + 6);
            let frame_end = self.offset + size;
            let mut chunk = self.offset + 16;
            for _ in 0..count {
                if chunk + 6 > frame_end {
                    return Err(failed(format!(
                        "frame {} chunk header beyond frame",
                        self.shown + 1
                    )));
                }
                let mut length = self.u32(chunk);
                let chunk_kind = self.u16(chunk + 4);
                // Some writers store an odd size and pad; never step past the frame.
                if length < 6 || chunk + length > frame_end {
                    if length < 6 || chunk + length - 1 > frame_end {
                        return Err(failed(format!(
                            "frame {} chunk {chunk_kind} size {length}",
                            self.shown + 1
                        )));
                    }
                    length = frame_end - chunk;
                }
                // The original event files' COPY chunks declare 2 bytes less than
                // their 6 + width x height (the frame size is right): read to the frame.
                if chunk_kind == 16 && length < 6 + self.width * self.height {
                    length = (6 + self.width * self.height).min(frame_end - chunk);
                }
                let start = chunk + 6;
                let end = chunk + length;
                self.decode(chunk_kind, start, end)?;
                chunk += length;
            }
            self.offset = frame_end;
            self.shown += 1;
            return Ok(());
        }
    }

    /// Back to frame 1's position; the buffer keeps the last image, as a loop's
    /// ring frame expects.
    pub fn rewind(&mut self) {
        self.offset = self.first_frame;
        self.shown = 0;
    }

    fn u16(&self, offset: usize) -> usize {
        self.data[offset] as usize | (self.data[offset + 1] as usize) << 8
    }

    fn u32(&self, offset: usize) -> usize {
        self.u16(offset) | self.u16(offset + 2) << 16
    }

    fn decode(&mut self, kind: usize, start: usize, end: usize) -> Result<(), FlcError> {
        let shown = self.shown + 1;
        let fail = |what: &str| failed(format!("frame {shown} chunk {kind}: {what}"));
        let mut position = start;
        macro_rules! byte {
            () => {{
                if position >= end {
                    return Err(fail("truncated"));
                }
                let value = self.data[position] as usize;
                position += 1;
                value
            }};
        }
        macro_rules! word {
            () => {{
                let low = byte!();
                low | (byte!() << 8)
            }};
        }
        match kind {
            4 | 11 => {
                let packets = word!();
                let mut index = 0usize;
                for _ in 0..packets {
                    index += byte!();
                    let mut count = byte!();
                    if count == 0 {
                        count = 256;
                    }
                    for _ in 0..count {
                        if index >= 256 {
                            return Err(fail("palette overflow"));
                        }
                        for channel in 0..3 {
                            let value = byte!();
                            self.palette[index * 3 + channel] = if kind == 4 {
                                value as u8
                            } else {
                                let v = (value & 63) as u8;
                                v.wrapping_shl(2) | (v >> 4)
                            };
                        }
                        index += 1;
                    }
                }
                self.palette_changed = true;
            }
            13 => self.pixels.fill(0),
            16 => {
                if end.saturating_sub(start) < self.width * self.height {
                    return Err(fail("COPY truncated"));
                }
                self.pixels[..self.width * self.height]
                    .copy_from_slice(&self.data[start..start + self.width * self.height]);
            }
            15 => {
                for y in 0..self.height {
                    let _ = byte!(); // packet count: the line is filled by width
                    let mut x = 0usize;
                    while x < self.width {
                        if position >= end {
                            return Err(fail("truncated"));
                        }
                        let count = self.data[position] as i8;
                        position += 1;
                        if count >= 0 {
                            let value = byte!() as u8;
                            let count = count as usize;
                            if x + count > self.width {
                                return Err(fail("BRUN run past line"));
                            }
                            for i in 0..count {
                                self.pixels[y * self.width + x + i] = value;
                            }
                            x += count;
                        } else {
                            let count = (-(count as i32)) as usize;
                            if x + count > self.width || position + count > end {
                                return Err(fail("BRUN copy past line"));
                            }
                            for i in 0..count {
                                self.pixels[y * self.width + x + i] = self.data[position + i];
                            }
                            position += count;
                            x += count;
                        }
                        if count == 0 {
                            break;
                        }
                    }
                }
            }
            12 => {
                let mut y = word!();
                let lines = word!();
                for _ in 0..lines {
                    if y >= self.height {
                        return Err(fail("LC line past image"));
                    }
                    let mut x = 0usize;
                    let packets = byte!();
                    for _ in 0..packets {
                        x += byte!();
                        if position >= end {
                            return Err(fail("truncated"));
                        }
                        let count = self.data[position] as i8;
                        position += 1;
                        if count >= 0 {
                            let count = count as usize;
                            if x + count > self.width || position + count > end {
                                return Err(fail("LC copy past line"));
                            }
                            for i in 0..count {
                                self.pixels[y * self.width + x + i] = self.data[position + i];
                            }
                            position += count;
                            x += count;
                        } else {
                            let value = byte!() as u8;
                            let count = (-(count as i32)) as usize;
                            if x + count > self.width {
                                return Err(fail("LC run past line"));
                            }
                            for i in 0..count {
                                self.pixels[y * self.width + x + i] = value;
                            }
                            x += count;
                        }
                    }
                    y += 1;
                }
            }
            7 => {
                let lines = word!();
                let mut y = 0usize;
                for _ in 0..lines {
                    let packets;
                    loop {
                        let op = word!();
                        match op >> 14 {
                            0 => {
                                packets = op;
                            }
                            3 => {
                                y += 0x10000 - op; // signed negative line skip
                                continue;
                            }
                            2 => {
                                if y >= self.height {
                                    return Err(fail("SS2 last byte past image"));
                                }
                                self.pixels[y * self.width + self.width - 1] = (op & 0xff) as u8;
                                continue;
                            }
                            _ => return Err(fail("SS2 opcode")),
                        }
                        break;
                    }
                    if y >= self.height {
                        return Err(fail("SS2 line past image"));
                    }
                    let mut x = 0usize;
                    for _ in 0..packets {
                        x += byte!();
                        if position >= end {
                            return Err(fail("truncated"));
                        }
                        let count = self.data[position] as i8;
                        position += 1;
                        if count >= 0 {
                            let count = count as usize;
                            if x + 2 * count > self.width || position + 2 * count > end {
                                return Err(fail("SS2 copy past line"));
                            }
                            for i in 0..2 * count {
                                self.pixels[y * self.width + x + i] = self.data[position + i];
                            }
                            position += 2 * count;
                            x += 2 * count;
                        } else {
                            let low = byte!() as u8;
                            let high = byte!() as u8;
                            let count = (-(count as i32)) as usize;
                            if x + 2 * count > self.width {
                                return Err(fail("SS2 run past line"));
                            }
                            for i in 0..count {
                                self.pixels[y * self.width + x + 2 * i] = low;
                                self.pixels[y * self.width + x + 2 * i + 1] = high;
                            }
                            x += 2 * count;
                        }
                    }
                    y += 1;
                }
            }
            _ => {} // PSTAMP and other chunks carry no image data for playback.
        }
        Ok(())
    }

    /// Overlays the current frame centred on an 800x600 indexed game surface.
    pub fn composite(&self, surface: &mut [u8]) {
        let left = (800 - self.width) / 2;
        let top = (600 - self.height) / 2;
        for y in 0..self.height {
            let row = (top + y) * 800 + left;
            surface[row..row + self.width]
                .copy_from_slice(&self.pixels[y * self.width..(y + 1) * self.width]);
        }
    }

    /// Overlays the current frame at the exact rectangle used by the credits port.
    pub fn blit(&self, surface: &mut [u8], x: usize, y: usize) {
        for row in 0..self.height {
            let destination = (y + row) * 800 + x;
            surface[destination..destination + self.width]
                .copy_from_slice(&self.pixels[row * self.width..(row + 1) * self.width]);
        }
    }

    pub fn surface_size(&self) -> usize {
        SURFACE_PIXELS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le16(value: u16) -> [u8; 2] {
        value.to_le_bytes()
    }

    fn le32(value: u32) -> [u8; 4] {
        value.to_le_bytes()
    }

    #[test]
    fn decodes_a_color_and_copy_frame() {
        let width = 4u16;
        let height = 4u16;
        let mut palette = Vec::new();
        palette.extend(le32(6 + 2 + 48));
        palette.extend(le16(4));
        palette.extend(le16(1));
        palette.extend([0u8, 16]); // skip 0, 16 entries
        for index in 0..16u8 {
            palette.extend([index * 16, index * 8, index * 4]);
        }
        let mut copy = Vec::new();
        copy.extend(le32(6 + 16));
        copy.extend(le16(16));
        copy.extend([7u8; 16]);
        let chunks: Vec<u8> = palette.into_iter().chain(copy).collect();
        let mut frame = Vec::new();
        frame.extend(le32((16 + chunks.len()) as u32));
        frame.extend(le16(0xF1FA));
        frame.extend(le16(2));
        frame.extend([0u8; 8]);
        frame.extend(chunks);
        let mut header = vec![0u8; 128];
        header[4..6].copy_from_slice(&le16(0xAF12));
        header[6..8].copy_from_slice(&le16(1));
        header[8..10].copy_from_slice(&le16(width));
        header[10..12].copy_from_slice(&le16(height));
        header[12..14].copy_from_slice(&le16(8));
        header[16..20].copy_from_slice(&le32(71));
        header[80..84].copy_from_slice(&le32(128));
        let mut bytes = header;
        bytes.extend(frame);
        let mut flc = Flc::new(bytes).unwrap();
        assert_eq!((flc.frames, flc.width, flc.height), (1, 4, 4));
        flc.next_frame().unwrap();
        assert!(flc.palette_changed);
        assert_eq!(flc.palette[3], 16);
        assert_eq!(flc.pixels, vec![7u8; 16]);
        let mut surface = vec![0u8; SURFACE_PIXELS];
        flc.composite(&mut surface);
        assert_eq!(surface[300 * 800 + 400], 7);
    }
}
