//! Background-only PNG, MP4 and ledger writes; a failed append poisons the worker.

use super::{Event, Sample, Work};
use bevy::prelude::Image;
use image::{
    DynamicImage, ImageEncoder, Rgb, RgbImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

/// Persist frames and the ledger on the bounded background worker.
pub(super) fn write_frames(
    directory: &std::path::Path,
    initial: &str,
    video_fps: Option<u32>,
    jobs: &Receiver<Work>,
    events: &std::sync::mpsc::Sender<Event>,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .recursive(false)
        .create(directory)
        .map_err(|e| e.to_string())?;
    std::fs::write(directory.join("recording.json"), initial).map_err(|e| e.to_string())?;
    let mut ledger =
        std::fs::File::create(directory.join("frames.jsonl")).map_err(|e| e.to_string())?;
    events.send(Event::Ready).map_err(|e| e.to_string())?;
    let mut write_error = None;
    let mut video_frames: Vec<(u64, RgbImage)> = Vec::new();
    for work in jobs {
        match work {
            Work::Frame(metadata, image) => {
                let slot = metadata.slot;
                let result = write_or_cancel(&mut write_error, cancelled, || {
                    save_frame(
                        directory,
                        &mut ledger,
                        &metadata,
                        *image,
                        video_fps.is_some().then_some(&mut video_frames),
                    )
                });
                events
                    .send(Event::Written(slot, result))
                    .map_err(|e| e.to_string())?;
            }
            Work::Finish(json) => {
                let mut result = Ok(());
                if let Some(fps) = video_fps
                    && write_error.is_none()
                    && !cancelled.load(Ordering::Relaxed)
                    && !video_frames.is_empty()
                {
                    result = write_videos(directory, fps, &video_frames);
                }
                let result = result.and_then(|()| {
                    std::fs::write(directory.join("recording.json.tmp"), json)
                        .and_then(|()| {
                            std::fs::rename(
                                directory.join("recording.json.tmp"),
                                directory.join("recording.json"),
                            )
                        })
                        .map_err(|e| e.to_string())
                });
                let _ = events.send(Event::Finished(result));
                return Ok(());
            }
        }
    }
    Ok(())
}

// A partial ledger append poisons this worker: no later frame may append
// behind a truncated row, even if it was already queued before the error.
pub(super) fn write_or_cancel(
    write_error: &mut Option<String>,
    cancelled: &AtomicBool,
    save: impl FnOnce() -> Result<(), String>,
) -> Result<bool, String> {
    if let Some(error) = write_error {
        return Err(format!("writer stopped after: {error}"));
    }
    if cancelled.load(Ordering::Relaxed) {
        return Ok(false);
    }
    match save() {
        Ok(()) => Ok(true),
        Err(error) => {
            *write_error = Some(error.clone());
            Err(error)
        }
    }
}

fn save_frame(
    directory: &std::path::Path,
    ledger: &mut std::fs::File,
    metadata: &Sample,
    image: Image,
    video_frames: Option<&mut Vec<(u64, RgbImage)>>,
) -> Result<(), String> {
    let dynamic = image.try_into_dynamic().map_err(|e| e.to_string())?;
    if let Some(frames) = video_frames {
        let width = metadata.width.clamp(1, 640);
        let height = u32::try_from(
            ((u64::from(metadata.height) * u64::from(width) + u64::from(metadata.width) / 2)
                / u64::from(metadata.width.max(1)))
            .max(1),
        )
        .map_err(|e| e.to_string())?;
        frames.push((
            metadata.requested_us,
            downsample_rgb(&dynamic, width, height),
        ));
    }
    let temp = directory.join(format!("{}.tmp", metadata.file));
    {
        let file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
        let encoder = PngEncoder::new_with_quality(
            BufWriter::new(file),
            CompressionType::Fast,
            FilterType::NoFilter,
        );
        encoder
            .write_image(
                dynamic.as_bytes(),
                dynamic.width(),
                dynamic.height(),
                dynamic.color().into(),
            )
            .map_err(|e| e.to_string())?;
    }
    std::fs::rename(temp, directory.join(&metadata.file)).map_err(|e| e.to_string())?;
    // Serialize before touching the ledger, then commit the row with its
    // newline. Recovery accepts only the valid newline-terminated prefix.
    let mut line = serde_json::to_vec(metadata).map_err(|e| e.to_string())?;
    line.push(b'\n');
    ledger.write_all(&line).map_err(|e| e.to_string())
}

fn downsample_rgb(dynamic: &DynamicImage, width: u32, height: u32) -> RgbImage {
    let src_w = dynamic.width().max(1);
    let src_h = dynamic.height().max(1);
    if let Some(rgba) = dynamic.as_rgba8() {
        let raw = rgba.as_raw();
        let stride = src_w as usize * 4;
        return RgbImage::from_fn(width, height, |dx, dy| {
            let sx0 = (u64::from(dx) * u64::from(src_w) / u64::from(width)) as usize;
            let sy0 = (u64::from(dy) * u64::from(src_h) / u64::from(height)) as usize;
            let sx1 = ((sx0 + 1).min(src_w as usize - 1)) * 4;
            let sx0 = sx0 * 4;
            let row0 = sy0 * stride;
            let row1 = ((sy0 + 1).min(src_h as usize - 1)) * stride;
            let r = (u32::from(raw[row0 + sx0])
                + u32::from(raw[row0 + sx1])
                + u32::from(raw[row1 + sx0])
                + u32::from(raw[row1 + sx1])
                + 2)
                >> 2;
            let g = (u32::from(raw[row0 + sx0 + 1])
                + u32::from(raw[row0 + sx1 + 1])
                + u32::from(raw[row1 + sx0 + 1])
                + u32::from(raw[row1 + sx1 + 1])
                + 2)
                >> 2;
            let b = (u32::from(raw[row0 + sx0 + 2])
                + u32::from(raw[row0 + sx1 + 2])
                + u32::from(raw[row1 + sx0 + 2])
                + u32::from(raw[row1 + sx1 + 2])
                + 2)
                >> 2;
            Rgb([r as u8, g as u8, b as u8])
        });
    }
    let rgb = dynamic.to_rgb8();
    if (rgb.width(), rgb.height()) == (width, height) {
        rgb
    } else {
        image::imageops::resize(&rgb, width, height, image::imageops::FilterType::Nearest)
    }
}

fn write_videos(
    directory: &std::path::Path,
    fps: u32,
    video_frames: &[(u64, RgbImage)],
) -> Result<(), String> {
    let size = video_frames[0].1.dimensions();
    let images: Vec<RgbImage> = video_frames.iter().map(|(_, img)| img.clone()).collect();
    let default_ms = (1000 / fps.max(1)).max(1);
    let mut real_delays = Vec::with_capacity(video_frames.len());
    for pair in video_frames.windows(2) {
        let delta_us = pair[1].0.saturating_sub(pair[0].0);
        let ms = u32::try_from((delta_us + 500) / 1000)
            .unwrap_or(60_000)
            .clamp(1, 60_000);
        real_delays.push(ms);
    }
    real_delays.push(real_delays.last().copied().unwrap_or(default_ms));
    super::mp4::write_mp4(
        &directory.join("recording.mp4"),
        size,
        &images,
        &real_delays,
    )?;
    let step_delays = vec![1000_u32; images.len()];
    super::mp4::write_mp4(
        &directory.join("recording-step.mp4"),
        size,
        &images,
        &step_delays,
    )
}
