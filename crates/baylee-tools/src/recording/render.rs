//! Local preview encoding and exact raw RGB difference measurements.
use super::cli::{Cadence, Format, Preview};
use super::{control, data, mp4, sheet, timing, webp};
use anyhow::{Result, ensure};
use image::{
    Delay, DynamicImage, Frame, RgbImage,
    codecs::gif::{GifEncoder, Repeat},
    imageops,
};
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;

pub(super) fn encode(directory: &Path, options: &Preview, allow_incomplete: bool) -> Result<Value> {
    ensure!(
        (16..=1920).contains(&options.width) && (1..=48).contains(&options.sheet_frames),
        "width must be 16..1920 and sheet-frames 1..48"
    );
    let recording = data::load(directory, allow_incomplete)?;
    let canvas = preview_size(&recording, options.width)?;
    let formats: &[Format] = match options.format {
        Format::Gif => &[Format::Gif],
        Format::Webp => &[Format::Webp],
        Format::Mp4 => &[Format::Mp4],
        Format::Both => &[Format::Gif, Format::Webp],
        Format::All => &[Format::Gif, Format::Webp, Format::Mp4],
    };
    let mut plans = Vec::new();
    for format in formats {
        let (name, quantum, limit) = match *format {
            Format::Gif => ("gif", 10, 65535),
            Format::Webp => ("webp", 1, 16384),
            Format::Mp4 => ("mp4", 1, 4096),
            Format::Both | Format::All => unreachable!(),
        };
        ensure!(
            canvas.0 <= limit && canvas.1 <= limit,
            "{name} canvas dimensions exceed {limit}"
        );
        plans.push((
            *format,
            name,
            timing::delays(&recording, options.cadence, quantum)?,
        ));
    }
    let mut outputs = Vec::new();
    if options.diff {
        outputs.push(differences(&recording)?);
    }
    let images: Vec<_> = recording
        .frames
        .iter()
        .map(|sample| {
            control::check_interrupted()?;
            let raw = data::read_frame(&recording, sample)?;
            Ok(imageops::resize(
                &raw,
                canvas.0,
                canvas.1,
                imageops::FilterType::Lanczos3,
            ))
        })
        .collect::<Result<_>>()?;
    let mut timing = serde_json::Map::new();
    for (format, name, delays) in plans {
        let path = data::output(
            &recording.directory,
            &format!("preview-{}.{name}", options.cadence.name()),
            |output| animation(output, format, canvas, &images, &delays),
        )?;
        outputs.push(path);
        timing.insert(name.into(), json!({"duration_ms":delays,"total_ms":delays.iter().map(|n| u64::from(*n)).sum::<u64>()}));
        if format == Format::Mp4 && options.cadence != Cadence::Step {
            let step_delays = timing::delays(&recording, Cadence::Step, 1)?;
            let step_path = data::output(&recording.directory, "preview-step.mp4", |output| {
                animation(output, Format::Mp4, canvas, &images, &step_delays)
            })?;
            outputs.push(step_path);
        }
    }
    let (contact, slots) = sheet::build(&recording, &images, options.sheet_frames)?;
    outputs.push(data::output(
        &recording.directory,
        "contact-sheet.png",
        |file| {
            DynamicImage::ImageRgb8(contact).write_to(file, image::ImageFormat::Png)?;
            Ok(())
        },
    )?);
    let summary = &recording.summary;
    let report = json!({
        "source":recording.directory,"saved_frames":recording.frames.len(),"phase":summary["phase"],
        "ledger_recovery":summary["ledger_recovery"],"reason":summary["reason"],
        "dropped":summary["dropped"],"discarded":summary["discarded"],"cadence":options.cadence.name(),
        "timing":timing,"contact_sheet_slots":slots,"outputs":outputs,
        "difference_report":options.diff.then_some("Consecutive raw RGB deltas over the full viewport; heuristic only, not automatic flicker/bug proof. Legitimate motion also changes pixels."),
        "note":"Real=request timestamp, virtual=game clock, nominal=requested FPS, step=1000 ms per frame. No presentation timestamps. GIF quantizes to 10 ms; WebP and MP4 to 1 ms. Raw PNGs and frames.jsonl retain all saved samples. Preview scaling/palette changes are unsuitable for exact pixel diffs. Viewer playback cadence is not guaranteed; inspect preview-step.mp4, raw frames, or contact sheet for agent QA."
    });
    data::output(
        &recording.directory,
        &format!("preview-{}.json", options.cadence.name()),
        |file| {
            serde_json::to_writer_pretty(&mut *file, &report)?;
            file.write_all(b"\n")?;
            Ok(())
        },
    )?;
    Ok(report)
}

