//! Explicit host namespace/resource binding; no host filesystem access here.
use crate::{chain_event_path, message_box, EngineError, MemoriaJogo, Result};

pub struct Input<'a> {
    pub directory: &'a [u8],
    pub file: &'a [u8],
}
fn refused(name: &'static str) -> EngineError {
    EngineError::UnportedCallee {
        name,
        address: 0x469a68,
    }
}
impl Input<'_> {
    /// This narrow host namespace is a contract, not a historical game rule.
    /// Absolute drive paths only, no aliases, separator ambiguity or dot segments.
    pub fn validate(&self) -> Result<()> {
        let d = self.directory;
        if !(4..=244).contains(&d.len())
            || !d[0].is_ascii_alphabetic()
            || &d[1..3] != b":\\"
            || d[3..].split(|&c| c == b'\\').any(|s| {
                s.is_empty()
                    || s == b"."
                    || s == b".."
                    || s.ends_with(b" ")
                    || s.ends_with(b".")
                    || s.iter()
                        .any(|&c| !c.is_ascii_alphanumeric() && !b" _-().".contains(&c))
            })
        {
            return Err(refused("EVENTS host logical directory domain"));
        }
        if self.file.is_empty() || self.file.len() > 1024 * 1024 || self.file.contains(&0) {
            return Err(refused("EVENTS host file bounds"));
        }
        Ok(())
    }
    /// Called in ProgInit after its permutation init, before types/MissionLoad. All writes
    /// are private until mission entry publishes its new handle.
    pub fn initialize(&self, m: &mut MemoriaJogo) -> Result<()> {
        self.validate()?;
        if m.u8(0x509094)? != 0 || m.blocks().any(|(p, _)| p == message_box::EVENTS) {
            return Err(refused(
                "EVENTS fresh initialization requires unbound startup",
            ));
        }
        chain_event_path::bind_directory(m, self.directory)?;
        chain_event_path::events_filename(m)?;
        message_box::install_events(m, self.file)
    }
    /// Reassociate external ownership after restore. Never repair or normalize
    /// saved bytes, rerun startup, change clock/generation, or open a saved path.
    pub fn verify(&self, m: &MemoriaJogo) -> Result<()> {
        self.validate()?;
        let mut directory = self.directory.to_vec();
        directory.push(0);
        let mut filename = self.directory.to_vec();
        filename.extend_from_slice(b"\\EVENTS.INI\0");
        if m.read(0x509494, directory.len())? != directory
            || m.read(0x509094, filename.len())? != filename
        {
            return Err(refused(
                "EVENTS saved logical filename mismatch or legacy unbound save",
            ));
        }
        let Some((_, stored)) = m.blocks().find(|(p, _)| *p == message_box::EVENTS) else {
            return Err(refused("EVENTS saved owned input missing"));
        };
        if stored.len() != self.file.len() + 4
            || stored[..4] != (self.file.len() as u32).to_le_bytes()
            || &stored[4..] != self.file
        {
            return Err(refused(
                "EVENTS saved input differs from declared host file",
            ));
        }
        Ok(())
    }
}
