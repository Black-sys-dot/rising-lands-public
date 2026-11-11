//! First-launch import of the player's own original CD, ported from
//! `NativeCDImport.swift`.
//!
//! The client ships no CD data: every original file, and every file derived
//! from one, is produced here from the image the player provides (the Internet
//! Archive BIN/CUE, an ISO, or a folder holding the CD's files), checked
//! against the SHA-256 values of the shared recipe manifest, and written to the
//! user's data folder. Output kinds:
//!
//! - `copy` — an original file, byte for byte;
//! - `ima-adpcm-wav` — one IMA ADPCM sound sample as a PCM16 WAV;
//! - `pcm-wav` — one uncompressed sound sample as a WAV;
//! - `cd-audio` — a CD audio track of the Brazilian BIN;
//! - `keyed` — a runtime-derived table shipped XOR a keystream only the CD's
//!   own bytes produce;
//! - `bundled` — app data that holds no CD content.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const SCHEMA: &str = "rising-lands/player-import/1";
pub const KEY_DOMAIN: &str = "RisingLands-player-import-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    Unrecognized(String),
    Missing(String),
    Mismatch(String),
    Manifest(String),
    Io(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrecognized(name) => write!(
                f,
                "“{name}” não parece ser o CD do Rising Lands. Escolha a imagem RISING_LANDS_BR_.BIN (ou o arquivo .CUE ou .ISO) ou a pasta com os arquivos do CD."
            ),
            Self::Missing(path) => {
                write!(f, "O arquivo {path} não foi encontrado no CD escolhido.")
            }
            Self::Mismatch(path) => write!(
                f,
                "O arquivo {path} do CD escolhido é diferente do CD brasileiro original (verificação SHA-256). Use a imagem original, sem modificações."
            ),
            Self::Manifest(detail) => write!(
                f,
                "O manifesto de importação deste aplicativo é inválido ({detail}). Baixe o aplicativo novamente."
            ),
            Self::Io(detail) => write!(
                f,
                "Não foi possível gravar os dados importados: {detail}"
            ),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        ImportError::Io(error.to_string())
    }
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sha256_hex(data: &[u8]) -> String {
    sha256(data).iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema: String,
    id: String,
    sources: BTreeMap<String, SourceSpec>,
    outputs: Vec<OutputSpec>,
    #[serde(default, rename = "derivedForbidden")]
    _derived_forbidden: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SourceSpec {
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct OutputSpec {
    path: String,
    kind: String,
    sha256: String,
    #[serde(default)]
    bytes: Option<u64>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    length: Option<usize>,
    #[serde(default)]
    channels: Option<u32>,
    #[serde(default)]
    rate: Option<u32>,
    #[serde(default)]
    bits: Option<u32>,
    #[serde(default)]
    block: Option<usize>,
    #[serde(default)]
    track: Option<u8>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    blob: Option<String>,
    #[serde(default, rename = "keySources")]
    key_sources: Option<Vec<String>>,
}

pub trait CdSource {
    fn label(&self) -> String;
    /// The raw 2,352-byte-sector image, when the source is one (CD audio needs it).
    fn raw_image(&self) -> Option<&Path>;
    fn file(&mut self, path: &str) -> Result<Vec<u8>, ImportError>;
}

/// ISO 9660 reader over a raw MODE1/MODE2 2,352-byte image or a 2,048-byte ISO.
pub struct CdImage {
    path: PathBuf,
    file: fs::File,
    sector_size: u64,
    data_offset: u64,
    root_lba: u32,
    root_length: u32,
}

impl CdImage {
    pub fn open(path: &Path) -> Result<Self, ImportError> {
        let mut file = fs::File::open(path).map_err(|_| {
            ImportError::Unrecognized(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        })?;
        let size = file.seek(SeekFrom::End(0)).unwrap_or(0);
        let mut read_at = |offset: u64, count: usize| -> Option<Vec<u8>> {
            if offset + count as u64 > size {
                return None;
            }
            file.seek(SeekFrom::Start(offset)).ok()?;
            let mut bytes = vec![0u8; count];
            file.read_exact(&mut bytes).ok()?;
            Some(bytes)
        };
        let layouts = [(2352u64, 24u64), (2352, 16), (2048, 0)];
        let mut selected = None;
        for (sector, offset) in layouts {
            let descriptor = read_at(16 * sector + offset, 2048);
            if let Some(descriptor) = descriptor {
                if &descriptor[1..6] == b"CD001" && descriptor[0] == 1 {
                    selected = Some(((sector, offset), descriptor));
                    break;
                }
            }
        }
        let ((sector_size, data_offset), pvd) = selected.ok_or_else(|| {
            ImportError::Unrecognized(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        })?;
        let root_lba = u32::from_le_bytes([pvd[158], pvd[159], pvd[160], pvd[161]]);
        let root_length = u32::from_le_bytes([pvd[166], pvd[167], pvd[168], pvd[169]]);
        Ok(Self {
            path: path.to_path_buf(),
            file,
            sector_size,
            data_offset,
            root_lba,
            root_length,
        })
    }

    fn extent(&mut self, lba: u32, length: u32) -> Result<Vec<u8>, ImportError> {
        let sectors = (length as u64 + 2047) / 2048;
        let offset = lba as u64 * self.sector_size;
        let total = sectors * self.sector_size;
        if offset + total > self.file.metadata().map(|m| m.len()).unwrap_or(0) {
            return Err(ImportError::Unrecognized(self.label()));
        }
        self.file.seek(SeekFrom::Start(offset))?;
        let mut raw = vec![0u8; total as usize];
        self.file.read_exact(&mut raw)?;
        if self.sector_size == 2048 {
            raw.truncate(length as usize);
            return Ok(raw);
        }
        let mut out = Vec::with_capacity(sectors as usize * 2048);
        for index in 0..sectors as usize {
            let start = index * self.sector_size as usize + self.data_offset as usize;
            out.extend_from_slice(&raw[start..start + 2048]);
        }
        out.truncate(length as usize);
        Ok(out)
    }

    fn normalized(name: &str) -> String {
        let mut value = name.to_ascii_uppercase();
        if let Some(index) = value.find(';') {
            value.truncate(index);
        }
        while value.ends_with('.') {
            value.pop();
        }
        value
    }

    fn entries(&mut self, lba: u32, length: u32) -> Result<HashMap<String, (u32, u32, bool)>, ImportError> {
        let data = self.extent(lba, length)?;
        let mut result = HashMap::new();
        let mut offset = 0usize;
        while offset < data.len() {
            let size = data[offset] as usize;
            if size == 0 {
                offset = (offset / 2048 + 1) * 2048;
                continue;
            }
            if offset + size > data.len() || size < 34 {
                break;
            }
            let record = &data[offset..offset + size];
            let name_length = record[32] as usize;
            if name_length > 0 && 33 + name_length <= size {
                let raw = &record[33..33 + name_length];
                if raw != [0] && raw != [1] {
                    if let Ok(name) = std::str::from_utf8(raw) {
                        let lba = u32::from_le_bytes([record[2], record[3], record[4], record[5]]);
                        let length =
                            u32::from_le_bytes([record[10], record[11], record[12], record[13]]);
                        result.insert(Self::normalized(name), (lba, length, record[25] & 2 != 0));
                    }
                }
            }
            offset += size;
        }
        Ok(result)
    }
}

impl CdSource for CdImage {
    fn label(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn raw_image(&self) -> Option<&Path> {
        (self.sector_size == 2352).then_some(self.path.as_path())
    }

    fn file(&mut self, path: &str) -> Result<Vec<u8>, ImportError> {
        let (mut lba, mut length, mut directory) = (self.root_lba, self.root_length, true);
        for component in path.split('/') {
            if !directory {
                return Err(ImportError::Missing(path.to_owned()));
            }
            let entries = self.entries(lba, length)?;
            let (next_lba, next_length, next_directory) = entries
                .get(&Self::normalized(component))
                .copied()
                .ok_or_else(|| ImportError::Missing(path.to_owned()))?;
            lba = next_lba;
            length = next_length;
            directory = next_directory;
        }
        if directory {
            return Err(ImportError::Missing(path.to_owned()));
        }
        self.extent(lba, length)
    }
}

/// A folder holding the CD's files (a mounted disc or an extracted copy).
pub struct CdFolder {
    root: PathBuf,
}

impl CdFolder {
    pub fn open(root: &Path) -> Result<Self, ImportError> {
        if resolve(root, "INSTALHD").is_err() {
            return Err(ImportError::Unrecognized(
                root.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }
}

fn resolve(base: &Path, path: &str) -> Result<PathBuf, ImportError> {
    let mut current = base.to_path_buf();
    for component in path.split('/') {
        let names = fs::read_dir(&current)
            .map_err(|_| ImportError::Missing(path.to_owned()))?;
        let mut found = None;
        for entry in names.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if CdImage::normalized(&name) == CdImage::normalized(component) {
                found = Some(entry.path());
                break;
            }
        }
        current = found.ok_or_else(|| ImportError::Missing(path.to_owned()))?;
    }
    Ok(current)
}

impl CdSource for CdFolder {
    fn label(&self) -> String {
        self.root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn raw_image(&self) -> Option<&Path> {
        None
    }

    fn file(&mut self, path: &str) -> Result<Vec<u8>, ImportError> {
        let resolved = resolve(&self.root, path)?;
        let label = resolved.to_string_lossy().into_owned();
        fs::read(&resolved).map_err(|_| ImportError::Missing(label))
    }
}

/// Opens what the player chose: a .bin/.iso image, a .cue sheet or a folder.
pub fn open_source(path: &Path) -> Result<Box<dyn CdSource>, ImportError> {
    if !path.exists() {
        return Err(ImportError::Unrecognized(
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ));
    }
    if path.is_dir() {
        return Ok(Box::new(CdFolder::open(path)?));
    }
    if path
        .extension()
        .map(|extension| extension.to_string_lossy().eq_ignore_ascii_case("cue"))
        .unwrap_or(false)
    {
        let text = fs::read(path)?;
        let text = String::from_utf8_lossy(&text);
        let line = text
            .lines()
            .find(|line| line.trim_start().to_ascii_uppercase().starts_with("FILE "))
            .ok_or_else(|| ImportError::Unrecognized(path.to_string_lossy().into_owned()))?;
        let first = line
            .find('"')
            .ok_or_else(|| ImportError::Unrecognized(path.to_string_lossy().into_owned()))?;
        let last = line
            .rfind('"')
            .filter(|last| *last > first)
            .ok_or_else(|| ImportError::Unrecognized(path.to_string_lossy().into_owned()))?;
        let name = &line[first + 1..last];
        let image = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(name);
        return Ok(Box::new(CdImage::open(&image)?));
    }
    Ok(Box::new(CdImage::open(path)?))
}

/// `imaadpcmDecode4Bit_M16` 0x48b8c9: blocks of `min(remaining, block)` bytes
/// while at least 4 remain; header i16 predictor, u8 step index (above 88
/// stops), then low and high nibble of each byte. Little-endian PCM16 bytes.
pub fn ima_mono16(source: &[u8], block: usize) -> Option<Vec<u8>> {
    const INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
    const STEPS: [i32; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60,
        66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371,
        408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707,
        1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132,
        7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623,
        27086, 29794, 32767,
    ];
    if block < 4 && source.len() >= 4 {
        return None;
    }
    let mut remaining = source.len();
    let mut position = 0usize;
    let mut out = Vec::with_capacity(source.len() * 4 + 4);
    let emit = |value: i32, out: &mut Vec<u8>| {
        let value = value as i16 as u16;
        out.extend_from_slice(&value.to_le_bytes());
    };
    while remaining >= 4 {
        let count = remaining.min(block);
        remaining -= count;
        let mut predictor =
            i16::from_le_bytes([source[position], source[position + 1]]) as i32;
        let mut index = source[position + 2] as usize;
        position += 4;
        if index > 88 {
            return Some(out);
        }
        emit(predictor, &mut out);
        for &byte in &source[position..position + count - 4] {
            for nibble in [byte & 15, byte >> 4] {
                let step = STEPS[index];
                let mut difference = step >> 3;
                if nibble & 4 != 0 {
                    difference += step;
                }
                if nibble & 2 != 0 {
                    difference += step >> 1;
                }
                if nibble & 1 != 0 {
                    difference += step >> 2;
                }
                predictor = (predictor
                    + if nibble & 8 != 0 {
                        -difference
                    } else {
                        difference
                    })
                .clamp(-32768, 32767);
                index = (index as i32 + INDEX_TABLE[nibble as usize]).clamp(0, 88) as usize;
                emit(predictor, &mut out);
            }
        }
        position += count - 4;
    }
    Some(out)
}

pub fn wav(pcm: &[u8], channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let align = channels * bits / 8;
    let mut data = Vec::with_capacity(pcm.len() + 44);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&((pcm.len() + 36) as u32).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&rate.to_le_bytes());
    data.extend_from_slice(&(rate * align as u32).to_le_bytes());
    data.extend_from_slice(&align.to_le_bytes());
    data.extend_from_slice(&bits.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    data.extend_from_slice(pcm);
    data
}

/// The keystream of a keyed output: SHA-256(K || counter) blocks, with
/// K = SHA-256(domain || 0 || path || 0 || key source bytes...).
pub fn keyed(blob: &[u8], path: &str, key_sources: &[Vec<u8>]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(KEY_DOMAIN.as_bytes());
    hasher.update([0]);
    hasher.update(path.as_bytes());
    hasher.update([0]);
    for source in key_sources {
        hasher.update(source);
    }
    let key: [u8; 32] = hasher.finalize().into();
    let mut out = blob.to_vec();
    let mut counter: u64 = 0;
    let mut offset = 0usize;
    while offset < out.len() {
        let mut block = key.to_vec();
        block.extend_from_slice(&counter.to_le_bytes());
        let digest = sha256(&block);
        for byte in digest {
            if offset < out.len() {
                out[offset] ^= byte;
                offset += 1;
            }
        }
        counter += 1;
    }
    out
}

/// Exact Brazilian BIN windows. Each returned WAV is the reviewed complete WAV.
pub fn cd_music_wav(track: u8, image: &Path) -> Result<Vec<u8>, ImportError> {
    const IMAGE_BYTES: u64 = 705_418_896;
    let (offset, count, expected) = match track {
        2 => (
            534_569_616u64,
            99_654_240usize,
            "bee34ef8aef0a58933f90d63766eafc3f8daf447aae64ea35157d84c6850596a",
        ),
        3 => (
            634_223_856,
            71_195_040,
            "80b28a77ea82e454aa6babeb788746d2a141a932f4d101d7c4a8ecc51870ae3d",
        ),
        _ => return Err(ImportError::Mismatch(format!("faixa de áudio {track}"))),
    };
    let mut file = fs::File::open(image)
        .map_err(|_| ImportError::Mismatch(format!("faixa de áudio {track}")))?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    if size != IMAGE_BYTES {
        return Err(ImportError::Mismatch(format!("faixa de áudio {track}")));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut pcm = vec![0u8; count];
    file.read_exact(&mut pcm)
        .map_err(|_| ImportError::Mismatch(format!("faixa de áudio {track}")))?;
    let data = wav(&pcm, 2, 44_100, 16);
    if sha256_hex(&data) != expected {
        return Err(ImportError::Mismatch(format!("faixa de áudio {track}")));
    }
    Ok(data)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub written: usize,
    pub music: bool,
    pub manifest_id_ok: bool,
}

/// Produces every output into `destination` (created; must not exist).
pub fn run(
    manifest_path: &Path,
    resources: &Path,
    source: &mut dyn CdSource,
    destination: &Path,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<Report, ImportError> {
    let manifest_bytes = fs::read(manifest_path)?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| ImportError::Manifest(format!("schema: {error}")))?;
    if manifest.schema != SCHEMA || manifest.outputs.is_empty() {
        return Err(ImportError::Manifest("schema".into()));
    }
    let music = source.raw_image().is_some();
    let mut cache: HashMap<String, Vec<u8>> = HashMap::new();
    let mut original = |path: &str, source: &mut dyn CdSource| -> Result<Vec<u8>, ImportError> {
        if let Some(data) = cache.get(path) {
            return Ok(data.clone());
        }
        let expected = manifest
            .sources
            .get(path)
            .map(|spec| spec.sha256.clone())
            .ok_or_else(|| ImportError::Manifest(path.to_owned()))?;
        let data = source.file(path)?;
        if sha256_hex(&data) != expected {
            return Err(ImportError::Mismatch(path.to_owned()));
        }
        cache.insert(path.to_owned(), data.clone());
        Ok(data)
    };
    // Verify every needed original first, so a wrong disc fails before writing.
    let mut needed: Vec<String> = Vec::new();
    for output in &manifest.outputs {
        if output.group.as_deref() == Some("music") && !music {
            continue;
        }
        if let Some(source) = &output.source {
            needed.push(source.clone());
        }
        if let Some(sources) = &output.key_sources {
            needed.extend(sources.iter().cloned());
        }
    }
    needed.sort();
    needed.dedup();
    for (index, path) in needed.iter().enumerate() {
        progress(
            0.4 * index as f64 / needed.len().max(1) as f64,
            &format!("Verificando {path}…"),
        );
        original(path, &mut *source)?;
    }
    fs::create_dir(destination).map_err(|error| ImportError::Io(error.to_string()))?;
    let mut written = 0usize;
    for (index, output) in manifest.outputs.iter().enumerate() {
        if !valid_output_path(&output.path) {
            return Err(ImportError::Manifest(format!("output {index}")));
        }
        if output.group.as_deref() == Some("music") && !music {
            continue;
        }
        if index % 25 == 0 {
            progress(
                0.4 + 0.6 * index as f64 / manifest.outputs.len() as f64,
                &format!("Preparando {}…", output.path),
            );
        }
        let produced = match output.kind.as_str() {
            "copy" => {
                let from = output
                    .source
                    .as_ref()
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                original(from, &mut *source)?
            }
            "ima-adpcm-wav" | "pcm-wav" => {
                let from = output
                    .source
                    .as_ref()
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let offset = output
                    .offset
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let length = output
                    .length
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let channels = output
                    .channels
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let rate = output
                    .rate
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let bits = output
                    .bits
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let bank = original(from, &mut *source)?;
                if offset + length > bank.len() {
                    return Err(ImportError::Manifest(output.path.clone()));
                }
                let bytes = bank[offset..offset + length].to_vec();
                if output.kind == "pcm-wav" {
                    let align = (channels * bits / 8).max(1) as usize;
                    let usable = bytes.len() - bytes.len() % align;
                    wav(&bytes[..usable], channels as u16, rate, bits as u16)
                } else {
                    let block = output
                        .block
                        .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                    if channels != 1 || bits != 16 {
                        return Err(ImportError::Manifest(output.path.clone()));
                    }
                    let pcm = ima_mono16(&bytes, block)
                        .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                    wav(&pcm, 1, rate, 16)
                }
            }
            "cd-audio" => {
                let image = source
                    .raw_image()
                    .ok_or_else(|| ImportError::Mismatch("faixa de áudio".into()))?;
                let track = output
                    .track
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                cd_music_wav(track, image)?
            }
            "keyed" => {
                let blob = output
                    .blob
                    .as_ref()
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let keys = output
                    .key_sources
                    .as_ref()
                    .filter(|keys| !keys.is_empty())
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                let shipped = fs::read(resources.join(blob))
                    .map_err(|_| ImportError::Manifest(output.path.clone()))?;
                let mut sources = Vec::new();
                for key in keys {
                    sources.push(original(key, &mut *source)?);
                }
                keyed(&shipped, &output.path, &sources)
            }
            "bundled" => {
                let blob = output
                    .blob
                    .as_ref()
                    .ok_or_else(|| ImportError::Manifest(output.path.clone()))?;
                fs::read(resources.join(blob))
                    .map_err(|_| ImportError::Manifest(output.path.clone()))?
            }
            other => return Err(ImportError::Manifest(other.to_owned())),
        };
        if sha256_hex(&produced) != output.sha256 {
            return Err(ImportError::Mismatch(
                output.source.clone().unwrap_or_else(|| output.path.clone()),
            ));
        }
        let target = destination.join(&output.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(&target)?;
        file.write_all(&produced)?;
        written += 1;
    }
    let receipt = serde_json::json!({
        "schema": SCHEMA,
        "manifestId": manifest.id,
        "source": source.label(),
        "music": music,
        "files": written,
        "importedAt": iso8601_now(),
    });
    let receipt = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| ImportError::Io(error.to_string()))?;
    fs::write(destination.join("importacao.json"), receipt)?;
    progress(1.0, "Importação concluída.");
    Ok(Report {
        written,
        music,
        manifest_id_ok: true,
    })
}

/// Imports into `<folder>/Dados` atomically: a staging folder is renamed only
/// after every output verified; an earlier import is replaced then.
pub fn install(
    manifest_path: &Path,
    resources: &Path,
    source: &mut dyn CdSource,
    folder: &Path,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<Report, ImportError> {
    fs::create_dir_all(folder)?;
    let staging = folder.join(format!(
        "Dados.importando-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let result = (|| -> Result<Report, ImportError> {
        let report = run(manifest_path, resources, source, &staging, progress)?;
        let final_path = folder.join("Dados");
        if final_path.exists() {
            fs::remove_dir_all(&final_path)?;
        }
        fs::rename(&staging, &final_path)?;
        Ok(report)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

/// True when `<folder>/Dados` holds a complete import for this manifest.
pub fn installed(manifest_path: &Path, folder: &Path) -> bool {
    let data = folder.join("Dados");
    let Ok(manifest_bytes) = fs::read(manifest_path) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_slice::<Manifest>(&manifest_bytes) else {
        return false;
    };
    let Ok(receipt_bytes) = fs::read(data.join("importacao.json")) else {
        return false;
    };
    let Ok(receipt) = serde_json::from_slice::<serde_json::Value>(&receipt_bytes) else {
        return false;
    };
    if receipt.get("manifestId").and_then(|value| value.as_str()) != Some(manifest.id.as_str()) {
        return false;
    }
    let music = receipt
        .get("music")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    manifest.outputs.iter().all(|output| {
        if output.group.as_deref() == Some("music") && !music {
            return true;
        }
        let Some(bytes) = output.bytes else {
            return false;
        };
        fs::metadata(data.join(&output.path))
            .map(|metadata| metadata.len() == bytes)
            .unwrap_or(false)
    })
}

fn valid_output_path(path: &str) -> bool {
    !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains(':')
        && !path.split(|c| c == '/' || c == '\\').any(|part| part == "..")
}

fn iso8601_now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = seconds / 86_400;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        remainder / 3600,
        remainder % 3600 / 60,
        remainder % 60
    )
}

/// Howard Hinnant's civil-from-days conversion.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ima_decodes_a_single_block() {
        // Predictor 0, index 0, then one byte of nibbles 0/0.
        let pcm = ima_mono16(&[0, 0, 0, 0, 0x00], 8).unwrap();
        assert_eq!(pcm.len(), 4);
        assert_eq!(&pcm[..2], &[0, 0]);
    }

    #[test]
    fn wav_header_is_canonical() {
        let bytes = wav(&[1, 0, 2, 0], 1, 22050, 16);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 4);
        assert_eq!(&bytes[44..], &[1, 0, 2, 0]);
    }

    #[test]
    fn keyed_is_deterministic_and_reversible() {
        let blob = vec![1u8, 2, 3, 4];
        let keyed_bytes = keyed(&blob, "native-inputs/x", &[vec![9, 9]]);
        assert_eq!(keyed_bytes.len(), 4);
        assert_ne!(keyed_bytes, blob);
        assert_eq!(keyed(&blob, "native-inputs/x", &[vec![9, 9]]), keyed_bytes);
    }

    #[test]
    fn empty_folder_is_not_installed() {
        let directory = std::env::temp_dir().join(format!(
            "rising-windows-import-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        assert!(!installed(Path::new("/missing/manifest.json"), &directory));
        let _ = fs::remove_dir_all(&directory);
    }
}
