//! What a figure's own data says, read from the file the game wrote to it.
//! A figure is 64 blocks of 16 bytes. The first eight blocks and the last
//! block of each sector of four are plain; every other block the game has
//! written is encrypted with AES-128, under a key made with MD5 from the
//! figure's first two blocks, the block's number and a fixed sentence of
//! Activision's. Omoio only reads: it decrypts a copy and never writes a
//! figure, and the app reads traps only, for the villain each one holds.
//!
//! The game keeps its data twice, in two areas it writes in turn, each with
//! a counter and a checksum, so a figure is never left half written. A
//! checksum only comes out right when the block was decrypted with the
//! right key, which is how a decryption is known to be correct.

use aes::cipher::{BlockCipherDecrypt, KeyInit};
use aes::Aes128;
use md5::{Digest, Md5};

pub const SIZE: usize = 1024;
const BLOCK: usize = 16;

/// The sentence every key is made with, with a space before and after.
const SENTENCE: &[u8; 0x35] = b" Copyright (C) 2010 Activision. All Rights Reserved. ";

/// Where the two areas start.
const AREAS: [usize; 2] = [0x80, 0x240];

/// The figure's id, which says what it is, and its variant.
pub fn id(figure: &[u8; SIZE]) -> u16 {
    u16::from_le_bytes([figure[0x10], figure[0x11]])
}

pub fn variant(figure: &[u8; SIZE]) -> u16 {
    u16::from_le_bytes([figure[0x1C], figure[0x1D]])
}

/// Whether block `index` is kept plain: the first eight, and the last of
/// each sector, which holds the tag's keys.
fn plain(index: usize) -> bool {
    index < 8 || index % 4 == 3
}

/// A copy of the figure with every encrypted block decrypted. Blocks no game
/// has written are all zero and stay so.
pub fn decrypted(figure: &[u8; SIZE]) -> [u8; SIZE] {
    let mut seed = [0u8; 0x56];
    seed[..0x20].copy_from_slice(&figure[..0x20]);
    seed[0x21..].copy_from_slice(SENTENCE);
    let mut out = *figure;
    for (index, block) in out.chunks_exact_mut(BLOCK).enumerate() {
        if plain(index) || block.iter().all(|&byte| byte == 0) {
            continue;
        }
        seed[0x20] = index as u8;
        let cipher = Aes128::new(&Md5::digest(seed));
        let block: &mut aes::Block = block.try_into().expect("a block is 16 bytes");
        cipher.decrypt_block(block);
    }
    out
}

/// CRC-16 as the game checks its data: CCITT's, starting from 0xFFFF.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Whether the figure's own number checks out: a checksum over its first 30
/// bytes, kept in the next two. The emulator writes it when it makes the
/// figure, so this holds for every figure, written to or not.
pub fn number_ok(figure: &[u8; SIZE]) -> bool {
    crc16(&figure[..0x1E]) == u16::from_le_bytes([figure[0x1E], figure[0x1F]])
}

/// Whether an area's first block is as the game wrote it. Its checksum is
/// kept in its last two bytes and is taken over the block with 05 00 there.
fn header_ok(data: &[u8; SIZE], area: usize) -> bool {
    let mut block = [0u8; BLOCK];
    block.copy_from_slice(&data[area..area + BLOCK]);
    let kept = u16::from_le_bytes([block[0xE], block[0xF]]);
    block[0xE..].copy_from_slice(&[0x05, 0x00]);
    block.iter().any(|&byte| byte != 0) && crc16(&block) == kept
}

