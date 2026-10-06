//! What a figure's own data says, read from the file the game wrote to it.
//! A figure is 64 blocks of 16 bytes. The first eight blocks and the last
//! block of each sector of four are plain; every other block the game has
//! written is encrypted with AES-128, under a key made with MD5 from the
//! figure's first two blocks, the block's number and a fixed sentence of
//! Activision's. Omoio only reads: it decrypts a copy and never writes a
//! figure, and the app reads traps only, for the villain each one holds.
//!
//! The game keeps its data twice, in two areas it writes in turn, so a
//! figure is never left half written. An area is the 21 data blocks of seven
//! sectors (blocks 8 to 35, and 36 to 63): 336 bytes once each sector's last
//! block is left out. Its first 16 bytes carry a counter and checksums: one
//! over those 16 bytes and one over the next 48, on every kind of figure. A
//! checksum only comes out right when the blocks were decrypted with the
//! right key, which is how a decryption is known to be correct.
//!
//! A trap keeps the villain it holds in its area, as the Modified_SkyEditGUI
//! trap editor documents it: byte 16 is the villain's number (0 when it is
//! empty), byte 17 whether it is evolved, and bytes 0 and 7 whether it is
//! the villain's variant form. A third checksum covers the rest of a trap's
//! area, where it keeps the villains it held before; a character's covers
//! less, so it is only checked on traps. The villain itself sits under the
//! second, so when neither area passes the third it is still read from the
//! one the game wrote last, as that editor does.

use aes::cipher::{BlockCipherDecrypt, KeyInit};
use aes::Aes128;
use md5::{Digest, Md5};
use serde::Serialize;

pub const SIZE: usize = 1024;
const BLOCK: usize = 16;
const AREA: usize = 21 * BLOCK;

/// The sentence every key is made with, with a space before and after.
const SENTENCE: &[u8; 0x35] = b" Copyright (C) 2010 Activision. All Rights Reserved. ";

/// The block each area starts at.
const AREA_STARTS: [usize; 2] = [8, 36];

/// The figure's id, which says what it is, and its variant.
pub fn id(figure: &[u8; SIZE]) -> u16 {
    u16::from_le_bytes([figure[0x10], figure[0x11]])
}

pub fn variant(figure: &[u8; SIZE]) -> u16 {
    u16::from_le_bytes([figure[0x1C], figure[0x1D]])
}

/// Whether block `index` is a sector's last, which holds the tag's keys.
fn sector_end(index: usize) -> bool {
    index % 4 == 3
}

