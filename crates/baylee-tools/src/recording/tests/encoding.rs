use super::*;
use image::{
    AnimationDecoder, DynamicImage, Frame,
    codecs::{gif::GifDecoder, webp::WebPDecoder},
    metadata::LoopCount,
};
use std::fs::File;
use std::io::{BufReader, Cursor, Write};

fn assert_animation(frames: &[Frame]) {
    assert_eq!(frames.len(), 3);
    assert_eq!(
        frames
            .iter()
            .map(|frame| frame.delay().numer_denom_ms())
            .collect::<Vec<_>>(),
        [(100, 1), (300, 1), (100, 1)]
    );
    assert!(
        frames
            .iter()
            .all(|frame| frame.buffer().dimensions() == (80, 48))
    );
}

#[test]
fn actual_gif_webp_durations_sheet_and_raw_preservation() {
    let dir = tempfile::tempdir().unwrap();
    let rows = fixture(dir.path());
    let original: Vec<_> = rows
        .iter()
        .map(|row| fs::read(dir.path().join(row["file"].as_str().unwrap())).unwrap())
        .collect();
    let report = render::encode(dir.path(), &preview(), false).unwrap();
    assert_eq!(report["saved_frames"], 3);
    assert_eq!(report["contact_sheet_slots"], json!([0, 1, 4]));
    assert_eq!(report["dropped"], 2);
    assert_eq!(report["outputs"].as_array().unwrap().len(), 3);
    for kind in ["gif", "webp"] {
        assert_eq!(
            report["timing"][kind]["duration_ms"],
            json!([100, 300, 100])
        );
        assert_eq!(report["timing"][kind]["total_ms"], 500);
    }
    let gif = GifDecoder::new(BufReader::new(
        File::open(dir.path().join("preview-real.gif")).unwrap(),
    ))
    .unwrap();
    // GIF exposes the loop extension after its header has been decoded.
    assert!(matches!(gif.loop_count(), LoopCount::Infinite));
    assert_animation(&gif.into_frames().collect_frames().unwrap());
    let webp = WebPDecoder::new(BufReader::new(
        File::open(dir.path().join("preview-real.webp")).unwrap(),
    ))
    .unwrap();
    assert!(webp.has_animation());
    assert!(matches!(webp.loop_count(), LoopCount::Infinite));
    let decoded = webp.into_frames().collect_frames().unwrap();
    assert_animation(&decoded);
    for ((row, bytes), frame) in rows.iter().zip(&original).zip(decoded) {
        assert_eq!(
            &fs::read(dir.path().join(row["file"].as_str().unwrap())).unwrap(),
            bytes
        );
        assert_eq!(
            frame.into_buffer(),
            image::load_from_memory(bytes).unwrap().into_rgba8()
        );
    }
    let sheet = image::open(dir.path().join("contact-sheet.png"))
        .unwrap()
        .into_rgb8();
    assert_eq!(sheet.dimensions(), (960, 100));
    assert!(
        sheet
            .rows()
            .skip(48)
            .flatten()
            .any(|pixel| pixel.0 == [255; 3])
    );
    let saved: Value =
        serde_json::from_slice(&fs::read(dir.path().join("preview-real.json")).unwrap()).unwrap();
    assert_eq!(saved, report);
}

#[test]
fn raw_difference_report_has_identical_counterexample_and_exact_delta() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    fs::copy(
        dir.path().join("frame-000000.png"),
        dir.path().join("frame-000001.png"),
    )
    .unwrap();
    let mut options = preview();
    options.diff = true;
    let report = render::encode(dir.path(), &options, false).unwrap();
    let csv = fs::read_to_string(dir.path().join("differences.csv")).unwrap();
    let rows: Vec<Vec<_>> = csv
        .lines()
        .skip(1)
        .map(|line| line.split(',').collect())
        .collect();
    assert_eq!(&rows[0][..5], ["0", "1", "100.000", "50.000", "0"]);
    assert!(rows[0][5].parse::<f64>().unwrap().abs() < f64::EPSILON);
    assert!(rows[0][6].parse::<f64>().unwrap().abs() < f64::EPSILON);
    assert_eq!(rows[1][4], "233");
    // Two non-overlapping 17x25 rectangles change between slots 0 and 4.
    let expected = 850.0 / (80.0 * 48.0);
    assert!((rows[1][6].parse::<f64>().unwrap() - expected).abs() < f64::EPSILON);
    assert!(
        report["difference_report"]
            .as_str()
            .unwrap()
            .contains("not automatic")
    );
}

