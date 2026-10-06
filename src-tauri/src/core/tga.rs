//! A Wii U dump keeps its pictures in its meta folder as TGA files, which
//! the webview can't show, so they are turned into PNG for the library.
//!
//! Only the kinds a dump uses are read: true colour of 24 or 32 bits, stored
//! as they are or run-length encoded, rows from the bottom or from the top.
//! The layout is Truevision's TGA 2.0 specification. The file comes from a
//! dump we did not make, so no size in it is trusted beyond the buffer.

const HEADER_LEN: usize = 18;
const TRUE_COLOUR: u8 = 2;
const TRUE_COLOUR_RLE: u8 = 10;
/// Bit 5 of the image descriptor: the first row stored is the top one.
/// Without it the bottom row comes first.
const TOP_FIRST: u8 = 0x20;

#[derive(Debug, PartialEq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    /// Red, green, blue and alpha for each pixel, the top row first.
    pub rgba: Vec<u8>,
}

pub fn decode(data: &[u8]) -> Option<Picture> {
    let header = data.get(..HEADER_LEN)?;
    let id_len = usize::from(header[0]);
    let has_map = header[1] == 1;
    let kind = header[2];
    let map_len = usize::from(u16::from_le_bytes([header[5], header[6]]));
    let map_entry_bits = usize::from(header[7]);
    let width = usize::from(u16::from_le_bytes([header[12], header[13]]));
    let height = usize::from(u16::from_le_bytes([header[14], header[15]]));
    let bytes_per_pixel = match header[16] {
        24 => 3,
        32 => 4,
        _ => return None,
    };
    let top_first = header[17] & TOP_FIRST != 0;
    if width == 0 || height == 0 {
        return None;
    }

    // A true-colour picture may still carry a colour map it doesn't use.
    let map_size = if has_map { map_len * map_entry_bits.div_ceil(8) } else { 0 };
    let pixels = data.get(HEADER_LEN + id_len + map_size..)?;
    let stored_len = width * height * bytes_per_pixel;
    let rgba = match kind {
        TRUE_COLOUR => to_rgba(pixels.get(..stored_len)?, width, bytes_per_pixel, top_first),
        TRUE_COLOUR_RLE => to_rgba(&unpack_runs(pixels, stored_len, bytes_per_pixel)?, width, bytes_per_pixel, top_first),
        _ => return None,
    };
    Some(Picture {
        width: width as u32,
        height: height as u32,
        rgba,
    })
}

/// Each packet starts with a byte whose high bit says whether one pixel
/// follows, to be repeated, or the pixels follow one by one. The low seven
/// bits are the count less one. A packet may run on into the next row.
fn unpack_runs(mut data: &[u8], stored_len: usize, bytes_per_pixel: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    while out.len() < stored_len {
        let (&head, rest) = data.split_first()?;
        let count = usize::from(head & 0x7F) + 1;
        if head & 0x80 != 0 {
            let pixel = rest.get(..bytes_per_pixel)?;
            for _ in 0..count {
                out.extend_from_slice(pixel);
            }
            data = &rest[bytes_per_pixel..];
        } else {
            let run = count * bytes_per_pixel;
            out.extend_from_slice(rest.get(..run)?);
            data = &rest[run..];
        }
    }
    out.truncate(stored_len);
    Some(out)
}

/// TGA stores each pixel as blue, green, red, then alpha when it has one.
fn to_rgba(stored: &[u8], width: usize, bytes_per_pixel: usize, top_first: bool) -> Vec<u8> {
    let mut rows: Vec<&[u8]> = stored.chunks_exact(width * bytes_per_pixel).collect();
    if !top_first {
        rows.reverse();
    }
    let mut rgba = Vec::with_capacity(rows.len() * width * 4);
    for row in rows {
        for pixel in row.chunks_exact(bytes_per_pixel) {
            let alpha = if bytes_per_pixel == 4 { pixel[3] } else { 255 };
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], alpha]);
        }
    }
    rgba
}

