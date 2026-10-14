//! Lossless full-canvas WebP animation, using image's VP8L encoder per frame.
//! Container layout: <https://developers.google.com/speed/webp/docs/riff_container>
use anyhow::{Result, ensure};
use image::{ExtendedColorType, RgbImage, codecs::webp::WebPEncoder};
use std::io::{Seek, SeekFrom, Write};

pub(super) struct Animation<W> {
    output: W,
    size: (u32, u32),
}

impl<W: Write + Seek> Animation<W> {
    pub fn new(mut output: W, size: (u32, u32)) -> Result<Self> {
        ensure!(
            size.0 > 0 && size.1 > 0 && size.0 <= 16384 && size.1 <= 16384,
            "WebP dimensions must be 1..16384"
        );
        output.write_all(b"RIFF\0\0\0\0WEBP")?;
        let mut extended = vec![0x02, 0, 0, 0]; // animation, opaque RGB
        uint24(&mut extended, size.0 - 1)?;
        uint24(&mut extended, size.1 - 1)?;
        chunk(&mut output, *b"VP8X", &extended)?;
        chunk(&mut output, *b"ANIM", &[0, 0, 0, 255, 0, 0])?; // infinite loop
        Ok(Self { output, size })
    }

    pub fn frame(&mut self, image: &RgbImage, delay_ms: u32) -> Result<()> {
        ensure!(
            image.dimensions() == self.size && (1..=60_000).contains(&delay_ms),
            "invalid animation frame dimensions/duration"
        );
        let mut still = Vec::new();
        WebPEncoder::new_lossless(&mut still).encode(
            image.as_raw(),
            self.size.0,
            self.size.1,
            ExtendedColorType::Rgb8,
        )?;
        ensure!(
            still.len() >= 20
                && &still[..4] == b"RIFF"
                && &still[8..12] == b"WEBP"
                && &still[12..16] == b"VP8L",
            "lossless encoder returned an unexpected WebP container"
        );
        let length = u32::from_le_bytes(still[16..20].try_into()?) as usize;
        ensure!(
            still.len() == 20 + length + length % 2,
            "unexpected lossless chunk length"
        );
        let mut header = vec![0; 6]; // x = y = 0
        uint24(&mut header, self.size.0 - 1)?;
        uint24(&mut header, self.size.1 - 1)?;
        uint24(&mut header, delay_ms)?;
        header.push(2); // overwrite full canvas, leave it until the next frame
        self.output.write_all(b"ANMF")?;
        self.output
            .write_all(&u32::try_from(header.len() + still.len() - 12)?.to_le_bytes())?;
        self.output.write_all(&header)?;
        self.output.write_all(&still[12..])?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        let size = self.output.stream_position()?;
        ensure!(size < u64::from(u32::MAX), "WebP RIFF exceeds 4 GiB");
        self.output.seek(SeekFrom::Start(4))?;
        self.output
            .write_all(&u32::try_from(size - 8)?.to_le_bytes())?;
        self.output.seek(SeekFrom::Start(size))?;
        Ok(())
    }
}

fn uint24(output: &mut Vec<u8>, value: u32) -> Result<()> {
    ensure!(value < 1 << 24, "WebP uint24 overflow");
    output.extend_from_slice(&value.to_le_bytes()[..3]);
    Ok(())
}

fn chunk(output: &mut impl Write, kind: [u8; 4], payload: &[u8]) -> Result<()> {
    output.write_all(&kind)?;
    output.write_all(&u32::try_from(payload.len())?.to_le_bytes())?;
    output.write_all(payload)?;
    if payload.len() % 2 == 1 {
        output.write_all(&[0])?;
    }
    Ok(())
}