/// A copy of the figure with every encrypted block decrypted. Blocks no game
/// has written are all zero and stay so.
pub fn decrypted(figure: &[u8; SIZE]) -> [u8; SIZE] {
    let mut seed = [0u8; 0x56];
    seed[..0x20].copy_from_slice(&figure[..0x20]);
    seed[0x21..].copy_from_slice(SENTENCE);
    let mut out = *figure;
    for (index, block) in out.chunks_exact_mut(BLOCK).enumerate() {
        if index < 8 || sector_end(index) || block.iter().all(|&byte| byte == 0) {
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

/// An area's 336 bytes: its blocks in order, each sector's last left out.
fn area(data: &[u8; SIZE], first: usize) -> [u8; AREA] {
    let mut out = [0u8; AREA];
    for (n, block) in (first..).filter(|&b| !sector_end(b)).take(AREA / BLOCK).enumerate() {
        out[n * BLOCK..(n + 1) * BLOCK].copy_from_slice(&data[block * BLOCK..(block + 1) * BLOCK]);
    }
    out
}

fn kept(area: &[u8; AREA], at: usize) -> u16 {
    u16::from_le_bytes([area[at], area[at + 1]])
}

/// Whether an area's first 64 bytes are as the game wrote them, which every
/// kind of figure checks. The first checksum is taken over the first 16
/// bytes with 05 00 where it is kept.
fn head_ok(area: &[u8; AREA]) -> bool {
    let mut head = [0u8; BLOCK];
    head.copy_from_slice(&area[..BLOCK]);
    head[0xE..].copy_from_slice(&[0x05, 0x00]);
    area[..BLOCK].iter().any(|&byte| byte != 0)
        && kept(area, 0xE) == crc16(&head)
        && kept(area, 0xC) == crc16(&area[16..64])
}

/// Whether a trap's area is whole, the villains it held before included.
fn trap_area_ok(area: &[u8; AREA]) -> bool {
    head_ok(area) && kept(area, 0xA) == crc16(&area[64..])
}

/// The area the game wrote last, of those that pass `ok`. `None` for a
/// figure no game has written to, or one that doesn't decrypt.
fn current(data: &[u8; SIZE], ok: fn(&[u8; AREA]) -> bool) -> Option<[u8; AREA]> {
    let [first, second] = AREA_STARTS.map(|start| area(data, start));
    match (ok(&first), ok(&second)) {
        // Each write takes the other area's counter plus one.
        (true, true) => Some(if second[9] == first[9].wrapping_add(1) { second } else { first }),
        (true, false) => Some(first),
        (false, true) => Some(second),
        (false, false) => None,
    }
}

/// Trap Team's traps: one id to an element, and 220 for Kaos's own.
pub fn is_trap(id: u16) -> bool {
    (210..=220).contains(&id)
}

/// What a trap holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Trapped {
    /// The villain's number in the game's own files, 1001 to 1046.
    pub villain: u16,
    /// Its variant form, such as Outlaw Brawl and Chain.
    pub variant: bool,
    pub evolved: bool,
}

/// The villain a trap holds, from the data the game wrote to it. `None`
/// when it holds none, has never been played, or isn't a trap.
pub fn trapped(figure: &[u8; SIZE]) -> Option<Trapped> {
    if !is_trap(id(figure)) {
        return None;
    }
    let data = decrypted(figure);
    let area = current(&data, trap_area_ok).or_else(|| current(&data, head_ok))?;
    (area[16] != 0).then(|| Trapped {
        villain: 1000 + u16::from(area[16]),
        variant: area[0] == 1 && area[7] == area[16],
        evolved: area[17] == 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockCipherEncrypt;

    /// The game's side, for the tests only: encrypts what the game would
    /// have written. Omoio itself never encrypts or writes a figure.
    fn encrypted(plain: &[u8; SIZE]) -> [u8; SIZE] {
        let mut seed = [0u8; 0x56];
        seed[..0x20].copy_from_slice(&plain[..0x20]);
        seed[0x21..].copy_from_slice(SENTENCE);
        let mut out = *plain;
        for (index, block) in out.chunks_exact_mut(BLOCK).enumerate() {
            if index < 8 || sector_end(index) || block.iter().all(|&byte| byte == 0) {
                continue;
            }
            seed[0x20] = index as u8;
            let cipher = Aes128::new(&Md5::digest(seed));
            cipher.encrypt_block(block.try_into().unwrap());
        }
        out
    }

    /// A new figure, as the emulator makes it: its number and checksum.
    fn made(id: u16, variant: u16) -> [u8; SIZE] {
        let mut figure = [0u8; SIZE];
        figure[..4].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        figure[0x10..0x12].copy_from_slice(&id.to_le_bytes());
        figure[0x1C..0x1E].copy_from_slice(&variant.to_le_bytes());
        let crc = crc16(&figure[..0x1E]);
        figure[0x1E..0x20].copy_from_slice(&crc.to_le_bytes());
        figure
    }

    /// Writes an area as the game would: its counter, its three checksums,
    /// and its bytes into the blocks from `first`, skipping each sector's
    /// last.
    fn write(figure: &mut [u8; SIZE], first: usize, counter: u8, mut area: [u8; AREA]) {
        area[9] = counter;
        let crc = crc16(&area[64..]);
        area[0xA..0xC].copy_from_slice(&crc.to_le_bytes());
        let crc = crc16(&area[16..64]);
        area[0xC..0xE].copy_from_slice(&crc.to_le_bytes());
        area[0xE..0x10].copy_from_slice(&[0x05, 0x00]);
        let crc = crc16(&area[..BLOCK]);
        area[0xE..0x10].copy_from_slice(&crc.to_le_bytes());
        for (n, block) in (first..).filter(|&b| !sector_end(b)).take(AREA / BLOCK).enumerate() {
            figure[block * BLOCK..(block + 1) * BLOCK].copy_from_slice(&area[n * BLOCK..(n + 1) * BLOCK]);
        }
    }

    fn some_data(fill: u8) -> [u8; AREA] {
        let mut area = [0u8; AREA];
        area[3] = 0x2A;
        area[20..24].copy_from_slice(&[1, 2, 3, fill]);
        area[200] = fill;
        area
    }

    #[test]
    fn the_checksum_is_ccitts() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn a_written_figure_decrypts_to_what_the_game_wrote() {
        let mut plain = made(3000, 0x2000);
        write(&mut plain, 8, 7, some_data(9));
        let figure = encrypted(&plain);
        assert_ne!(figure[0x80..0x90], plain[0x80..0x90]);
        assert_eq!(decrypted(&figure), plain);
        assert!(number_ok(&figure));
        assert_eq!((id(&figure), variant(&figure)), (3000, 0x2000));
        assert_eq!(current(&decrypted(&figure), trap_area_ok).map(|a| a[200]), Some(9));
    }

    #[test]
    fn the_newer_area_is_the_one_written_last() {
        let mut plain = made(3000, 0x2000);
        write(&mut plain, 8, 7, some_data(1));
        write(&mut plain, 36, 8, some_data(2));
        assert_eq!(current(&decrypted(&encrypted(&plain)), trap_area_ok).map(|a| a[200]), Some(2));
        write(&mut plain, 8, 9, some_data(3));
        assert_eq!(current(&decrypted(&encrypted(&plain)), trap_area_ok).map(|a| a[200]), Some(3));
        // The counter wraps round after 255.
        write(&mut plain, 8, 255, some_data(4));
        write(&mut plain, 36, 0, some_data(5));
        assert_eq!(current(&decrypted(&encrypted(&plain)), trap_area_ok).map(|a| a[200]), Some(5));
    }

    #[test]
    fn a_broken_or_unwritten_area_is_passed_over() {
        let mut plain = made(3000, 0x2000);
        write(&mut plain, 8, 7, some_data(1));
        write(&mut plain, 36, 8, some_data(2));
        // A newer area that was cut off half way loses to the older one.
        plain[36 * BLOCK + 40] ^= 0xFF;
        assert_eq!(current(&decrypted(&encrypted(&plain)), trap_area_ok).map(|a| a[200]), Some(1));
        assert_eq!(current(&decrypted(&made(3000, 0x2000)), trap_area_ok), None);
        // Decrypted with the wrong key, nothing checks out.
        let mut wrong = encrypted(&plain);
        wrong[0] ^= 1;
        assert_eq!(current(&decrypted(&wrong), trap_area_ok), None);
    }

    fn trap(id: u16, variant: u16, villain: u8, variant_form: bool, evolved: bool) -> [u8; SIZE] {
        let mut area = [0u8; AREA];
        area[0] = u8::from(variant_form);
        area[1] = 1;
        area[7] = if variant_form { villain } else { 0 };
        area[16] = villain;
        area[17] = u8::from(evolved);
        let mut plain = made(id, variant);
        write(&mut plain, 8, 1, area);
        encrypted(&plain)
    }

    #[test]
    fn a_trap_tells_the_villain_it_holds() {
        let oak_eagle = trap(217, 0x3003, 11, false, false);
        assert_eq!(trapped(&oak_eagle), Some(Trapped { villain: 1011, variant: false, evolved: false }));
        let outlaw = trap(211, 0x3000, 16, true, true);
        assert_eq!(trapped(&outlaw), Some(Trapped { villain: 1016, variant: true, evolved: true }));
        assert_eq!(trapped(&trap(217, 0x3003, 0, false, false)), None);
        assert_eq!(trapped(&made(217, 0x3003)), None);
        // Only a trap is read.
        assert_eq!(trapped(&trap(3000, 0x2000, 11, false, false)), None);
    }

    #[test]
    fn a_trap_tells_its_villain_when_only_the_older_ones_dont_check_out() {
        let mut area = [0u8; AREA];
        area[1] = 1;
        area[16] = 11;
        let mut plain = made(217, 0x3003);
        write(&mut plain, 8, 1, area);
        // Where byte `n` of the area sits in the figure.
        let at = |n: usize| (8..).filter(|&b| !sector_end(b)).nth(n / BLOCK).unwrap() * BLOCK + n % BLOCK;
        // A byte among the villains it held before, under the third checksum only.
        plain[at(200)] ^= 0xFF;
        assert_eq!(current(&decrypted(&encrypted(&plain)), trap_area_ok), None);
        assert_eq!(trapped(&encrypted(&plain)), Some(Trapped { villain: 1011, variant: false, evolved: false }));
        // The part with the villain broken too, nothing is read.
        plain[at(20)] ^= 0xFF;
        assert_eq!(trapped(&encrypted(&plain)), None);
    }

    /// The figures the user's own games wrote on this machine, read only, to
    /// prove the decryption and the checksums against the real thing.
    /// Skipped where there are none; the files never leave this machine.
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
            if figure[8 * BLOCK..].iter().any(|&byte| byte != 0) {
                written += 1;
                // Characters keep less under the third checksum than traps do, so
                // the two that every figure has are checked.
                assert!(current(&decrypted(&figure), head_ok).is_some(), "{:?} doesn't check out", entry.file_name());
            }
        }
        println!("{read} figures here, {written} written by a game, all check out");
    }
}