/// Where the area the game wrote last starts, of the areas that check out.
/// `None` for a figure no game has written to, or one that doesn't decrypt.
pub fn current_area(data: &[u8; SIZE]) -> Option<usize> {
    let [first, second] = AREAS.map(|area| header_ok(data, area));
    match (first, second) {
        (true, true) => {
            // Each write takes the other area's counter plus one.
            let newer = data[AREAS[1] + 9] == data[AREAS[0] + 9].wrapping_add(1);
            Some(AREAS[usize::from(newer)])
        }
        (true, false) => Some(AREAS[0]),
        (false, true) => Some(AREAS[1]),
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockCipherEncrypt;

    /// The game's side, for the tests only: encrypts what the game would
    /// have written. Omoio itself never encrypts or writes a figure.
    fn encrypted(plain_figure: &[u8; SIZE]) -> [u8; SIZE] {
        let mut seed = [0u8; 0x56];
        seed[..0x20].copy_from_slice(&plain_figure[..0x20]);
        seed[0x21..].copy_from_slice(SENTENCE);
        let mut out = *plain_figure;
        for (index, block) in out.chunks_exact_mut(BLOCK).enumerate() {
            if plain(index) || block.iter().all(|&byte| byte == 0) {
                continue;
            }
            seed[0x20] = index as u8;
            let cipher = Aes128::new(&Md5::digest(seed));
            cipher.encrypt_block(block.try_into().unwrap());
        }
        out
    }

    /// A figure as the game leaves it: the number with its checksum, and an
    /// area header with a counter, some data and its checksum.
    fn written(id: u16, variant: u16, area: usize, counter: u8) -> [u8; SIZE] {
        let mut figure = [0u8; SIZE];
        figure[..4].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        figure[0x10..0x12].copy_from_slice(&id.to_le_bytes());
        figure[0x1C..0x1E].copy_from_slice(&variant.to_le_bytes());
        let crc = crc16(&figure[..0x1E]);
        figure[0x1E..0x20].copy_from_slice(&crc.to_le_bytes());
        figure[area + 3] = 0x2A;
        figure[area + 9] = counter;
        figure[area + 0xE] = 0x05;
        let crc = crc16(&figure[area..area + BLOCK]);
        figure[area + 0xE..area + BLOCK].copy_from_slice(&crc.to_le_bytes());
        figure[area + 0x10..area + 0x14].copy_from_slice(&[1, 2, 3, 4]);
        figure
    }

    #[test]
    fn the_checksum_is_ccitts() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn a_written_figure_decrypts_to_what_the_game_wrote() {
        let plain_figure = written(212, 0x300E, 0x80, 7);
        let figure = encrypted(&plain_figure);
        assert_ne!(figure[0x80..0x90], plain_figure[0x80..0x90]);
        let data = decrypted(&figure);
        assert_eq!(data, plain_figure);
        assert!(number_ok(&figure));
        assert_eq!((id(&figure), variant(&figure)), (212, 0x300E));
        assert_eq!(current_area(&data), Some(0x80));
    }

    #[test]
    fn the_newer_area_is_the_one_written_last() {
        let mut plain_figure = written(210, 0x3015, 0x80, 7);
        let second = written(210, 0x3015, 0x240, 8);
        plain_figure[0x240..0x260].copy_from_slice(&second[0x240..0x260]);
        assert_eq!(current_area(&decrypted(&encrypted(&plain_figure))), Some(0x240));
        // The counter wraps round after 255.
        plain_figure[0x89] = 255;
        let first = &mut plain_figure[0x80..0x90];
        first[0xE..].copy_from_slice(&[0x05, 0x00]);
        let crc = crc16(first);
        first[0xE..].copy_from_slice(&crc.to_le_bytes());
        plain_figure[0x249] = 0;
        let header = &mut plain_figure[0x240..0x250];
        header[0xE..].copy_from_slice(&[0x05, 0x00]);
        let crc = crc16(header);
        header[0xE..].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(current_area(&decrypted(&encrypted(&plain_figure))), Some(0x240));
    }

    #[test]
    fn a_figure_no_game_has_written_has_no_area() {
        let mut figure = written(216, 0x3004, 0x80, 1);
        figure[0x80..].fill(0);
        assert_eq!(current_area(&decrypted(&figure)), None);
        // Decrypted with the wrong key, a header doesn't check out.
        let mut wrong = encrypted(&written(216, 0x3004, 0x80, 1));
        wrong[0] ^= 1;
        assert_eq!(current_area(&decrypted(&wrong)), None);
    }

    /// The figures the user's own games wrote on this machine, read only, to
    /// prove the decryption against the real thing. Skipped where there are
    /// none; the files never leave this machine.
    #[test]
    fn the_figures_the_game_wrote_here_decrypt_and_check_out() {
        let Some(folder) = std::env::var_os("APPDATA").map(|a| std::path::PathBuf::from(a).join("Omoio").join("figures")) else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(&folder) else {
            return;
        };
        let (mut read, mut written) = (0, 0);
        for entry in entries.flatten() {
            let Ok(bytes) = std::fs::read(entry.path()) else { continue };
            let Ok(figure) = <[u8; SIZE]>::try_from(bytes.as_slice()) else { continue };
            read += 1;
            assert!(number_ok(&figure), "{:?}", entry.file_name());
            let data = decrypted(&figure);
            let touched = AREAS.iter().any(|&area| figure[area..area + BLOCK].iter().any(|&b| b != 0));
            if touched {
                written += 1;
                assert!(current_area(&data).is_some(), "{:?} doesn't decrypt", entry.file_name());
            }
        }
        println!("{read} figures here, {written} written by a game, all check out");
    }
}
