//! Contact sheet labels use a tiny code-defined diagnostic font, no font assets.
use super::data::{MAX_PIXELS, Recording};
use super::timing::{round_ratio, sheet_indices};
use anyhow::{Result, ensure};
use image::{Rgb, RgbImage, imageops};

pub(super) fn build(
    recording: &Recording,
    previews: &[RgbImage],
    requested: u32,
) -> Result<(RgbImage, Vec<u32>)> {
    let indices = sheet_indices(previews.len(), requested);
    let columns = (indices.len() as u32).min(4);
    let tile_width = previews[0].width().min(320);
    let tile_height = u32::try_from(
        round_ratio(
            u64::from(previews[0].height()) * u64::from(tile_width),
            u64::from(previews[0].width()),
        )
        .max(1),
    )?;
    // Keep labels readable even when the original is a tiny test fixture.
    let cell_width = 320;
    let cell_height = tile_height
        .checked_add(52)
        .ok_or_else(|| anyhow::anyhow!("contact sheet height overflow"))?;
    let height = (indices.len() as u32)
        .div_ceil(columns)
        .checked_mul(cell_height)
        .ok_or_else(|| anyhow::anyhow!("contact sheet height overflow"))?;
    let width = columns * cell_width;
    ensure!(
        u64::from(width) * u64::from(height) <= MAX_PIXELS,
        "contact sheet exceeds 64M pixels; reduce --sheet-frames"
    );
    let mut sheet = RgbImage::from_pixel(width, height, Rgb([22, 27, 34]));
    for (ordinal, index) in indices.iter().copied().enumerate() {
        let x = ordinal as u32 % columns * cell_width;
        let y = ordinal as u32 / columns * cell_height;
        let tile = imageops::resize(
            &previews[index],
            tile_width,
            tile_height,
            imageops::FilterType::Lanczos3,
        );
        imageops::replace(
            &mut sheet,
            &tile,
            i64::from(x + (cell_width - tile_width) / 2),
            i64::from(y),
        );
        let sample = &recording.frames[index];
        for (row, label) in [
            format!(
                "SLOT {}  REQUEST {}S",
                sample.slot,
                seconds(sample.requested_us)
            ),
            format!("VIRTUAL {}S", seconds(sample.virtual_us)),
            format!("DELIVERY {}S", seconds(sample.received_us)),
        ]
        .iter()
        .enumerate()
        {
            label_text(
                &mut sheet,
                x + 4,
                y + tile_height + 4 + row as u32 * 15,
                x + cell_width,
                label,
            );
        }
    }
    Ok((
        sheet,
        indices
            .iter()
            .map(|index| recording.frames[*index].slot)
            .collect(),
    ))
}

fn seconds(microseconds: u64) -> String {
    let milliseconds = round_ratio(microseconds, 1000);
    format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000)
}

fn label_text(image: &mut RgbImage, x: u32, y: u32, right: u32, text: &str) {
    for (column, letter) in text.bytes().enumerate() {
        let left = x + column as u32 * 8;
        if left + 6 >= right {
            break;
        }
        for (row, bits) in glyph(letter).into_iter().enumerate() {
            for bit in 0..3 {
                if bits & (1 << (2 - bit)) != 0 {
                    for dy in 0..2 {
                        for dx in 0..2 {
                            image.put_pixel(
                                left + bit * 2 + dx,
                                y + row as u32 * 2 + dy,
                                Rgb([255; 3]),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn glyph(letter: u8) -> [u8; 5] {
    match letter {
        b'0' | b'O' => [7, 5, 5, 5, 7],
        b'1' => [2, 6, 2, 2, 7],
        b'2' => [7, 1, 7, 4, 7],
        b'3' => [7, 1, 7, 1, 7],
        b'4' => [5, 5, 7, 1, 1],
        b'5' | b'S' => [7, 4, 7, 1, 7],
        b'6' => [7, 4, 7, 5, 7],
        b'7' => [7, 1, 2, 2, 2],
        b'8' => [7, 5, 7, 5, 7],
        b'9' => [7, 5, 7, 1, 7],
        b'A' => [2, 5, 7, 5, 5],
        b'D' => [6, 5, 5, 5, 6],
        b'E' => [7, 4, 6, 4, 7],
        b'I' => [7, 2, 2, 2, 7],
        b'L' => [4, 4, 4, 4, 7],
        b'Q' => [7, 5, 5, 7, 1],
        b'R' => [6, 5, 6, 5, 5],
        b'T' => [7, 2, 2, 2, 2],
        b'U' => [5, 5, 5, 5, 7],
        b'V' => [5, 5, 5, 5, 2],
        b'Y' => [5, 5, 2, 2, 2],
        b'.' => [0, 0, 0, 0, 2],
        _ => [0; 5],
    }
}