fn animation(
    output: &mut std::fs::File,
    format: Format,
    canvas: (u32, u32),
    images: &[RgbImage],
    delays: &[u32],
) -> Result<()> {
    match format {
        Format::Gif => {
            let mut encoder = GifEncoder::new_with_speed(output, 10);
            encoder.set_repeat(Repeat::Infinite)?;
            for (image, delay) in images.iter().zip(delays) {
                control::check_interrupted()?;
                let rgba = DynamicImage::ImageRgb8(image.clone()).into_rgba8();
                encoder.encode_frame(Frame::from_parts(
                    rgba,
                    0,
                    0,
                    Delay::from_numer_denom_ms(*delay, 1),
                ))?;
            }
        }
        Format::Webp => {
            let mut encoder = webp::Animation::new(output, canvas)?;
            for (image, delay) in images.iter().zip(delays) {
                control::check_interrupted()?;
                encoder.frame(image, *delay)?;
            }
            encoder.finish()?;
        }
        Format::Mp4 => {
            let mut encoder = mp4::Animation::new(output, canvas)?;
            for (image, delay) in images.iter().zip(delays) {
                control::check_interrupted()?;
                encoder.frame(image, *delay)?;
            }
            encoder.finish()?;
        }
        Format::Both | Format::All => unreachable!(),
    }
    Ok(())
}

pub(super) fn preview_size(recording: &data::Recording, width: u32) -> Result<(u32, u32)> {
    let first = &recording.frames[0];
    let width = width.min(first.width);
    let height = u32::try_from(
        timing::round_ratio(
            u64::from(first.height) * u64::from(width),
            u64::from(first.width),
        )
        .max(1),
    )?;
    ensure!(
        u64::from(width) * u64::from(height) * recording.frames.len() as u64 <= data::MAX_PIXELS,
        "preview exceeds 64M pixels; use a smaller --width (raw frames are kept)"
    );
    Ok((width, height))
}

fn differences(recording: &data::Recording) -> Result<std::path::PathBuf> {
    data::output(&recording.directory, "differences.csv", |output| {
        writeln!(
            output,
            "previous_slot,slot,request_gap_ms,virtual_gap_ms,max_channel_delta,mean_absolute_channel_delta,changed_pixel_fraction"
        )?;
        let mut previous: Option<(RgbImage, &data::Sample)> = None;
        for sample in &recording.frames {
            control::check_interrupted()?;
            let current = data::read_frame(recording, sample)?;
            if let Some((last, metadata)) = &previous {
                let (peak, mean, changed) = difference_values(last, &current)?;
                writeln!(
                    output,
                    "{},{},{},{},{peak},{mean},{changed}",
                    metadata.slot,
                    sample.slot,
                    milliseconds(
                        i128::from(sample.requested_us) - i128::from(metadata.requested_us)
                    ),
                    milliseconds(i128::from(sample.virtual_us) - i128::from(metadata.virtual_us))
                )?;
            }
            previous = Some((current, sample));
        }
        Ok(())
    })
}

pub(super) fn difference_values(first: &RgbImage, second: &RgbImage) -> Result<(u8, f64, f64)> {
    ensure!(
        first.dimensions() == second.dimensions(),
        "frame dimensions changed"
    );
    let mut peak = 0;
    let mut sum = 0_u64;
    let mut changed = 0_u32;
    for (left, right) in first.pixels().zip(second.pixels()) {
        let deltas = [
            left[0].abs_diff(right[0]),
            left[1].abs_diff(right[1]),
            left[2].abs_diff(right[2]),
        ];
        for delta in deltas {
            peak = peak.max(delta);
            sum += u64::from(delta);
        }
        changed += u32::from(deltas != [0; 3]);
    }
    let pixels = f64::from(first.width()) * f64::from(first.height());
    // At most 8M RGB pixels * 255 per channel, exactly representable in f64.
    #[allow(clippy::cast_precision_loss)]
    let mean = sum as f64 / (pixels * 3.0);
    Ok((peak, mean, f64::from(changed) / pixels))
}

fn milliseconds(microseconds: i128) -> String {
    let sign = if microseconds < 0 { "-" } else { "" };
    let absolute = microseconds.unsigned_abs();
    format!("{sign}{}.{:03}", absolute / 1000, absolute % 1000)
}
