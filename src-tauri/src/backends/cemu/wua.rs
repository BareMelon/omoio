//! Cemu's compressed Wii U archive (.wua), which is Exzap's ZArchive format
//! (MIT-0): the files' bytes one after another, cut into 64 KiB blocks that
//! are each stored as zstd, or as they are when that saved nothing, then a
//! table of where the blocks start, the names, the file tree, and a footer
//! that says where those are. Every number is big-endian. Cemu writes the
//! files already decrypted, so reading them needs no key. The same reader
//! as omoio-portraits', cut down to the few small files Omoio needs.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const MAGIC: u32 = 0x169f_52d6;
const VERSION: u32 = 0x61bf_3a01;
const FOOTER_SIZE: u64 = 144;
const BLOCK: usize = 64 * 1024;
/// Each offset record gives where one block starts and the stored sizes of
/// it and the fifteen after it, less one.
const BLOCKS_PER_RECORD: usize = 16;
const RECORD_SIZE: usize = 8 + 2 * BLOCKS_PER_RECORD;
/// A node of the file tree: a name and a flag, then a file's offset and size
/// or a folder's first child and count.
const NODE_SIZE: usize = 16;
const IS_FILE: u32 = 0x8000_0000;
/// The tables of a 13 GB game come to about half a megabyte. A damaged
/// footer must not have Omoio set aside gigabytes for them.
const MOST_TABLE: u64 = 64 * 1024 * 1024;
/// Omoio reads meta.xml and the game's pictures, a few megabytes at most.
const MOST_FILE: u64 = 16 * 1024 * 1024;

pub struct Archive {
    file: File,
    data_start: u64,
    records: Vec<u8>,
    names: Vec<u8>,
    tree: Vec<u8>,
}

