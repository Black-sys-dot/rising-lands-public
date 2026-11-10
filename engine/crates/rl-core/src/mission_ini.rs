//! Byte-preserving Win32 profile input used by the original startup loaders.
//! Text remains in the original CP1252 encoding; section/key matching is ASCII-insensitive.
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct MissionIni {
    values: BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>>,
}
fn trim(b: &[u8]) -> &[u8] {
    let a = b
        .iter()
        .position(|c| !c.is_ascii_whitespace())
        .unwrap_or(b.len());
    let z = b
        .iter()
        .rposition(|c| !c.is_ascii_whitespace())
        .map_or(a, |i| i + 1);
    &b[a..z]
}
fn lower(b: &[u8]) -> Vec<u8> {
    b.iter().map(u8::to_ascii_lowercase).collect()
}
impl MissionIni {
    pub fn parse(bytes: &[u8]) -> Self {
        let mut result = Self::default();
        let mut section = Vec::new();
        for line in bytes.split(|c| *c == b'\n') {
            let line = trim(line);
            if line.is_empty() || matches!(line[0], b';' | b'#' | 0x1a) {
                continue;
            }
            if line[0] == b'[' {
                if let Some(end) = line.iter().position(|c| *c == b']') {
                    section = lower(trim(&line[1..end]));
                }
            } else if let Some(eq) = line.iter().position(|c| *c == b'=') {
                result.values.insert(
                    (section.clone(), lower(trim(&line[..eq]))),
                    trim(&line[eq + 1..]).to_vec(),
                );
            }
        }
        result
    }
    pub fn value(&self, section: &[u8], key: &[u8]) -> Option<&[u8]> {
        self.values
            .get(&(lower(section), lower(key)))
            .map(Vec::as_slice)
    }
    pub fn integer(&self, section: &[u8], key: &[u8], default: u32) -> u32 {
        let Some(raw) = self.value(section, key) else {
            return default;
        };
        let raw = trim(raw);
        let (negative, raw) = match raw.first() {
            Some(b'-') => (true, &raw[1..]),
            Some(b'+') => (false, &raw[1..]),
            _ => (false, raw),
        };
        let (radix, raw) = if raw.starts_with(b"0x") || raw.starts_with(b"0X") {
            (16, &raw[2..])
        } else {
            (10, raw)
        };
        let mut value = 0u32;
        for &c in raw {
            let digit = match c {
                b'0'..=b'9' => u32::from(c - b'0'),
                b'a'..=b'f' => u32::from(c - b'a') + 10,
                b'A'..=b'F' => u32::from(c - b'A') + 10,
                _ => break,
            };
            if digit >= radix {
                break;
            }
            value = value.wrapping_mul(radix).wrapping_add(digit);
        }
        if negative {
            value.wrapping_neg()
        } else {
            value
        }
    }
    pub fn string(&self, section: &[u8], key: &[u8], default: &[u8], capacity: usize) -> Vec<u8> {
        let mut raw = self.value(section, key).unwrap_or(default);
        if raw.len() >= 2 && matches!(raw[0], b'\'' | b'"') && raw.last() == raw.first() {
            raw = &raw[1..raw.len() - 1];
        }
        raw[..raw.len().min(capacity.saturating_sub(1))].to_vec()
    }
}
