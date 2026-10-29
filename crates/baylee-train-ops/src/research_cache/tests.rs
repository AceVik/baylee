use crate as ops;
mod support;

use super::*;
use serde_json::json;
use support::Fixture;

#[test]
fn lossless_random_reads_preserve_all_history_and_separate_denominators() {
    let fixture = Fixture::new(false);
    let result = build(&fixture.converted, &fixture.cache, &fixture.expected).unwrap();
    assert_eq!(result["frames"], 10);
    let mut reader = Reader::open(&fixture.cache, &fixture.expected).unwrap();
    assert_eq!(reader.len(), fixture.raw.len());
    assert_eq!(
        reader.metadata()["attempt_counts"],
        json!({"planned":5,"assigned":4,"recorded":3,"operational_unknown":1,"not_started":1,"recorded_unknown":1,"target_episodes":2})
    );
    for row in (0..reader.len()).rev() {
        assert_eq!(reader.read_raw(row).unwrap(), fixture.raw[row]);
    }
    assert!(reader.read_raw(reader.len()).is_err());
    let row: Value = serde_json::from_slice(&reader.read_raw(1).unwrap()).unwrap();
    assert_eq!(row["features"]["target"]["pick_sequence"], json!([1, 0]));
    assert!(row["returns"]["mtg_outcome"].is_null());
    assert_eq!(row["returns"]["training_utility"], 0.0);
    assert_eq!(reader.metadata()["histories"][0]["actor_candidates"], 2);
    assert_eq!(reader.metadata()["histories"][0]["genuine_choices"], 1);
    assert_eq!(reader.metadata()["tensor_cache"], false);
    assert_eq!(reader.metadata()["admitted"], false);
    assert_eq!(
        fs::read(fixture.cache.join("attempts.jsonl")).unwrap(),
        fs::read(fixture.converted.join("attempts.jsonl")).unwrap()
    );
    assert!(!fixture.cache.join("meta.json").exists());
    assert!(crate::trajectory::cache::build(&fixture.cache, "train", &[]).is_err());
    assert!(crate::trajectory::contracts::reject_policy_gradient(reader.metadata()).is_err());
    assert!(build(&fixture.converted, &fixture.cache, &fixture.expected).is_err());
}

#[test]
fn zero_target_cache_retains_operational_unknown_and_unstarted_attempts() {
    let fixture = Fixture::new(true);
    let result = build(&fixture.converted, &fixture.cache, &fixture.expected).unwrap();
    assert_eq!(result["frames"], 0);
    assert_eq!(result["attempt_counts"]["planned"], 5);
    assert_eq!(result["attempt_counts"]["operational_unknown"], 4);
    assert_eq!(result["attempt_counts"]["not_started"], 1);
    assert!(
        Reader::open(&fixture.cache, &fixture.expected)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::metadata(fixture.cache.join("frames.brc1"))
            .unwrap()
            .len(),
        8
    );
}

#[test]
fn changed_external_sources_completion_catalog_and_expectation_fail_closed() {
    let fixture = Fixture::new(false);
    assert!(build(&fixture.converted, &fixture.cache, &"0".repeat(64)).is_err());
    assert!(!fixture.cache.exists());
    build(&fixture.converted, &fixture.cache, &fixture.expected).unwrap();
    for path in [
        fixture.converted.join("research.complete.json"),
        fixture.converted.join("card_dsl.bin"),
        fixture.source.join("attempts.complete.json"),
        fixture.source.join("records/orphan.partial"),
        fixture.source.parent().unwrap().join("job.json"),
    ] {
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"changed").unwrap();
        assert!(
            Reader::open(&fixture.cache, &fixture.expected).is_err(),
            "{}",
            path.display()
        );
        fs::write(&path, original).unwrap();
    }
    verify(&fixture.cache, &fixture.expected).unwrap();
}

fn reseal_cache(root: &Path) {
    let path = root.join("research-cache.manifest.json");
    let mut manifest = read_json(&path).unwrap();
    let outputs = manifest["output_files"].as_object_mut().unwrap();
    for (name, value) in outputs {
        *value = json!(files::sha256(&root.join(name)).unwrap());
    }
    manifest
        .as_object_mut()
        .unwrap()
        .remove("content_fingerprint");
    manifest["content_fingerprint"] = json!(json_digest(&manifest).unwrap());
    files::atomic_json(&path, &manifest).unwrap();
    files::atomic_json(&root.join("research-cache.complete.json"), &json!({"schema":CACHE_SCHEMA,
        "manifest_sha256":files::sha256(&path).unwrap(),"content_fingerprint":manifest["content_fingerprint"],
        "expected_conversion_manifest_sha256":manifest["expected_conversion_manifest_sha256"],"admitted":false})).unwrap();
}

