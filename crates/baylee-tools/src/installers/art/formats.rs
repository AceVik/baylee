//! Standard BMP-backed ICO and PNG-backed ICNS containers. No OS encoder.
use anyhow::{Result, ensure};
use image::{ImageEncoder, RgbaImage};

pub(super) fn ico(images: &[RgbaImage]) -> Result<Vec<u8>> {
    let mut output = vec![0, 0, 1, 0];
    output.extend_from_slice(&u16::try_from(images.len())?.to_le_bytes());
    let mut payloads = Vec::new();
    let mut offset = 6 + images.len() * 16;
    for image in images {
        let n = image.width();
        ensure!(
            n > 0 && n <= 256 && image.height() == n,
            "ICO requires square images up to 256px"
        );
        let mask_stride = n.div_ceil(32) * 4;
        let payload_size = 40 + n * n * 4 + mask_stride * n;
        let side = u8::try_from(n).unwrap_or(0);
        output.extend_from_slice(&[side, side, 0, 0]);
        output.extend_from_slice(&1_u16.to_le_bytes());
        output.extend_from_slice(&32_u16.to_le_bytes());
        output.extend_from_slice(&payload_size.to_le_bytes());
        output.extend_from_slice(&u32::try_from(offset)?.to_le_bytes());
        offset += usize::try_from(payload_size)?;
        let mut data = vec![0_u8; usize::try_from(payload_size)?];
        data[..4].copy_from_slice(&40_u32.to_le_bytes());
        data[4..8].copy_from_slice(&n.to_le_bytes());
        data[8..12].copy_from_slice(&(2 * n).to_le_bytes());
        data[12..14].copy_from_slice(&1_u16.to_le_bytes());
        data[14..16].copy_from_slice(&32_u16.to_le_bytes());
        data[20..24].copy_from_slice(&(4 * n * n).to_le_bytes());
        for y in 0..n {
            for x in 0..n {
                let pixel = image.get_pixel(x, n - 1 - y);
                let at = 40 + usize::try_from((y * n + x) * 4)?;
                data[at..at + 4].copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                if pixel[3] == 0 {
                    let at = 40 + usize::try_from(4 * n * n + y * mask_stride + x / 8)?;
                    data[at] |= 1 << (7 - x % 8);
                }
            }
        }
        payloads.push(data);
    }
    for data in payloads {
        output.extend(data);
    }
    Ok(output)
}
pub(super) fn icns(mut make: impl FnMut(u32) -> RgbaImage) -> Result<Vec<u8>> {
    let mut output = b"icns\0\0\0\0".to_vec();
    // Point-size entries plus their Retina representations. Modern macOS
    // reads PNG chunks at all these sizes, including the 16/32pt entries.
    for (kind, size) in [
        (b"icp4", 16),
        (b"ic11", 32),
        (b"icp5", 32),
        (b"ic12", 64),
        (b"ic07", 128),
        (b"ic13", 256),
        (b"ic08", 256),
        (b"ic14", 512),
        (b"ic09", 512),
        (b"ic10", 1024),
    ] {
        let image = make(size);
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png).write_image(
            image.as_raw(),
            size,
            size,
            image::ExtendedColorType::Rgba8,
        )?;
        output.extend_from_slice(kind);
        output.extend_from_slice(&u32::try_from(png.len() + 8)?.to_be_bytes());
        output.extend(png);
    }
    let size = u32::try_from(output.len())?;
    output[4..8].copy_from_slice(&size.to_be_bytes());
    Ok(output)
}
