//! InitTextLangue0x479383 / language-0 parser0x478ea8 and TextGetMessage0x478a3b.
//! File bytes are the game's PT-BR TEXTES.TXT, never decoded or translated.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result, World};

pub const TABLE: u32 = 0x5099a4;
pub const HEAP: u32 = 0x06000000;

/// Fixed-address owned allocation projection. The platform allocator's addresses
/// and zeroed backing storage are explicit inputs to the native differential.
/// Startup is atomic; unsupported files retain a named stop instead of a blank HUD.
pub fn init_langue(m: &mut MemoriaJogo, file: &[u8], language: u8) -> Result<()> {
    if language != 0 {
        return stop(
            "InitTextLangue: only the bundled PT-BR FRANCAIS section is ported",
            0x478ea8,
        );
    }
    if m.u32(TABLE)? != 0 {
        return stop("InitTextLangue: text table already loaded", 0x479383);
    }
    let capacity = u32::from(m.u16(0x4973ec)?);
    if capacity == 0 {
        return stop("InitTextLangue: empty allocation capacity", 0x479394);
    }
    let mut next = m.clone();
    let strings = HEAP + 4 * capacity;
    next.declare(HEAP, vec![0; (4 * capacity + 256 * capacity) as usize])?;
    next.set_u32(TABLE, HEAP)?;
    for id in 0..capacity {
        next.set_u32(HEAP + 4 * id, strings + 256 * id)?;
        next.set_u8(strings + 256 * id, 0)?;
    }
    // Watcom fopen("rt") translates CRLF and stops at DOS EOF (0x1a).
    let end = file.iter().position(|b| *b == 0x1a).unwrap_or(file.len());
    let mut translated = Vec::with_capacity(end);
    for (i, &byte) in file[..end].iter().enumerate() {
        if byte != b'\r' || file.get(i + 1) != Some(&b'\n') {
            translated.push(byte);
        }
    }
    let mut at = 0;
    let mut found = false;
    let mut id = 0;
    while at < translated.len() {
        // Original fgets(buffer,256): at most255 bytes, through the first LF.
        let remaining = &translated[at..];
        let length = remaining
            .iter()
            .position(|b| *b == b'\n')
            .map_or(remaining.len(), |i| i + 1)
            .min(255);
        let line = &remaining[..length];
        at += length;
        if !found {
            if line.starts_with(b"[FRANCAIS") {
                found = true;
            }
            continue;
        }
        // Native0x479284 skips every slash-prefixed line, not just double slash.
        if line.first() == Some(&b'/') {
            continue;
        }
        if line.first() == Some(&b'[') {
            break;
        }
        let Some(length) = line.iter().position(|b| *b == b'\n') else {
            return stop(
                "InitTextLangue: unterminated/oversize native message line",
                0x47929e,
            );
        };
        if id >= capacity {
            return stop("InitTextLangue: native text allocation exhausted", 0x4792b8);
        }
        next.write(strings + 256 * id, &line[..length])?;
        next.set_u8(strings + 256 * id + length as u32, 0)?;
        id += 1;
    }
    if !found {
        return stop(
            "InitTextLangue: missing FRANCAIS section (native search cannot finish)",
            0x478ed9,
        );
    }
    next.set_u16(0x4973ec, id as u16)?;
    next.set_u32(0x4e2f3c, next.u32(0x4e2f3c)?.wrapping_add(260 * capacity))?;
    // InitTextFont0x45a4c1: bitmap language0, no external multibyte font.
    next.set_u32(0x4e1d94, u32::from(language))?;
    next.set_u32(0x4e1d98, 0)?;
    *m = next;
    Ok(())
}

pub fn get(m: &MemoriaJogo, id: u32) -> Result<u32> {
    let table = m.u32(TABLE)?;
    if table == 0 {
        return stop(
            "TextGetMessage with the text table pointer 0x5099a4 = 0 (texts not loaded)",
            0x478a47,
        );
    }
    m.u32(table.wrapping_add(4 * (id & 65535)))
}

/// Literal native ABI: only EAX changes. EBX/ECX/EDX and all saved GPRs survive.
pub fn get_registers(m: &MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    Ok(FrameRegisters {
        eax: get(m, r.eax)?,
        esp: r.esp.wrapping_add(4),
        ..r
    })
}

impl World {
    /// Apply the real text startup to an S0 world before its first input frame.
    /// Callers read INSTALHD/TEXTES.TXT from the bundled CD assets and supply bytes.
    pub fn load_texts(&mut self, file: &[u8], language: u8) -> Result<()> {
        let mut memory = self.project()?;
        init_langue(&mut memory, file, language)?;
        self.replace_memory(memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn startup() -> MemoriaJogo {
        let mut m = MemoriaJogo::from_blocks([(0x494000, vec![0; 0x170000])]).unwrap();
        m.set_u16(0x4973ec, 300).unwrap();
        m.set_u32(0x4e2f3c, 0xfffffff0).unwrap();
        m.set_u32(crate::WORLD_SEED, 0x12345678).unwrap();
        m
    }

    #[test]
    fn invalid_resource_and_repeated_initialization_leave_the_world_unchanged() {
        let mut m = startup();
        let before = m.clone();
        for (file, language) in [
            (b"[ENGLISH]\r\nText\r\n".as_slice(), 0),
            (b"[FRANCAIS]\r\nno newline".as_slice(), 0),
            (b"[FRANCAIS]\r\nText\r\n".as_slice(), 6),
        ] {
            assert!(init_langue(&mut m, file, language).is_err());
            assert_eq!(m, before);
        }
        init_langue(&mut m, b"[FRANCAIS]\r\nOl\xe1\r\n", 0).unwrap();
        assert_eq!(m.u32(crate::WORLD_SEED).unwrap(), 0x12345678);
        assert_eq!(m.u32(0x4e2f3c).unwrap(), 77984);
        let loaded = m.clone();
        assert!(init_langue(&mut m, b"[FRANCAIS]\n", 0).is_err());
        assert_eq!(m, loaded);
        assert_eq!(m.read(get(&m, 0).unwrap(), 4).unwrap(), b"Ol\xe1\0");
    }
}