#[test]
fn cache_and_resealed_index_tampering_never_rebuild_or_change_history() {
    let fixture = Fixture::new(false);
    build(&fixture.converted, &fixture.cache, &fixture.expected).unwrap();
    let path = fixture.cache.join("frames.bri1");
    let original = fs::read(&path).unwrap();
    for offset in [8_usize, 8 + 24, 8 + 36, 8 + 40, 8 + 44] {
        let mut changed = original.clone();
        changed[offset] ^= 255;
        fs::write(&path, &changed).unwrap();
        reseal_cache(&fixture.cache);
        assert!(
            Reader::open(&fixture.cache, &fixture.expected).is_err(),
            "offset {offset}"
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            changed,
            "verification may not repair evidence"
        );
        fs::write(&path, &original).unwrap();
        reseal_cache(&fixture.cache);
    }
    let mut reader = Reader::open(&fixture.cache, &fixture.expected).unwrap();
    let data = fixture.cache.join("frames.brc1");
    let mut bytes = fs::read(&data).unwrap();
    bytes[10] ^= 255;
    fs::write(&data, bytes).unwrap();
    assert!(reader.read_raw(0).is_err());
    assert!(verify(&fixture.cache, &fixture.expected).is_err());
}

#[test]
fn malformed_cursors_returns_labels_masks_and_scalar_probabilities_are_refused() {
    let fixture = Fixture::new(false);
    let source = source::Source::load(&fixture.converted, &fixture.expected).unwrap();
    let history = &source.histories[0];
    let roots = std::collections::BTreeMap::new();
    for mutation in [
        "cursor", "return", "legacy", "mask", "scalar", "terminal", "width",
    ] {
        let mut row: Value = serde_json::from_slice(&fixture.raw[1]).unwrap();
        match mutation {
            "cursor" => row["features"]["step_cursor"] = json!(2),
            "return" => row["returns"]["mtg_outcome"] = json!({"kind":"draw"}),
            "legacy" => row["features"]["outcome"] = json!(0.0),
            "mask" => row["features"]["target"]["free_mask"] = json!([]),
            "scalar" => row["features"]["head"] = json!(3),
            "terminal" => row["training_terminal"] = json!(true),
            "width" => row["features"]["state"]["global_feat"] = json!([0]),
            _ => unreachable!(),
        }
        assert!(
            rows::validate(&serde_json::to_vec(&row).unwrap(), history, 7, 1, &roots).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn resealed_valid_payload_cannot_replace_original_behavior_probability() {
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write as _;
    let fixture = Fixture::new(false);
    build(&fixture.converted, &fixture.cache, &fixture.expected).unwrap();
    let mut index = fs::read(fixture.cache.join("frames.bri1")).unwrap();
    let mut data = DATA_MAGIC.to_vec();
    for (row, original) in fixture.raw.iter().enumerate() {
        let mut raw = original.clone();
        if row == 1 {
            let mut value: Value = serde_json::from_slice(&raw).unwrap();
            value["features"]["provenance"]["behavior_logprob"] = json!(-8.0);
            raw = serde_json::to_vec(&value).unwrap();
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();
        let start = 8 + row * 48;
        index[start + 24..start + 32]
            .copy_from_slice(&u64::try_from(data.len()).unwrap().to_le_bytes());
        index[start + 32..start + 36]
            .copy_from_slice(&u32::try_from(compressed.len()).unwrap().to_le_bytes());
        index[start + 36..start + 40]
            .copy_from_slice(&u32::try_from(raw.len()).unwrap().to_le_bytes());
        data.extend(compressed);
    }
    fs::write(fixture.cache.join("frames.brc1"), data).unwrap();
    fs::write(fixture.cache.join("frames.bri1"), index).unwrap();
    reseal_cache(&fixture.cache);
    let Err(error) = Reader::open(&fixture.cache, &fixture.expected) else {
        panic!("resealed relabelled LP admitted");
    };
    assert!(
        format!("{error:#}").contains("original frame bytes"),
        "{error:#}"
    );
}
