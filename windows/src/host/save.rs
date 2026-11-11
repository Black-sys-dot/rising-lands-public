//! Durable host saves.
//!
//! The engine serializes a complete slot through `rl_salvar_bytes` /
//! `rl_autosalvar_bytes`; the host writes those exact bytes with the same
//! temp-file + sync + rename policy as the engine file helper. That helper is
//! the one fixed for Windows in `rl_core::save::write_file` (a directory is
//! opened with `FILE_FLAG_BACKUP_SEMANTICS`), so there is a single durable
//! implementation in the repository.

use std::io;
use std::path::Path;

/// Atomically replaces `path`; a failure keeps the previous file.
pub fn write_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    rl_core::save::write_file(path, bytes).map_err(|error| io::Error::other(error.to_string()))
}

/// Reads a slot file for diagnostics and tests.
pub fn read_slot(path: &Path) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rising-windows-save-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn writes_and_replaces_a_slot() {
        let path = scratch("replace.rls");
        let _ = std::fs::remove_file(&path);
        write_durable(&path, b"first").unwrap();
        assert_eq!(read_slot(&path).unwrap(), b"first");
        write_durable(&path, b"second").unwrap();
        assert_eq!(read_slot(&path).unwrap(), b"second");
        let _ = std::fs::remove_file(&path);
    }
}