fn be16(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn be32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn be64(bytes: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn read_at(file: &mut File, offset: u64, length: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = vec![0; length];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn damaged() -> String {
    "This .wua file is damaged. Have Cemu make it again.".to_string()
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, String> {
        let not_wua = || "This isn't a .wua file made by Cemu.".to_string();
        let mut file = File::open(path).map_err(|_| "Couldn't open this .wua file.".to_string())?;
        let size = file.metadata().map_err(|_| not_wua())?.len();
        if size < FOOTER_SIZE {
            return Err(not_wua());
        }
        let footer = read_at(&mut file, size - FOOTER_SIZE, FOOTER_SIZE as usize).map_err(|_| not_wua())?;
        if be32(&footer, 140) != MAGIC || be32(&footer, 136) != VERSION || be64(&footer, 128) != size {
            return Err(not_wua());
        }
        let section = |index: usize| (be64(&footer, index * 16), be64(&footer, index * 16 + 8));
        let mut load = |(offset, length): (u64, u64)| {
            if length > MOST_TABLE || offset.checked_add(length).is_none_or(|end| end > size) {
                return Err(damaged());
            }
            read_at(&mut file, offset, length as usize).map_err(|_| damaged())
        };
        let records = load(section(1))?;
        let names = load(section(2))?;
        let tree = load(section(3))?;
        Ok(Self {
            file,
            data_start: section(0).0,
            records,
            names,
            tree,
        })
    }

    /// A name from the name table: one byte of length, or two when the
    /// first has its top bit set, then the text.
    fn name(&self, offset: usize) -> String {
        let Some(&first) = self.names.get(offset) else {
            return String::new();
        };
        let (length, start) = if first & 0x80 == 0 {
            (usize::from(first), offset + 1)
        } else {
            let second = self.names.get(offset + 1).copied().unwrap_or(0);
            (usize::from(first & 0x7f) | usize::from(second) << 7, offset + 2)
        };
        self.names
            .get(start..start + length)
            .map(|text| String::from_utf8_lossy(text).into_owned())
            .unwrap_or_default()
    }

    fn node(&self, index: usize) -> Option<&[u8]> {
        self.tree.get(index * NODE_SIZE..(index + 1) * NODE_SIZE)
    }

    /// A folder's children, as their node numbers.
    fn children(&self, folder: usize) -> std::ops::Range<usize> {
        match self.node(folder) {
            Some(node) if be32(node, 0) & IS_FILE == 0 => {
                let (first, count) = (be32(node, 4) as usize, be32(node, 8) as usize);
                first..first.saturating_add(count)
            }
            _ => 0..0,
        }
    }

    /// The names of the folders at the top, in the archive's own order.
    pub fn folders(&self) -> Vec<String> {
        self.children(0)
            .filter_map(|child| self.node(child).map(|node| be32(node, 0)))
            .filter(|head| head & IS_FILE == 0)
            .map(|head| self.name(head as usize))
            .collect()
    }

    /// One small file by its path from the top, such as
    /// `0005000010101e00_v0/meta/meta.xml`. Names are matched without regard
    /// to case, as ZArchive's own lookup does.
    pub fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        let mut at = 0;
        for part in path.split('/').filter(|part| !part.is_empty()) {
            at = self.children(at).find(|&child| {
                self.node(child)
                    .is_some_and(|node| self.name((be32(node, 0) & !IS_FILE) as usize).eq_ignore_ascii_case(part))
            })?;
        }
        let node = self.node(at)?;
        if be32(node, 0) & IS_FILE == 0 {
            return None;
        }
        let high = u64::from(be32(node, 12));
        let offset = u64::from(be32(node, 4)) | (high & 0xffff) << 32;
        let size = u64::from(be32(node, 8)) | (high & 0xffff_0000) << 16;
        if size > MOST_FILE {
            return None;
        }
        self.bytes(offset, size).ok()
    }

    /// One block, unpacked.
    fn block(&mut self, index: usize) -> Result<Vec<u8>, String> {
        let at = index / BLOCKS_PER_RECORD * RECORD_SIZE;
        let record = self.records.get(at..at + RECORD_SIZE).ok_or_else(damaged)?;
        let sub = index % BLOCKS_PER_RECORD;
        let offset = be64(record, 0) + (0..sub).map(|k| u64::from(be16(record, 8 + 2 * k)) + 1).sum::<u64>();
        let stored = usize::from(be16(record, 8 + 2 * sub)) + 1;
        let bytes = read_at(&mut self.file, self.data_start + offset, stored).map_err(|_| damaged())?;
        if stored == BLOCK {
            return Ok(bytes);
        }
        zstd::bulk::decompress(&bytes, BLOCK).map_err(|_| damaged())
    }

    fn bytes(&mut self, offset: u64, size: u64) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::with_capacity(size as usize);
        let end = offset + size;
        let mut at = offset;
        while at < end {
            let block = self.block((at / BLOCK as u64) as usize)?;
            let from = (at % BLOCK as u64) as usize;
            if from >= block.len() {
                return Err(damaged());
            }
            let take = (block.len() - from).min((end - at) as usize);
            bytes.extend_from_slice(&block[from..from + take]);
            at += take as u64;
        }
        Ok(bytes)
    }
}

/// Builds small archives the way Cemu lays one out, for the tests here and
/// in the backend.
#[cfg(test)]
pub mod build {
    use super::*;

    /// An archive of `files`, each a path from the top and its bytes. Every
    /// block is compressed unless that saves nothing, when it is stored as it
    /// is.
    pub fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        // The folders and files as a tree, children kept together and in the
        // order given, which is how ZArchive numbers its nodes.
        #[derive(Default)]
        struct Folder {
            name: String,
            folders: Vec<Folder>,
            files: Vec<(String, u64, u64)>,
        }
        let mut data = Vec::new();
        let mut root = Folder::default();
        for (path, bytes) in files {
            let parts: Vec<&str> = path.split('/').collect();
            let (file, folders) = parts.split_last().unwrap();
            let mut at = &mut root;
            for name in folders {
                let index = match at.folders.iter().position(|f| f.name == *name) {
                    Some(index) => index,
                    None => {
                        at.folders.push(Folder { name: name.to_string(), ..Folder::default() });
                        at.folders.len() - 1
                    }
                };
                at = &mut at.folders[index];
            }
            at.files.push((file.to_string(), data.len() as u64, bytes.len() as u64));
            data.extend_from_slice(bytes);
        }

        let mut names = Vec::new();
        let mut name = |text: &str| {
            let at = names.len() as u32;
            names.push(text.len() as u8);
            names.extend_from_slice(text.as_bytes());
            at
        };
        // Breadth first: a folder's children sit together, numbered after
        // every node before them.
        let mut nodes: Vec<[u32; 4]> = vec![[0; 4]];
        let mut queue = vec![(0usize, &root)];
        while let Some((index, folder)) = queue.first().copied() {
            queue.remove(0);
            let first = nodes.len() as u32;
            nodes[index][1] = first;
            nodes[index][2] = (folder.folders.len() + folder.files.len()) as u32;
            for child in &folder.folders {
                nodes.push([name(&child.name), 0, 0, 0]);
                queue.push((nodes.len() - 1, child));
            }
            for (file, offset, size) in &folder.files {
                nodes.push([IS_FILE | name(file), *offset as u32, *size as u32, 0]);
            }
        }

        let blocks: Vec<Vec<u8>> = data
            .chunks(BLOCK)
            .map(|chunk| {
                let packed = zstd::bulk::compress(chunk, 3).unwrap();
                if packed.len() >= BLOCK && chunk.len() == BLOCK {
                    chunk.to_vec()
                } else {
                    packed
                }
            })
            .collect();
        let mut out: Vec<u8> = blocks.concat();
        let data_size = out.len() as u64;

        let records_at = out.len() as u64;
        let mut before = 0u64;
        for group in blocks.chunks(BLOCKS_PER_RECORD) {
            out.extend_from_slice(&before.to_be_bytes());
            for k in 0..BLOCKS_PER_RECORD {
                let size = group.get(k).map_or(0, |b| (b.len() - 1) as u16);
                out.extend_from_slice(&size.to_be_bytes());
            }
            before += group.iter().map(|b| b.len() as u64).sum::<u64>();
        }
        let records_size = out.len() as u64 - records_at;

        let names_at = out.len() as u64;
        out.extend_from_slice(&names);
        let names_size = names.len() as u64;

        let tree_at = out.len() as u64;
        for node in &nodes {
            node.iter().for_each(|word| out.extend_from_slice(&word.to_be_bytes()));
        }
        let tree_size = out.len() as u64 - tree_at;

        let total = out.len() as u64 + FOOTER_SIZE;
        for (offset, size) in [(0, data_size), (records_at, records_size), (names_at, names_size), (tree_at, tree_size), (0, 0), (0, 0)] {
            out.extend_from_slice(&u64::to_be_bytes(offset));
            out.extend_from_slice(&u64::to_be_bytes(size));
        }
        out.extend_from_slice(&[0; 32]);
        out.extend_from_slice(&total.to_be_bytes());
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(&MAGIC.to_be_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(bytes: &[u8], name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("omoio-wua-{}-{name}.wua", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn a_file_reads_back_across_blocks() {
        // A first block that doesn't compress, so it is stored as it is, then
        // a short one that does.
        let mut seed = 0x2545_f491_u32;
        let mut data: Vec<u8> = (0..BLOCK)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        data.extend((0..5000).map(|i| (i % 251) as u8));
        let path = written(
            &build::archive(&[("title_v0/code/app.rpx", b"code"), ("title_v0/content/a.bin", &data)]),
            "blocks",
        );
        let mut archive = Archive::open(&path).unwrap();
        assert_eq!(archive.folders(), ["title_v0"]);
        assert_eq!(archive.read("title_v0/content/a.bin").unwrap(), data);
        assert_eq!(archive.read("TITLE_V0/Code/APP.RPX").unwrap(), b"code", "names match without case");
        assert_eq!(archive.read("title_v0/content"), None, "a folder is not a file");
        assert_eq!(archive.read("title_v0/meta/meta.xml"), None);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn every_title_folder_is_listed_in_order() {
        let path = written(
            &build::archive(&[
                ("0005000010101e00_v0/meta/meta.xml", b"game"),
                ("0005000e10101e00_v80/meta/meta.xml", b"update"),
                ("0005000c10101e00_v16/meta/meta.xml", b"dlc"),
            ]),
            "titles",
        );
        let mut archive = Archive::open(&path).unwrap();
        assert_eq!(archive.folders(), ["0005000010101e00_v0", "0005000e10101e00_v80", "0005000c10101e00_v16"]);
        assert_eq!(archive.read("0005000e10101e00_v80/meta/meta.xml").unwrap(), b"update");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn anything_else_is_refused() {
        let path = written(&[0; 400], "junk");
        assert!(Archive::open(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
