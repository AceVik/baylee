use super::*;
use std::fs::File;

#[test]
fn incomplete_runs_require_explicit_recovery() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    fs::write(
        dir.path().join("recording.json"),
        r#"{"schema":1,"phase":"starting"}"#,
    )
    .unwrap();
    assert!(data::load(dir.path(), false).is_err());
    assert_eq!(data::load(dir.path(), true).unwrap().frames.len(), 3);
}

#[test]
fn corrupted_ledger_recovers_only_newline_committed_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let rows = fixture(dir.path());
    let prefix = format!("{}\n", rows[0]).into_bytes();
    let suffixes = [
        b"{\"slot\":".to_vec(),
        b"{\"slot\":\"\xff".to_vec(),
        rows[1].to_string().into_bytes(),
        format!("{{\"slot\":broken}}\n{}\n", rows[2]).into_bytes(),
        format!("{{\"slot\":{}\n", rows[2]).into_bytes(),
    ];
    for suffix in suffixes {
        let original = [prefix.as_slice(), suffix.as_slice()].concat();
        let ledger = dir.path().join("frames.jsonl");
        fs::write(&ledger, &original).unwrap();
        assert!(data::load(dir.path(), false).is_err());
        let recovered = data::load(dir.path(), true).unwrap();
        assert_eq!(
            recovered
                .frames
                .iter()
                .map(|frame| frame.slot)
                .collect::<Vec<_>>(),
            [0]
        );
        assert_eq!(
            recovered.summary["ledger_recovery"]["ignored_bytes"],
            json!(suffix.len())
        );
        assert_eq!(recovered.summary["ledger_recovery"]["valid_rows"], 1);
        let report = render::encode(dir.path(), &preview(), true).unwrap();
        assert_eq!(
            report["ledger_recovery"],
            recovered.summary["ledger_recovery"]
        );
        assert_eq!(fs::read(&ledger).unwrap(), original);
    }
}

#[test]
fn semantic_corruption_is_not_silently_skipped_in_recovery_mode() {
    let dir = tempfile::tempdir().unwrap();
    let rows = fixture(dir.path());
    for (field, value) in [
        ("file", json!("../escape.png")),
        ("slot", json!(900)),
        ("requested_us", json!(-1)),
        ("received_us", json!(1.25)),
        ("virtual_us", json!(true)),
        ("width", json!(0)),
        ("height", json!(u32::MAX)),
    ] {
        let mut invalid = rows[0].clone();
        invalid[field] = value;
        write_rows(dir.path(), [&invalid]);
        for recover in [false, true] {
            assert!(data::load(dir.path(), recover).is_err(), "accepted {field}");
        }
    }
    write_rows(dir.path(), [&rows[0], &rows[0]]);
    assert!(data::load(dir.path(), true).is_err());
    fs::write(dir.path().join("frames.jsonl"), []).unwrap();
    assert!(data::load(dir.path(), true).is_err());
}

#[test]
fn metadata_ledger_and_png_sizes_are_bounded() {
    for (name, limit) in [
        ("recording.json", 1024 * 1024),
        ("frames.jsonl", 2 * 1024 * 1024),
    ] {
        let dir = tempfile::tempdir().unwrap();
        fixture(dir.path());
        File::create(dir.path().join(name))
            .unwrap()
            .set_len(limit + 1)
            .unwrap();
        assert!(data::load(dir.path(), true).is_err());
    }
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let recording = data::load(dir.path(), false).unwrap();
    File::create(dir.path().join("frame-000000.png"))
        .unwrap()
        .set_len(128 * 1024 * 1024 + 1)
        .unwrap();
    assert!(data::read_frame(&recording, &recording.frames[0]).is_err());
}

#[test]
fn preview_budget_rejects_before_decoding_or_writing_and_dimensions_must_match() {
    let dir = tempfile::tempdir().unwrap();
    let mut rows = fixture(dir.path());
    for row in &mut rows {
        row["width"] = json!(1);
        row["height"] = json!(8 * 1024 * 1024);
    }
    // Nine tall frames exceed the aggregate preview budget even at width 16.
    let many: Vec<_> = (0..9)
        .map(|slot| {
            let mut row = rows[0].clone();
            row["slot"] = json!(slot);
            row["file"] = json!(format!("frame-{slot:06}.png"));
            fs::write(dir.path().join(row["file"].as_str().unwrap()), b"not a PNG").unwrap();
            row
        })
        .collect();
    write_rows(dir.path(), &many);
    let error = render::encode(dir.path(), &preview(), false).unwrap_err();
    assert!(error.to_string().contains("smaller --width"));
    assert!(!dir.path().join("preview-real.webp").exists());
    fixture(dir.path());
    let recording = data::load(dir.path(), false).unwrap();
    RgbImage::new(81, 48)
        .save(dir.path().join("frame-000000.png"))
        .unwrap();
    assert!(data::read_frame(&recording, &recording.frames[0]).is_err());
}

#[cfg(unix)]
#[test]
fn input_symlinks_cannot_escape_and_outputs_do_not_follow_symlinks() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let target = outside.path().join("private.png");
    fs::copy(dir.path().join("frame-000000.png"), &target).unwrap();
    fs::remove_file(dir.path().join("frame-000000.png")).unwrap();
    symlink(&target, dir.path().join("frame-000000.png")).unwrap();
    assert!(data::load(dir.path(), true).is_err());
    let untouched = fs::read(&target).unwrap();
    symlink(&target, dir.path().join("preview-real.webp")).unwrap();
    data::output(dir.path(), "preview-real.webp", |file| {
        use std::io::Write;
        file.write_all(b"new")?;
        Ok(())
    })
    .unwrap();
    assert_eq!(fs::read(target).unwrap(), untouched);
    assert_eq!(
        fs::read(dir.path().join("preview-real.webp")).unwrap(),
        b"new"
    );
}
