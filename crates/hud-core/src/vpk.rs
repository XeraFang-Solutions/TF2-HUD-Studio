//! Read-only Valve Pak (VPK v1/v2) reader, used to fall back to stock TF2 files and materials.
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Entry {
    archive: u16,
    offset: u32,
    length: u32,
    preload: Vec<u8>,
}

#[derive(Debug)]
pub struct Vpk {
    dir_path: PathBuf,
    data_start: u64,
    entries: HashMap<String, Entry>,
}

fn cstr(buf: &[u8], pos: &mut usize) -> Result<String, String> {
    let start = *pos;
    while *pos < buf.len() && buf[*pos] != 0 {
        *pos += 1
    }
    if *pos >= buf.len() {
        return Err("Truncated VPK directory".into());
    }
    let s = String::from_utf8_lossy(&buf[start..*pos]).to_string();
    *pos += 1;
    Ok(s)
}

fn u16_at(buf: &[u8], pos: usize) -> Result<u16, String> {
    buf.get(pos..pos + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| "Truncated VPK".into())
}
fn u32_at(buf: &[u8], pos: usize) -> Result<u32, String> {
    buf.get(pos..pos + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "Truncated VPK".into())
}

impl Vpk {
    pub fn open(dir_path: &Path) -> Result<Vpk, String> {
        let mut f = File::open(dir_path).map_err(|e| format!("{}: {e}", dir_path.display()))?;
        let mut header = [0u8; 28];
        f.read_exact(&mut header[..12]).map_err(|e| e.to_string())?;
        if u32_at(&header, 0)? != 0x55aa1234 {
            return Err("Not a VPK directory file".into());
        }
        let version = u32_at(&header, 4)?;
        let tree_size = u32_at(&header, 8)? as usize;
        let header_size = match version {
            1 => 12,
            2 => {
                f.read_exact(&mut header[12..28]).map_err(|e| e.to_string())?;
                28
            }
            v => return Err(format!("Unsupported VPK version {v}")),
        };
        let mut tree = vec![0u8; tree_size];
        f.read_exact(&mut tree).map_err(|e| e.to_string())?;
        let mut entries = HashMap::new();
        let mut pos = 0usize;
        loop {
            let ext = cstr(&tree, &mut pos)?;
            if ext.is_empty() {
                break;
            }
            loop {
                let dir = cstr(&tree, &mut pos)?;
                if dir.is_empty() {
                    break;
                }
                loop {
                    let name = cstr(&tree, &mut pos)?;
                    if name.is_empty() {
                        break;
                    }
                    let preload_len = u16_at(&tree, pos + 4)? as usize;
                    let archive = u16_at(&tree, pos + 6)?;
                    let offset = u32_at(&tree, pos + 8)?;
                    let length = u32_at(&tree, pos + 12)?;
                    pos += 18;
                    let preload = tree.get(pos..pos + preload_len).ok_or("Truncated VPK preload")?.to_vec();
                    pos += preload_len;
                    let mut full = String::new();
                    if dir != " " {
                        full.push_str(&dir);
                        full.push('/');
                    }
                    full.push_str(&name);
                    if ext != " " {
                        full.push('.');
                        full.push_str(&ext);
                    }
                    entries.insert(full.to_ascii_lowercase(), Entry { archive, offset, length, preload });
                }
            }
        }
        Ok(Vpk { dir_path: dir_path.to_path_buf(), data_start: (header_size + tree_size) as u64, entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(&path.to_ascii_lowercase())
    }
    /// All paths starting with `prefix` (lowercase, forward slashes).
    pub fn list(&self, prefix: &str) -> Vec<String> {
        let p = prefix.to_ascii_lowercase();
        let mut v: Vec<String> = self.entries.keys().filter(|k| k.starts_with(&p)).cloned().collect();
        v.sort();
        v
    }

    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        let e = self.entries.get(&path.to_ascii_lowercase())?;
        let mut out = e.preload.clone();
        if e.length == 0 {
            return Some(out);
        }
        let (file, offset) = if e.archive == 0x7fff {
            (self.dir_path.clone(), self.data_start + e.offset as u64)
        } else {
            let stem = self.dir_path.file_name()?.to_string_lossy().to_string();
            let base = stem.strip_suffix("_dir.vpk")?;
            (self.dir_path.with_file_name(format!("{base}_{:03}.vpk", e.archive)), e.offset as u64)
        };
        let mut f = File::open(file).ok()?;
        f.seek(SeekFrom::Start(offset)).ok()?;
        let mut data = vec![0u8; e.length as usize];
        f.read_exact(&mut data).ok()?;
        out.extend_from_slice(&data);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Builds a tiny v2 VPK with one file stored inline in the directory and one split preload/archive file.
    #[test]
    fn reads_inline_and_archived_entries() {
        let dir = std::env::temp_dir().join(format!("hudvpk{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut tree = Vec::new();
        let mut put = |s: &str| {
            tree.extend_from_slice(s.as_bytes());
            tree.push(0)
        };
        put("res");
        put("resource/ui");
        put("HudPlayerHealth");
        let inline = b"\"r\"{}";
        let mut entry = Vec::new();
        entry.extend_from_slice(&0u32.to_le_bytes());
        entry.extend_from_slice(&0u16.to_le_bytes());
        entry.extend_from_slice(&0x7fffu16.to_le_bytes());
        entry.extend_from_slice(&0u32.to_le_bytes());
        entry.extend_from_slice(&(inline.len() as u32).to_le_bytes());
        entry.extend_from_slice(&0xffffu16.to_le_bytes());
        tree.extend_from_slice(&entry);
        tree.push(0); // end files
        tree.push(0); // end dirs
        tree.extend_from_slice(b"txt\0 \0readme\0");
        let mut e2 = Vec::new();
        e2.extend_from_slice(&0u32.to_le_bytes());
        e2.extend_from_slice(&2u16.to_le_bytes());
        e2.extend_from_slice(&0u16.to_le_bytes());
        e2.extend_from_slice(&0u32.to_le_bytes());
        e2.extend_from_slice(&3u32.to_le_bytes());
        e2.extend_from_slice(&0xffffu16.to_le_bytes());
        tree.extend_from_slice(&e2);
        tree.extend_from_slice(b"hi");
        tree.extend_from_slice(&[0, 0, 0]);
        let mut file = Vec::new();
        file.extend_from_slice(&0x55aa1234u32.to_le_bytes());
        file.extend_from_slice(&2u32.to_le_bytes());
        file.extend_from_slice(&(tree.len() as u32).to_le_bytes());
        file.extend_from_slice(&[0u8; 16]);
        file.extend_from_slice(&tree);
        file.extend_from_slice(inline);
        std::fs::write(dir.join("pak_dir.vpk"), file).unwrap();
        std::fs::write(dir.join("pak_000.vpk"), b"!!!").unwrap();
        let vpk = Vpk::open(&dir.join("pak_dir.vpk")).unwrap();
        assert_eq!(vpk.read("resource/ui/hudplayerhealth.res").unwrap(), inline);
        assert_eq!(vpk.read("readme.txt").unwrap(), b"hi!!!");
        assert_eq!(vpk.list("resource/").len(), 1);
    }
}
