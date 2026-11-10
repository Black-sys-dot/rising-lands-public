//! ObjectifLoadText 0x47c4b7. Original CP1252 bytes remain uninterpreted.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

pub const BUFFER: u32 = 0x50d9a4;
pub const LINE_COUNT: u32 = 0x511824;
pub const LINES: u32 = 0x511828;
pub const PAGE: u32 = 0x511ff8;
pub const DIRTY: u32 = 0x511ffa;

/// The original file operations, in order. The first open is an existence probe;
/// the original does not close it before its second open. Hosts own that handle.
/// read() returns at most the requested bytes and must report I/O failures.
pub trait Files {
    fn open(&mut self, path: &[u8], mode: &[u8]) -> Result<bool>;
    fn read(&mut self, maximum: usize) -> Result<Vec<u8>>;
    fn close(&mut self) -> Result<()>;
    /// FileSeek 0x45848b on the open file; only the save-slot scan rewinds a
    /// foreign header. Hosts that never reach it keep this explicit stop.
    fn seek(&mut self, _offset: u32, _origin: u8) -> Result<()> {
        stop("FileSeek host boundary", 0x45848b)
    }
}

pub(crate) fn cstring(m: &MemoriaJogo, ptr: u32, maximum: u32) -> Result<Vec<u8>> {
    let mut text = Vec::new();
    for i in 0..maximum {
        let byte = m.u8(ptr.wrapping_add(i))?;
        if byte == 0 {
            return Ok(text);
        }
        text.push(byte);
    }
    stop("shell unterminated original string", ptr)
}

pub fn load(m: &mut MemoriaJogo, mission: u8, files: &mut impl Files) -> Result<u32> {
    let mut relative = cstring(m, 0x498430, 19)?;
    relative[12] = b'0'.wrapping_add(mission / 10);
    relative[13] = b'0'.wrapping_add(mission % 10);
    let mut path = cstring(m, 0x509594, 256)?;
    path.extend_from_slice(&relative);
    if !files.open(&path, &cstring(m, 0x494f00, 4)?)? {
        path = cstring(m, 0x509494, 256)?;
        path.extend_from_slice(&relative);
    }
    m.set_u16(PAGE, 0)?;
    m.write(BUFFER, &[0; 16000])?;
    if !files.open(&path, &cstring(m, 0x494f03, 4)?)? {
        return Ok(0);
    }
    let data = files.read(16000)?;
    if data.len() > 16000 {
        return stop("ObjectifLoadText oversized FileRead result", 0x47c5e4);
    }
    m.write(BUFFER, &data)?;
    files.close()?;
    m.set_u16(LINE_COUNT, 0)?;
    m.set_u8(DIRTY, 1)?;
    m.set_u32(LINES, BUFFER)?;
    for i in 0..16000u32 {
        let byte = m.u8(BUFFER + i)?;
        if byte == 0 {
            break;
        }
        if byte == 13 {
            m.set_u8(BUFFER + i, 0)?;
        }
        if byte == 10 {
            let count = m.u16(LINE_COUNT)?.wrapping_add(1);
            m.set_u8(BUFFER + i, 0)?;
            m.set_u16(LINE_COUNT, count)?;
            m.set_u32(LINES + u32::from(count) * 4, BUFFER + i + 1)?;
        }
    }
    while !m.u16(LINE_COUNT)?.is_multiple_of(18) {
        let count = m.u16(LINE_COUNT)?.wrapping_add(1);
        m.set_u16(LINE_COUNT, count)?;
        m.set_u32(LINES + u32::from(count) * 4, 0x494f06)?;
    }
    Ok(u32::from(m.u16(LINE_COUNT)?) / 18)
}

pub fn load_registers(
    m: &mut MemoriaJogo,
    mut r: FrameRegisters,
    files: &mut impl Files,
) -> Result<FrameRegisters> {
    r.eax = load(m, r.eax as u8, files)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
