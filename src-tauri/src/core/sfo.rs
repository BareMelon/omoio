//! PARAM.SFO is the metadata file in every PS3 dump, and every import decision
//! downstream reads from it. It comes from files we did not produce, so nothing
//! here trusts a length or an offset without checking it against the buffer.

use std::collections::BTreeMap;
use std::fmt;

const MAGIC: &[u8; 4] = b"\0PSF";
const HEADER_LEN: usize = 20;
const INDEX_ENTRY_LEN: usize = 16;

const FMT_UTF8: u16 = 0x0004;
const FMT_UTF8_NUL: u16 = 0x0204;
const FMT_INT32: u16 = 0x0404;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    TooSmall,
    BadMagic,
    TableOutOfBounds,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall => write!(f, "file is too small to be a PARAM.SFO"),
            Self::BadMagic => write!(f, "file does not start with the PARAM.SFO magic"),
            Self::TableOutOfBounds => write!(f, "PARAM.SFO tables point outside the file"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Int(u32),
}

/// What kind of thing a dump is. `GameData` is an update or add-on, never a
/// title in its own right, and must not reach the library as one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Disc,
    Hdd,
    GameData,
    Other,
}

impl Category {
    fn from_code(code: &str) -> Self {
        match code {
            "DG" => Self::Disc,
            "HG" => Self::Hdd,
            "GD" => Self::GameData,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Default)]
pub struct Sfo {
    entries: BTreeMap<String, Value>,
}

impl Sfo {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::TooSmall);
        }
        if &bytes[..4] != MAGIC {
            return Err(Error::BadMagic);
        }

        let key_table = read_u32(bytes, 8) as usize;
        let data_table = read_u32(bytes, 12) as usize;
        let count = read_u32(bytes, 16) as usize;

        if key_table > bytes.len() || data_table > bytes.len() {
            return Err(Error::TableOutOfBounds);
        }
        // The index sits between the header and the key table, so it cannot be
        // longer than the gap between them.
        let index_len = count
            .checked_mul(INDEX_ENTRY_LEN)
            .ok_or(Error::TableOutOfBounds)?;
        if HEADER_LEN + index_len > key_table {
            return Err(Error::TableOutOfBounds);
        }

        let mut entries = BTreeMap::new();
        for i in 0..count {
            let at = HEADER_LEN + i * INDEX_ENTRY_LEN;
            let key_offset = read_u16(bytes, at) as usize;
            let data_fmt = read_u16(bytes, at + 2);
            let data_len = read_u32(bytes, at + 4) as usize;
            let data_offset = read_u32(bytes, at + 12) as usize;

            // One malformed entry should not cost us the rest of the file.
            let Some(key) = read_key(bytes, key_table + key_offset) else {
                continue;
            };
            let Some(raw) = slice_at(bytes, data_table + data_offset, data_len) else {
                continue;
            };
            let Some(value) = decode(data_fmt, raw) else {
                continue;
            };
            entries.insert(key, value);
        }

        Ok(Self { entries })
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        match self.entries.get(key) {
            Some(Value::Text(s)) => Some(s),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<u32> {
        match self.entries.get(key) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        }
    }

    pub fn title(&self) -> Option<&str> {
        self.text("TITLE")
    }

    pub fn title_id(&self) -> Option<&str> {
        self.text("TITLE_ID")
    }

    /// The version of the game itself. Disc dumps label it VERSION, installed
    /// titles label it APP_VER, and plenty of dumps carry both.
    pub fn app_version(&self) -> Option<&str> {
        self.text("APP_VER").or_else(|| self.text("VERSION"))
    }

    pub fn firmware_required(&self) -> Option<&str> {
        self.text("PS3_SYSTEM_VER")
    }

    pub fn category(&self) -> Option<Category> {
        self.text("CATEGORY").map(Category::from_code)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

fn decode(data_fmt: u16, raw: &[u8]) -> Option<Value> {
    match data_fmt {
        // Both text forms appear in real dumps; the terminated one counts its
        // NUL in data_len, and some writers pad with more than one.
        FMT_UTF8 | FMT_UTF8_NUL => {
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            Some(Value::Text(String::from_utf8_lossy(&raw[..end]).into_owned()))
        }
        FMT_INT32 => {
            let bytes: [u8; 4] = raw.get(..4)?.try_into().ok()?;
            Some(Value::Int(u32::from_le_bytes(bytes)))
        }
        _ => None,
    }
}

fn read_key(bytes: &[u8], start: usize) -> Option<String> {
    let rest = bytes.get(start..)?;
    let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(String::from_utf8_lossy(&rest[..end]).into_owned())
}

fn slice_at(bytes: &[u8], start: usize, len: usize) -> Option<&[u8]> {
    bytes.get(start..start.checked_add(len)?)
}

// The header and index are fixed-size and already bounds-checked by the caller,
// so these read within the buffer by construction.
fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Also used by other modules' tests to make a PARAM.SFO.
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct Entry {
        key: &'static str,
        fmt: u16,
        data: Vec<u8>,
        max: usize,
    }

    pub(crate) fn text(key: &'static str, value: &str) -> Entry {
        let mut data = value.as_bytes().to_vec();
        data.push(0);
        let max = data.len();
        Entry { key, fmt: FMT_UTF8_NUL, data, max }
    }

    pub(crate) fn build(entries: &[Entry]) -> Vec<u8> {
        let (mut keys, mut data, mut index) = (Vec::new(), Vec::new(), Vec::new());
        for e in entries {
            let key_offset = keys.len();
            keys.extend_from_slice(e.key.as_bytes());
            keys.push(0);

            let data_offset = data.len();
            data.extend_from_slice(&e.data);
            data.resize(data_offset + e.max, 0);

            index.extend_from_slice(&(key_offset as u16).to_le_bytes());
            index.extend_from_slice(&e.fmt.to_le_bytes());
            index.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
            index.extend_from_slice(&(e.max as u32).to_le_bytes());
            index.extend_from_slice(&(data_offset as u32).to_le_bytes());
        }
        while keys.len() % 4 != 0 {
            keys.push(0);
        }

        let key_table = HEADER_LEN + index.len();
        let data_table = key_table + keys.len();

        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&0x0101u32.to_le_bytes());
        out.extend_from_slice(&(key_table as u32).to_le_bytes());
        out.extend_from_slice(&(data_table as u32).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&keys);
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn reads_all_three_data_formats() {
        let bytes = build(&[
            // Not null-terminated: every byte of data_len is content.
            Entry { key: "TITLE_ID", fmt: FMT_UTF8, data: b"BCES00141".to_vec(), max: 9 },
            text("TITLE", "LittleBigPlanet"),
            Entry {
                key: "PARENTAL_LEVEL",
                fmt: FMT_INT32,
                data: 5u32.to_le_bytes().to_vec(),
                max: 4,
            },
        ]);
        let sfo = Sfo::parse(&bytes).unwrap();

        assert_eq!(sfo.text("TITLE_ID"), Some("BCES00141"));
        assert_eq!(sfo.text("TITLE"), Some("LittleBigPlanet"));
        assert_eq!(sfo.int("PARENTAL_LEVEL"), Some(5));
    }

    #[test]
    fn keeps_text_and_int_lookups_apart() {
        let bytes = build(&[
            text("TITLE", "Demo"),
            Entry { key: "ATTRIBUTE", fmt: FMT_INT32, data: 1u32.to_le_bytes().to_vec(), max: 4 },
        ]);
        let sfo = Sfo::parse(&bytes).unwrap();

        assert_eq!(sfo.int("TITLE"), None);
        assert_eq!(sfo.text("ATTRIBUTE"), None);
    }

    #[test]
    fn reads_int_values_with_the_high_bit_set() {
        let bytes = build(&[Entry {
            key: "ATTRIBUTE",
            fmt: FMT_INT32,
            data: 0x8000_0001u32.to_le_bytes().to_vec(),
            max: 4,
        }]);
        assert_eq!(Sfo::parse(&bytes).unwrap().int("ATTRIBUTE"), Some(0x8000_0001));
    }

    #[test]
    fn maps_categories() {
        for (code, expected) in [
            ("DG", Category::Disc),
            ("HG", Category::Hdd),
            ("GD", Category::GameData),
            ("AP", Category::Other),
        ] {
            let bytes = build(&[text("CATEGORY", code)]);
            assert_eq!(Sfo::parse(&bytes).unwrap().category(), Some(expected));
        }
        assert_eq!(Sfo::parse(&build(&[])).unwrap().category(), None);
    }

    #[test]
    fn prefers_app_ver_but_accepts_version() {
        let both = build(&[text("APP_VER", "01.03"), text("VERSION", "01.00")]);
        assert_eq!(Sfo::parse(&both).unwrap().app_version(), Some("01.03"));

        let disc_only = build(&[text("VERSION", "01.00")]);
        assert_eq!(Sfo::parse(&disc_only).unwrap().app_version(), Some("01.00"));

        assert_eq!(Sfo::parse(&build(&[])).unwrap().app_version(), None);
    }

    #[test]
    fn strips_padding_from_fixed_width_fields() {
        // Real dumps reserve far more room than the value needs.
        let mut data = b"BCES00141".to_vec();
        data.push(0);
        let bytes = build(&[Entry { key: "TITLE_ID", fmt: FMT_UTF8_NUL, data, max: 16 }]);
        assert_eq!(Sfo::parse(&bytes).unwrap().title_id(), Some("BCES00141"));
    }

    #[test]
    fn rejects_files_that_are_not_sfo() {
        assert_eq!(Sfo::parse(b"").unwrap_err(), Error::TooSmall);
        assert_eq!(Sfo::parse(b"\0PSF").unwrap_err(), Error::TooSmall);
        assert_eq!(Sfo::parse(&[0u8; 64]).unwrap_err(), Error::BadMagic);
        assert_eq!(
            Sfo::parse(b"PK\x03\x04padding padding padding").unwrap_err(),
            Error::BadMagic
        );
    }

    #[test]
    fn rejects_tables_pointing_outside_the_file() {
        let mut bytes = build(&[text("TITLE", "Demo")]);
        bytes[8..12].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        assert_eq!(Sfo::parse(&bytes).unwrap_err(), Error::TableOutOfBounds);

        let mut bytes = build(&[text("TITLE", "Demo")]);
        bytes[16..20].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        assert_eq!(Sfo::parse(&bytes).unwrap_err(), Error::TableOutOfBounds);
    }

    #[test]
    fn survives_a_truncated_file_without_panicking() {
        let full = build(&[text("TITLE", "LittleBigPlanet"), text("TITLE_ID", "BCES00141")]);
        // Every truncation must fail cleanly or return what it could read.
        for len in 0..full.len() {
            let _ = Sfo::parse(&full[..len]);
        }
    }

    #[test]
    fn skips_a_bad_entry_but_keeps_the_rest() {
        let mut bytes = build(&[text("TITLE", "Demo"), text("TITLE_ID", "BCES00141")]);
        // Point the second entry's data far outside the file.
        let second = HEADER_LEN + INDEX_ENTRY_LEN;
        bytes[second + 12..second + 16].copy_from_slice(&0xFFFF_0000u32.to_le_bytes());

        let sfo = Sfo::parse(&bytes).unwrap();
        assert_eq!(sfo.title(), Some("Demo"));
        assert_eq!(sfo.title_id(), None);
    }

    #[test]
    fn ignores_formats_it_does_not_know() {
        let bytes = build(&[
            Entry { key: "MYSTERY", fmt: 0x0999, data: vec![1, 2, 3, 4], max: 4 },
            text("TITLE", "Demo"),
        ]);
        let sfo = Sfo::parse(&bytes).unwrap();
        assert_eq!(sfo.text("MYSTERY"), None);
        assert_eq!(sfo.title(), Some("Demo"));
    }

    /// Checks the parser against real PARAM.SFO files, which is the only thing
    /// that proves it works. Game files are not committed, so point this at a
    /// folder of dumps to run it:
    ///   set OMOIO_SFO_CORPUS=D:\dumps
    ///   cargo test real_files -- --ignored --nocapture
    #[test]
    #[ignore = "needs OMOIO_SFO_CORPUS pointing at real dumps"]
    fn parses_real_files() {
        let root = std::env::var("OMOIO_SFO_CORPUS").expect("set OMOIO_SFO_CORPUS");
        let mut found = 0;
        let mut stack = vec![std::path::PathBuf::from(root)];

        while let Some(dir) = stack.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else { continue };
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().is_some_and(|n| n.eq_ignore_ascii_case("PARAM.SFO")) {
                    let bytes = std::fs::read(&path).unwrap();
                    let sfo = Sfo::parse(&bytes)
                        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                    println!(
                        "{:?} {:>10} {:>7} {}",
                        sfo.category().unwrap_or(Category::Other),
                        sfo.title_id().unwrap_or("-"),
                        sfo.app_version().unwrap_or("-"),
                        sfo.title().unwrap_or("-"),
                    );
                    found += 1;
                }
            }
        }
        assert!(found > 0, "no PARAM.SFO files found");
    }
}
