//! ProgInit's EVENTS filename producer, intact474d53..474d9e.
use crate::{EngineError, MemoriaJogo, Result};

fn unsupported(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}

/// Explicit GetCurrentDirectory input for the headless Windows host scenario.
/// Leave both native 256-byte fields bounded; never truncate a directory.
pub fn bind_directory(m: &mut MemoriaJogo, directory: &[u8]) -> Result<()> {
    if directory.is_empty() || directory.len() > 244 || directory.contains(&0) {
        return Err(unsupported("EVENTS host install-directory input", 0x474c24));
    }
    m.write(0x509494, directory)?;
    m.set_u8(0x509494 + directory.len() as u32, 0)
}

/// Copy the declared install directory, then append the immutable original suffix.
/// There are no file/profile callbacks or RNG effects in this inline block.
pub fn events_filename(m: &mut MemoriaJogo) -> Result<()> {
    let mut directory = Vec::new();
    for i in 0..255 {
        let b = m.u8(0x509494 + i)?;
        if b == 0 {
            break;
        }
        directory.push(b);
    }
    let suffix = m.read(0x494d7c, 12)?.to_vec();
    if suffix != b"\\EVENTS.INI\0" || directory.len() > 244 {
        return Err(unsupported(
            "ProgInit EVENTS filename input domain",
            0x474d53,
        ));
    }
    // Preserve the original copy-terminator then append/terminator store order.
    for (i, b) in directory.iter().chain(std::iter::once(&0)).enumerate() {
        m.set_u8(0x509094 + i as u32, *b)?;
    }
    for (i, b) in suffix.iter().enumerate() {
        m.set_u8(0x509094 + directory.len() as u32 + i as u32, *b)?;
    }
    Ok(())
}
