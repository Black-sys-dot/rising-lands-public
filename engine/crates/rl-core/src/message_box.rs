//! PlayerResetAllie0x46d8cb / MessageBoxSet0x46994b, original EVENTS.INI bytes.
use crate::{frame::stop, MemoriaJogo, Result, World};
/// Owned input projection, separate from every original game allocation.
pub const EVENTS: u32 = 0x06200000;
pub fn install_events(m: &mut MemoriaJogo, file: &[u8]) -> Result<()> {
    if file.is_empty() || file.len() > 1024 * 1024 || file.contains(&0) {
        return stop("MessageBoxSet unsupported EVENTS.INI input", 0x46994b);
    }
    let mut bytes = (file.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(file);
    m.declare(EVENTS, bytes)
}
impl World {
    /// Supply bundled INSTALHD/EVENTS.INI before the first notification.
    pub fn load_events(&mut self, file: &[u8]) -> Result<()> {
        let mut memory = self.project()?;
        install_events(&mut memory, file)?;
        self.replace_memory(memory)
    }
}
fn profile(m: &MemoriaJogo, section: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    let mut filename = Vec::new();
    for i in 0..512 {
        let b = m.u8(0x509094 + i)?;
        if b == 0 {
            break;
        }
        filename.push(b);
    }
    // ProgInit474d53 is the only writer of 0x509094 in the intact image. An empty
    // field means the world was entered with EVENTS bytes but no logical filename
    // (legacy rl_carregar_eventos); the host must bind the EVENTS host instead.
    if filename.is_empty() {
        return stop(
            "MessageBoxSet EVENTS filename 0x509094 not bound (bytes-only EVENTS entry; bind the EVENTS host)",
            0x469a68,
        );
    }
    if filename.len() == 512
        || !filename
            .rsplit(|&b| b == b'\\' || b == b'/')
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case(b"EVENTS.INI"))
    {
        return stop("MessageBoxSet unsupported profile path", 0x469a68);
    }
    if !m.blocks().any(|(a, _)| a == EVENTS) {
        return stop("MessageBoxSet EVENTS.INI input not supplied", 0x469a68);
    }
    let file = m.read(EVENTS + 4, m.u32(EVENTS)? as usize)?;
    let mut active = false;
    for raw in file.split(|&b| b == b'\n') {
        let line = raw.trim_ascii();
        if line.starts_with(b"[") {
            active = line
                .strip_prefix(b"[")
                .and_then(|s| s.strip_suffix(b"]"))
                .is_some_and(|s| s.eq_ignore_ascii_case(section));
        } else if active && !line.starts_with(b";") {
            if let Some(pos) = line.iter().position(|&b| b == b'=') {
                if line[..pos].trim_ascii().eq_ignore_ascii_case(key) {
                    let mut value = line[pos + 1..].trim_ascii();
                    if value.len() >= 2
                        && matches!(value[0], b'\'' | b'"')
                        && value[0] == value[value.len() - 1]
                    {
                        value = &value[1..value.len() - 1];
                    }
                    return Ok(value[..value.len().min(249)].to_vec());
                }
            }
        }
    }
    // The two intact GetPrivateProfileStringA calls explicitly pass "" as
    // their missing-key default. Missing input files above remain addressed.
    Ok(Vec::new())
}
pub(crate) fn set(m: &mut MemoriaJogo, kind: u8, code: u16) -> Result<()> {
    if m.u8(0x507d58)? != 0 {
        return Ok(());
    }
    let tail = m.u16(0x507d56)?;
    if tail >= 256 {
        return stop("MessageBoxSet queue index", 0x469965);
    }
    let record = 0x4e7754 + 518 * u32::from(tail);
    m.set_u8(record, kind)?;
    m.set_u16(record + 2, code)?;
    let key = if code < 1000 {
        format!("{code:03}")
    } else {
        format!("{:04}", code % 10000)
    };
    for (section, offset) in [(b"EVENEMENT".as_slice(), 0x106), (b"TEXTE".as_slice(), 4)] {
        let value = profile(m, section, key.as_bytes())?;
        m.write(record + offset, &value)?;
        m.set_u8(record + offset + value.len() as u32, 0)?;
    }
    let mut star = None;
    for i in 0..250 {
        let c = m.u8(record + 4 + i)?;
        if c == 0 {
            break;
        }
        if c == b'*' {
            star = Some(i);
            break;
        }
    }
    if let Some(i) = star {
        m.set_u16(record + 0x102, i as u16 + 1)?;
        m.set_u8(record + 4 + i, 0)?;
    } else {
        m.set_u16(record + 0x102, 65535)?;
    }
    m.set_u16(record + 0x104, 400)?;
    let next = (tail + 1) % 256;
    m.set_u16(0x507d56, next)?;
    if next == m.u16(0x507d54)? {
        m.set_u8(0x507d58, 1)?;
    }
    Ok(())
}
pub(crate) fn reset_allie(m: &mut MemoriaJogo, owner: u8, other: u8) -> Result<()> {
    if owner == other || owner > 4 || other > 4 {
        return Ok(());
    }
    let a = m.u32(0x5080a8)?.wrapping_add(20 * u32::from(owner) + 6);
    m.set_u8(a, m.u8(a)? & !(1 << other))?;
    if owner == m.u8(0x508041)? {
        set(m, 10, u16::from(other) + 402)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_only_events_entry_names_the_missing_filename_binding() {
        let mut m = MemoriaJogo::from_blocks([(0x509094, vec![0; 512])]).unwrap();
        install_events(&mut m, b"[TEXTE]\n406=Original\n").unwrap();
        let error = profile(&m, b"TEXTE", b"406").unwrap_err().to_string();
        assert!(
            error.contains("EVENTS filename 0x509094 not bound"),
            "{error}"
        );
        assert!(
            error.contains("0x00469a68") || error.contains("469a68"),
            "{error}"
        );
        m.write(0x509094, b"C:\\RISING\\OTHER.INI\0").unwrap();
        let error = profile(&m, b"TEXTE", b"406").unwrap_err().to_string();
        assert!(error.contains("unsupported profile path"), "{error}");
    }

    #[test]
    fn profile_input_is_explicit_and_cannot_replace_an_installed_resource() {
        let mut m =
            MemoriaJogo::from_blocks([(0x509094, b"C:\\RISING\\EVENTS.INI\0".to_vec())]).unwrap();
        assert!(profile(&m, b"TEXTE", b"406").is_err());
        let before = m.clone();
        for invalid in [b"".as_slice(), b"[TEXTE]\n406=bad\0"] {
            assert!(install_events(&mut m, invalid).is_err());
            assert_eq!(m, before);
        }
        install_events(&mut m, b"[TEXTE]\n406=Original\n").unwrap();
        let loaded = m.clone();
        assert!(install_events(&mut m, b"[TEXTE]\n406=Replacement\n").is_err());
        assert_eq!(m, loaded);
        assert_eq!(profile(&m, b"TEXTE", b"406").unwrap(), b"Original");
        assert!(profile(&m, b"TEXTE", b"999").unwrap().is_empty());
    }
}
