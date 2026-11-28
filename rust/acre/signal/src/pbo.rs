//! Reading files out of PBOs, and finding which loaded PBO holds a virtual path
//! (`\a3\map_altis\altis.wrp`).

use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::compress;

/// `Cprs` (LZSS-compressed entry), as the C++ compared it: a multi-character constant.
const METHOD_COMPRESSED: u32 = 0x4370_7273;

#[derive(Debug, Clone)]
pub struct Entry {
    /// Lower-case path inside the PBO.
    pub name: String,
    pub method: u32,
    pub original_size: u32,
    pub storage_size: u32,
    /// Offset of the data from the start of the data block.
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub struct Pbo {
    /// Lower-case `prefix` header property, without a leading backslash.
    pub prefix: String,
    pub entries: Vec<Entry>,
    data_start: u64,
}

fn read_cstring(r: &mut impl Read) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut b = [0u8; 1];
    loop {
        r.read_exact(&mut b)?;
        if b[0] == 0 {
            break;
        }
        bytes.push(b[0]);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

struct RawEntry {
    name: String,
    method: u32,
    original_size: u32,
    storage_size: u32,
}

fn read_entry_header(r: &mut impl Read) -> io::Result<RawEntry> {
    let name = read_cstring(r)?.to_lowercase();
    let method = read_u32(r)?;
    let original_size = read_u32(r)?;
    let _reserved = read_u32(r)?;
    let _timestamp = read_u32(r)?;
    let storage_size = read_u32(r)?;
    Ok(RawEntry {
        name,
        method,
        original_size,
        storage_size,
    })
}

impl Pbo {
    /// Reads the header (properties and the entry table).
    pub fn read_header(r: &mut (impl Read + Seek)) -> io::Result<Self> {
        let first = read_entry_header(r)?;
        let mut properties = HashMap::new();
        let mut entries = Vec::new();
        let mut offset = 0u64;
        let mut push = |e: RawEntry, entries: &mut Vec<Entry>| {
            entries.push(Entry {
                name: e.name,
                method: e.method,
                original_size: e.original_size,
                storage_size: e.storage_size,
                offset,
            });
            offset += u64::from(e.storage_size);
        };
        if first.name.is_empty() {
            // The `Vers` entry, followed by key/value properties.
            loop {
                let key = read_cstring(r)?;
                if key.is_empty() {
                    break;
                }
                let value = read_cstring(r)?;
                properties.insert(key.to_lowercase(), value);
            }
        } else {
            push(first, &mut entries);
        }
        loop {
            let e = read_entry_header(r)?;
            if e.name.is_empty() {
                break;
            }
            push(e, &mut entries);
        }
        let data_start = r.stream_position()?;
        let prefix = properties
            .get("prefix")
            .map(|p| normalize(p))
            .unwrap_or_default();
        Ok(Self {
            prefix,
            entries,
            data_start,
        })
    }

    /// The entry's contents, decompressed.
    pub fn read(&self, r: &mut (impl Read + Seek), entry: &Entry) -> io::Result<Vec<u8>> {
        r.seek(SeekFrom::Start(self.data_start + entry.offset))?;
        let mut data = vec![0u8; entry.storage_size as usize];
        r.read_exact(&mut data)?;
        if entry.method == METHOD_COMPRESSED && entry.original_size != entry.storage_size {
            let (out, _) = compress::lzss(&data, entry.original_size as usize)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            return Ok(out);
        }
        Ok(data)
    }

    /// Virtual paths (`prefix\name`) of every entry.
    pub fn virtual_paths(&self) -> impl Iterator<Item = (String, &Entry)> {
        self.entries.iter().map(|e| {
            let path = if self.prefix.is_empty() {
                e.name.clone()
            } else {
                format!("{}\\{}", self.prefix, e.name)
            };
            (path, e)
        })
    }
}

/// Lower-case, backslashes, no leading or trailing backslash.
#[must_use]
pub fn normalize(path: &str) -> String {
    path.replace('/', "\\").trim_matches('\\').to_lowercase()
}

/// Which PBO (on disk) holds each virtual path.
#[derive(Debug, Default)]
pub struct Index {
    files: HashMap<String, PathBuf>,
    indexed: std::collections::HashSet<PathBuf>,
}

impl Index {
    /// Indexes the PBOs not seen before, in parallel. Unreadable PBOs are skipped.
    pub fn add(&mut self, pbos: impl IntoIterator<Item = PathBuf>) {
        let new: Vec<PathBuf> = pbos
            .into_iter()
            .filter(|p| !self.indexed.contains(p))
            .collect();
        if new.is_empty() {
            return;
        }
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(16));
        let chunk = new.len().div_ceil(threads);
        let found: Vec<(String, PathBuf)> = std::thread::scope(|s| {
            let handles: Vec<_> = new
                .chunks(chunk)
                .map(|paths| {
                    s.spawn(move || {
                        let mut out = Vec::new();
                        for path in paths {
                            match File::open(path)
                                .and_then(|f| Pbo::read_header(&mut BufReader::new(f)))
                            {
                                Ok(pbo) => out.extend(
                                    pbo.virtual_paths().map(|(v, _)| (v, path.clone())),
                                ),
                                Err(e) => {
                                    tracing::debug!(pbo = %path.display(), error = %e, "unreadable PBO");
                                }
                            }
                        }
                        out
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap_or_default())
                .collect()
        });
        tracing::info!(pbos = new.len(), files = found.len(), "indexed PBOs");
        self.indexed.extend(new);
        self.files.extend(found);
    }

    /// The PBO holding `virtual_path`.
    #[must_use]
    pub fn find(&self, virtual_path: &str) -> Option<&Path> {
        self.files
            .get(&normalize(virtual_path))
            .map(PathBuf::as_path)
    }

    /// Reads `virtual_path` from whichever indexed PBO holds it.
    pub fn read(&self, virtual_path: &str) -> io::Result<Vec<u8>> {
        let path = normalize(virtual_path);
        let pbo_path = self
            .files
            .get(&path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.clone()))?;
        let mut r = BufReader::new(File::open(pbo_path)?);
        let pbo = Pbo::read_header(&mut r)?;
        let (_, entry) = pbo
            .virtual_paths()
            .find(|(v, _)| *v == path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.clone()))?;
        pbo.read(&mut r, entry)
    }
}

/// Writes a minimal PBO (for tests and tools).
#[must_use]
pub fn write(prefix: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let entry = |out: &mut Vec<u8>, name: &str, method: u32, size: u32| {
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        for v in [method, size, 0, 0, size] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    };
    entry(&mut out, "", 0x5665_7273, 0);
    out.extend_from_slice(b"prefix\0");
    out.extend_from_slice(prefix.as_bytes());
    out.extend_from_slice(b"\0\0");
    for (name, data) in files {
        entry(
            &mut out,
            name,
            0,
            u32::try_from(data.len()).expect("small file"),
        );
    }
    entry(&mut out, "", 0, 0);
    for (_, data) in files {
        out.extend_from_slice(data);
    }
    out.push(0);
    out.extend_from_slice(&[0u8; 20]); // checksum stand-in
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip() {
        let bytes = write(
            "z\\test\\addons\\main",
            &[("a.txt", b"hello"), ("Data\\B.bin", &[1, 2, 3])],
        );
        let mut r = Cursor::new(bytes);
        let pbo = Pbo::read_header(&mut r).unwrap();
        assert_eq!(pbo.prefix, "z\\test\\addons\\main");
        let paths: Vec<_> = pbo.virtual_paths().map(|(v, _)| v).collect();
        assert_eq!(
            paths,
            [
                "z\\test\\addons\\main\\a.txt",
                "z\\test\\addons\\main\\data\\b.bin"
            ]
        );
        let b = pbo.read(&mut r, &pbo.entries[1]).unwrap();
        assert_eq!(b, [1, 2, 3]);
        assert_eq!(normalize("\\Z\\Test\\a.TXT"), "z\\test\\a.txt");
    }

    #[test]
    fn index_finds_files() {
        let dir = std::env::temp_dir().join(format!("acre2_signal_pbo_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.pbo");
        std::fs::write(&a, write("a3\\map_test", &[("test.wrp", b"wrp!")])).unwrap();
        let mut index = Index::default();
        index.add([a.clone(), dir.join("missing.pbo")]);
        assert_eq!(index.find("\\A3\\Map_Test\\Test.wrp"), Some(a.as_path()));
        assert_eq!(index.read("\\a3\\map_test\\test.wrp").unwrap(), b"wrp!");
        assert!(index.read("\\a3\\map_test\\other.wrp").is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
