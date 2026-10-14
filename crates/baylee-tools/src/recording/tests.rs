//! Local synthetic fixtures exercise conversion, not gameplay or performance.
mod encoding;
mod http;
mod validation;

use super::cli::{Cadence, Format, Preview};
use super::{data, render, timing};
use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn fixture(directory: &Path) -> Vec<Value> {
    let summary = json!({"schema":1,"phase":"complete","reason":"duration",
        "options":{"fps":10,"duration_ms":500},"capture_end_us":500_000,
        "dropped":2,"discarded":0});
    fs::write(directory.join("recording.json"), summary.to_string()).unwrap();
    let mut frames = Vec::new();
    for (slot, real) in [(0, 0), (1, 100_000), (4, 400_000)] {
        let name = format!("frame-{slot:06}.png");
        let mut image = RgbImage::from_pixel(80, 48, Rgb([22, 27, 34]));
        for y in 8..=32 {
            for x in slot * 12..=slot * 12 + 16 {
                image.put_pixel(x, y, Rgb([255; 3]));
            }
        }
        image.save(directory.join(&name)).unwrap();
        frames.push(json!({"slot":slot,"file":name,"width":80,"height":48,
            "nominal_us":real,"requested_us":real,"virtual_us":5_000_000 + real / 2,
            "received_us":real + 23_000}));
    }
    write_rows(directory, frames.iter().rev());
    frames
}

fn write_rows<'a>(directory: &Path, rows: impl IntoIterator<Item = &'a Value>) {
    let mut bytes = Vec::new();
    for row in rows {
        serde_json::to_writer(&mut bytes, row).unwrap();
        bytes.push(b'\n');
    }
    fs::write(directory.join("frames.jsonl"), bytes).unwrap();
}

fn preview() -> Preview {
    Preview {
        format: Format::Both,
        width: 80,
        cadence: Cadence::Real,
        sheet_frames: 12,
        diff: false,
    }
}

#[test]
fn timelines_preserve_drops_end_hold_and_virtual_scale() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let recording = data::load(dir.path(), false).unwrap();
    assert_eq!(
        timing::delays(&recording, Cadence::Real, 10).unwrap(),
        [100, 300, 100]
    );
    assert_eq!(
        timing::delays(&recording, Cadence::Virtual, 1).unwrap(),
        [50, 150, 150]
    );
    assert_eq!(
        timing::delays(&recording, Cadence::Nominal, 1).unwrap(),
        [100, 300, 300]
    );
}

#[test]
fn quantization_does_not_accumulate_sixty_fps_drift() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut recording = data::load(dir.path(), false).unwrap();
    let sample = recording.frames[0].clone();
    recording.frames = (0..60)
        .map(|i| data::Sample {
            requested_us: i * 1_000_000 / 60,
            ..sample.clone()
        })
        .collect();
    recording.summary["options"]["fps"] = json!(60);
    recording.summary["capture_end_us"] = json!(1_000_000);
    let delays = timing::delays(&recording, Cadence::Real, 10).unwrap();
    assert_eq!(delays.iter().sum::<u32>(), 1000);
    assert!(delays.contains(&10) && delays.contains(&20));
    assert!(delays.iter().all(|delay| matches!(delay, 10 | 20)));
}

#[test]
fn invalid_timing_is_rejected_and_short_holds_get_one_tick() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut recording = data::load(dir.path(), false).unwrap();
    recording.frames[1].virtual_us = recording.frames[0].virtual_us;
    assert!(timing::delays(&recording, Cadence::Virtual, 1).is_err());
    recording.frames[1].requested_us = recording.frames[0].requested_us;
    assert!(timing::delays(&recording, Cadence::Real, 10).is_err());
    recording.frames.truncate(1);
    recording.summary["capture_end_us"] = json!(60_000_001);
    assert!(timing::delays(&recording, Cadence::Real, 1).is_err());
    recording.summary["capture_end_us"] = json!(0);
    assert_eq!(timing::delays(&recording, Cadence::Real, 10).unwrap(), [10]);
    recording.summary["options"]["fps"] = json!(0);
    assert!(timing::delays(&recording, Cadence::Real, 10).is_err());
}

#[test]
fn sheet_sampling_spans_endpoints_without_duplicates() {
    assert_eq!(timing::sheet_indices(1, 12), [0]);
    assert_eq!(timing::sheet_indices(3, 12), [0, 1, 2]);
    let indices = timing::sheet_indices(900, 12);
    assert_eq!((indices[0], indices[11]), (0, 899));
    assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(timing::round_ratio(5, 2), 2);
    assert_eq!(timing::round_ratio(7, 2), 4);
}