#[test]
fn webp_odd_chunks_and_varied_sizes_round_trip_without_external_encoder() {
    for (width, height) in [(1, 1), (17, 11), (48, 7)] {
        let images = [
            RgbImage::from_fn(width, height, |x, y| {
                Rgb([
                    u8::try_from(x * 3).unwrap(),
                    u8::try_from(y * 17).unwrap(),
                    37,
                ])
            }),
            RgbImage::from_pixel(width, height, Rgb([125, 44, 8])),
        ];
        let mut bytes = Cursor::new(Vec::new());
        let mut encoder = super::super::webp::Animation::new(&mut bytes, (width, height)).unwrap();
        for (frame, delay) in images.iter().zip([1, 60_000]) {
            encoder.frame(frame, delay).unwrap();
        }
        encoder.finish().unwrap();
        let bytes = bytes.into_inner();
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8
        );
        let decoder = WebPDecoder::new(Cursor::new(bytes)).unwrap();
        let frames = decoder.into_frames().collect_frames().unwrap();
        assert_eq!(frames.len(), 2);
        for ((frame, expected), delay) in frames.into_iter().zip(images).zip([1, 60_000]) {
            assert_eq!(frame.delay().numer_denom_ms(), (delay, 1));
            assert_eq!(
                DynamicImage::ImageRgba8(frame.into_buffer()).into_rgb8(),
                expected
            );
        }
    }
}

#[test]
fn failed_output_preserves_previous_artifact_and_cleans_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("preview-real.webp");
    fs::write(&destination, b"previous").unwrap();
    assert!(
        data::output(dir.path(), "preview-real.webp", |file| {
            file.write_all(b"partial")?;
            anyhow::bail!("synthetic failure")
        })
        .is_err()
    );
    assert_eq!(fs::read(destination).unwrap(), b"previous");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn mp4_and_step_cadence_write_valid_isobmff_containers_without_external_encoder() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut options = preview();
    options.format = Format::All;
    let report = render::encode(dir.path(), &options, false).unwrap();
    assert_eq!(
        report["timing"]["mp4"]["duration_ms"],
        json!([100, 300, 100])
    );
    assert_eq!(report["timing"]["mp4"]["total_ms"], 500);
    for name in ["preview-real.mp4", "preview-step.mp4"] {
        let bytes = fs::read(dir.path().join(name)).unwrap();
        assert!(bytes.len() > 64, "{name}");
        assert_eq!(&bytes[4..8], b"ftyp", "{name}");
        let ftyp_len = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
        assert_eq!(&bytes[ftyp_len + 4..ftyp_len + 8], b"mdat", "{name}");
        let mdat_len =
            u32::from_be_bytes(bytes[ftyp_len..ftyp_len + 4].try_into().unwrap()) as usize;
        let moov_pos = ftyp_len + mdat_len;
        assert_eq!(&bytes[moov_pos + 4..moov_pos + 8], b"moov", "{name}");
        let moov_len =
            u32::from_be_bytes(bytes[moov_pos..moov_pos + 4].try_into().unwrap()) as usize;
        assert_eq!(moov_pos + moov_len, bytes.len(), "{name}");
    }

    for (width, height) in [(1, 1), (17, 11), (48, 7)] {
        let images = [
            RgbImage::from_fn(width, height, |x, y| {
                Rgb([
                    u8::try_from(x * 3).unwrap(),
                    u8::try_from(y * 17).unwrap(),
                    37,
                ])
            }),
            RgbImage::from_pixel(width, height, Rgb([125, 44, 8])),
            RgbImage::from_pixel(width, height, Rgb([125, 44, 8])),
        ];
        let mut bytes = Cursor::new(Vec::new());
        let mut encoder = super::super::mp4::Animation::new(&mut bytes, (width, height)).unwrap();
        for (frame, delay) in images.iter().zip([1, 500, 60_000]) {
            encoder.frame(frame, delay).unwrap();
        }
        encoder.finish().unwrap();
        let bytes = bytes.into_inner();
        assert_eq!(&bytes[4..8], b"ftyp");
    }
}