/// The picture as the bytes of a PNG file.
pub fn to_png(picture: &Picture) -> Option<Vec<u8>> {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, picture.width, picture.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&picture.rgba).ok()?;
    writer.finish().ok()?;
    Some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tga(kind: u8, depth: u8, descriptor: u8, width: u16, height: u16, body: &[u8]) -> Vec<u8> {
        let mut file = vec![0, 0, kind, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&height.to_le_bytes());
        file.extend_from_slice(&[depth, descriptor]);
        file.extend_from_slice(body);
        file
    }

    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];

    #[test]
    fn reads_24_bit_rows_stored_bottom_first() {
        // Bottom row blue, white; top row red, green. Each pixel is stored
        // blue, green, red.
        let body = [255, 0, 0, 255, 255, 255, 0, 0, 255, 0, 255, 0];
        let picture = decode(&tga(TRUE_COLOUR, 24, 0, 2, 2, &body)).unwrap();
        assert_eq!((picture.width, picture.height), (2, 2));
        assert_eq!(picture.rgba, [RED, GREEN, BLUE, WHITE].concat());
    }

    #[test]
    fn reads_32_bit_alpha_with_rows_stored_top_first() {
        let body = [0, 0, 255, 128, 255, 0, 0, 0];
        let picture = decode(&tga(TRUE_COLOUR, 32, 0x28, 2, 1, &body)).unwrap();
        assert_eq!(picture.rgba, [255, 0, 0, 128, 0, 0, 255, 0]);
    }

    #[test]
    fn reads_run_length_packets_that_cross_a_row() {
        // Three wide, two high, top first: a run of four reds, which goes
        // on into the second row, then two pixels as they are.
        let body = [0x83, 0, 0, 255, 0x01, 0, 255, 0, 255, 0, 0];
        let picture = decode(&tga(TRUE_COLOUR_RLE, 24, TOP_FIRST, 3, 2, &body)).unwrap();
        assert_eq!(picture.rgba, [RED, RED, RED, RED, GREEN, BLUE].concat());
    }

    #[test]
    fn reads_run_length_32_bit_rows_stored_bottom_first() {
        let body = [0x80, 0, 0, 255, 255, 0x80, 255, 0, 0, 255];
        let picture = decode(&tga(TRUE_COLOUR_RLE, 32, 0x08, 1, 2, &body)).unwrap();
        assert_eq!(picture.rgba, [BLUE, RED].concat());
    }

    #[test]
    fn a_run_longer_than_the_picture_is_cut_at_its_end() {
        let body = [0xFF, 255, 255, 255];
        let picture = decode(&tga(TRUE_COLOUR_RLE, 24, 0, 1, 1, &body)).unwrap();
        assert_eq!(picture.rgba, WHITE);
    }

    #[test]
    fn skips_the_id_and_a_colour_map_it_does_not_use() {
        let mut file = tga(TRUE_COLOUR, 24, 0, 1, 1, &[]);
        file[0] = 3;
        file[1] = 1;
        file[5..7].copy_from_slice(&2u16.to_le_bytes());
        file[7] = 24;
        file.extend_from_slice(b"abc");
        file.extend_from_slice(&[9; 6]);
        file.extend_from_slice(&[0, 255, 0]);
        assert_eq!(decode(&file).unwrap().rgba, GREEN);
    }

    #[test]
    fn refuses_what_it_does_not_read() {
        let pixel = [0, 0, 255];
        assert_eq!(decode(&tga(1, 24, 0, 1, 1, &pixel)), None, "colour-mapped");
        assert_eq!(decode(&tga(3, 24, 0, 1, 1, &pixel)), None, "greyscale");
        assert_eq!(decode(&tga(TRUE_COLOUR, 16, 0, 1, 1, &[0, 0])), None, "16 bit");
        assert_eq!(decode(&tga(TRUE_COLOUR, 24, 0, 0, 1, &pixel)), None, "no width");
        assert_eq!(decode(&tga(TRUE_COLOUR, 24, 0, 2, 1, &pixel)), None, "cut short");
        assert_eq!(decode(&tga(TRUE_COLOUR_RLE, 24, 0, 2, 1, &[0x80, 0, 0])), None, "run cut short");
        assert_eq!(decode(&tga(TRUE_COLOUR_RLE, 24, 0, 2, 1, &[0x80, 0, 0, 255])), None, "too few packets");
        assert_eq!(decode(&[0, 0, TRUE_COLOUR]), None, "no header");
    }

    #[test]
    fn the_png_holds_the_same_pixels() {
        let picture = Picture {
            width: 2,
            height: 2,
            rgba: [RED, GREEN, BLUE, [1, 2, 3, 4]].concat(),
        };
        let file = to_png(&picture).unwrap();
        let mut reader = png::Decoder::new(file.as_slice()).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!((info.width, info.height), (2, 2));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(&pixels[..info.buffer_size()], picture.rgba.as_slice());
    }
}
