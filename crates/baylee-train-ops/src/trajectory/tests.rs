use super::*;
use crate::files;
use anyhow::Result;
use flate2::{
    Compression,
    read::ZlibDecoder,
    write::{GzEncoder, ZlibEncoder},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
};
fn frame() -> Value {
    json!({"game_id":7,"seat":1,"step_cursor":0,"turn":2,"head":1,"pending_kind":3,"is_decision":true,"is_terminal":false,"loss_weight":0.75,"outcome":0.5,
    "provenance":{"source":"External","has_logprob":true,"thinking_steps":2,"temperature":0.7,"behavior_logprob":-1.0},
    "state":{"entity_feat":[[0.25,-0.5]],"entity_card_ids":[17],"entity_face_ids":[1],"entity_token_ids":[256],"entity_rule_refs":[[{"key":"public","role":7}]],"seat_feat":vec![vec![1.0,0.0];8],"global_feat":[0.25,0.5],"edges":[[0,0,2]]},
    "cand_feat":[[1.0,2.0],[3.0,4.0]],"cand_entity_idx":[0,-1],"cand_rel_seat_idx":[-1,1],"cand_card_id":[17,18],"cand_face_id":[0,255],"cand_rule_refs":[[],[{"key":"public","role":1}]],"pairs":[[0,1]],"target":{"single_idx":1,"scalar_norm":0.25,"multi_mask":[1,0],"free_mask":[1,1]}})
}
#[test]
fn encoder9_metadata_cannot_relabel_legacy_tensor_rows() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir(root.join("train"))?;
    let meta = json!({"encoder_version":9,"global_feat_width":82,"train_shards":["e9.gz"],"val_shards":[]});
    for width in [80, 82] {
        let mut step = frame();
        step["state"]["global_feat"] = json!(vec![0.0; width]);
        let packed = packing::pack_step(&step, &BTreeMap::from([("public".into(), 11)]))?;
        assert_eq!(packing::metadata(&packed)?["global_feat_width"], width);
        let mut stream = GzEncoder::new(
            File::create(root.join("train/e9.gz"))?,
            Compression::default(),
        );
        serde_json::to_writer(&mut stream, &step)?;
        stream.write_all(b"\n")?;
        stream.finish()?;
        assert_eq!(
            contracts::validate_encoder_rows(root, &meta).is_ok(),
            width == 82
        );
    }
    let mut missing = meta.clone();
    missing.as_object_mut().unwrap().remove("global_feat_width");
    assert!(contracts::validate_encoder_rows(root, &missing).is_err());
    let legacy = json!({"encoder_version":8,"train_shards":["e9.gz"]});
    assert!(contracts::validate_encoder_rows(root, &legacy).is_err());
    files::atomic_json(&root.join("meta.json"), &meta)?;
    files::atomic_json(
        &root.join("card_structure.json"),
        &json!({"roots":{"public":11}}),
    )?;
    let receipt = cache::build(root, "train", &["e9.gz".into()])?;
    let data = std::fs::read(receipt.data_path)?;
    let mut binary = b"BS4S\x01\x00\x00\x00".to_vec();
    binary.extend_from_slice(&data);
    binary.extend_from_slice(&std::fs::read(receipt.index_path)?);
    binary.extend_from_slice(&(8 + data.len() as u64).to_le_bytes());
    binary.extend_from_slice(&1u64.to_le_bytes());
    binary.extend_from_slice(b"BS4S_END");
    std::fs::write(root.join("train/e9.bst4"), binary)?;
    let mut packed_meta = meta.clone();
    packed_meta["train_shards"] = json!(["e9.bst4"]);
    contracts::validate_encoder_rows(root, &packed_meta)?;
    packed_meta["encoder_version"] = json!(8);
    packed_meta
        .as_object_mut()
        .unwrap()
        .remove("global_feat_width");
    assert!(contracts::validate_encoder_rows(root, &packed_meta).is_err());
    Ok(())
}
#[test]
fn bst4_layout_and_admission_metadata_match_the_fixed_python_abi() -> Result<()> {
    let raw = packing::pack_step(&frame(), &BTreeMap::from([("public".into(), 11)]))?;
    assert_eq!(raw.len(), 299);
    assert_eq!(&raw[..8], b"BST4\x01\x00\x00\x00");
    assert_eq!(&raw[136..144], &[0, 0, 128, 62, 0, 0, 0, 191]);
    assert_eq!(&raw[144..151], &[17, 0, 0, 0, 1, 0, 1]);
    let meta = packing::metadata(&raw)?;
    assert_eq!(meta["game_id"], 7);
    assert_eq!(meta["seat"], 1);
    assert_eq!(meta["provenance"]["source"], "External");
    assert_eq!(meta["provenance"]["has_logprob"], true);
    assert_eq!(meta["target"]["multi_mask"], json!([1, 0]));
    assert_eq!(meta["target"]["free_mask"], json!([1, 1]));
    assert_eq!(meta["cand_feat"].as_array().unwrap().len(), 2);
    for broken in [&raw[..135], &raw[..298]] {
        assert!(packing::metadata(broken).is_err());
    }
    let mut invalid = raw;
    invalid[88..92].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(packing::metadata(&invalid).is_err());
    Ok(())
}
#[test]
fn empty_cache_is_atomic_reusable_and_has_the_archived_metadata_fingerprint() -> Result<()> {
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("meta.json"), b"{}")?;
    let a = cache::build(directory.path(), "train", &[])?;
    assert_eq!(
        a.fingerprint,
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
    assert_eq!(a.records, 0);
    assert_ne!(a.data_path, a.index_path);
    let before = a.data_path.metadata()?.modified()?;
    let b = cache::build(directory.path(), "train", &[])?;
    assert_eq!(a.fingerprint, b.fingerprint);
    assert_eq!(before, b.data_path.metadata()?.modified()?);
    Ok(())
}
#[test]
fn gzip_and_binary_shards_share_one_native_reader_and_cache_abi() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir(root.join("train"))?;
    std::fs::write(root.join("meta.json"), b"{}")?;
    files::atomic_json(
        &root.join("card_structure.json"),
        &json!({"roots":{"public":11}}),
    )?;
    let step = frame();
    let roots = BTreeMap::from([("public".into(), 11)]);
    let packed = packing::pack_step(&step, &roots)?;
    let mut gz = GzEncoder::new(
        File::create(root.join("train/a.gz"))?,
        Compression::default(),
    );
    serde_json::to_writer(&mut gz, &step)?;
    gz.write_all(b"\n")?;
    gz.finish()?;
    let mut seen = Vec::new();
    assert_eq!(
        read_shard(&root.join("train/a.gz"), |v| {
            seen.push(v);
            Ok(())
        })?,
        1
    );
    assert_eq!(seen, [step]);
    let mut compressed = ZlibEncoder::new(Vec::new(), Compression::fast());
    compressed.write_all(&packed)?;
    let compressed = compressed.finish()?;
    let mut binary = b"BS4S\x01\x00\x00\x00".to_vec();
    binary.extend_from_slice(&compressed);
    let mut index = Vec::new();
    index.extend_from_slice(&7u32.to_le_bytes());
    index.extend_from_slice(&1u32.to_le_bytes());
    index.extend_from_slice(&0u64.to_le_bytes());
    index.extend_from_slice(&0u64.to_le_bytes());
    index.extend_from_slice(&u32::try_from(compressed.len())?.to_le_bytes());
    index.extend_from_slice(&1u32.to_le_bytes());
    binary.extend_from_slice(&index);
    binary.extend_from_slice(&(8 + compressed.len() as u64).to_le_bytes());
    binary.extend_from_slice(&1u64.to_le_bytes());
    binary.extend_from_slice(b"BS4S_END");
    std::fs::write(root.join("train/b.bst4"), &binary)?;
    let mut seen = Vec::new();
    assert_eq!(
        read_shard(&root.join("train/b.bst4"), |v| {
            seen.push(v);
            Ok(())
        })?,
        1
    );
    assert_eq!(seen[0], packing::metadata(&packed)?);
    let receipt = cache::build(root, "train", &["a.gz".into(), "b.bst4".into()])?;
    assert_eq!(receipt.records, 2);
    let idx = std::fs::read(&receipt.index_path)?;
    let data = std::fs::read(&receipt.data_path)?;
    for row in idx.as_chunks::<32>().0 {
        let offset = usize::try_from(u64::from_le_bytes(row[16..24].try_into()?))?;
        let length = u32::from_le_bytes(row[24..28].try_into()?) as usize;
        let mut raw = Vec::new();
        ZlibDecoder::new(&data[offset..offset + length]).read_to_end(&mut raw)?;
        assert_eq!(raw, packed);
    }
    binary[8 + compressed.len()..12 + compressed.len()].copy_from_slice(&999u32.to_le_bytes());
    std::fs::write(root.join("train/b.bst4"), binary)?;
    assert!(read_shard(&root.join("train/b.bst4"), |_| Ok(())).is_err());
    Ok(())
}
#[test]
fn cache_content_seals_rebuild_layout_valid_outcome_action_and_index_tampering() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir(root.join("train"))?;
    std::fs::write(root.join("meta.json"), b"{}")?;
    let roots = BTreeMap::from([("public".to_owned(), 11)]);
    files::atomic_json(&root.join("card_structure.json"), &json!({"roots":roots}))?;
    let source = root.join("train/a.gz");
    let original = frame();
    let mut gzip = GzEncoder::new(File::create(&source)?, Compression::default());
    serde_json::to_writer(&mut gzip, &original)?;
    gzip.write_all(b"\n")?;
    gzip.finish()?;
    let shards = ["a.gz".into()];
    let receipt = cache::build(root, "train", &shards)?;
    let data = std::fs::read(&receipt.data_path)?;
    let index = std::fs::read(&receipt.index_path)?;
    for (key, value) in [
        ("outcome", json!(-0.5)),
        (
            "target",
            json!({"single_idx":0,"scalar_norm":0.75,"multi_mask":[0,1],"free_mask":[1,1]}),
        ),
    ] {
        let mut tampered = original.clone();
        tampered[key] = value;
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        let packed = packing::pack_step(&tampered, &roots)?;
        packing::metadata(&packed)?;
        encoder.write_all(&packed)?;
        let bytes = encoder.finish()?;
        let mut changed_index = index.clone();
        changed_index[24..28].copy_from_slice(&u32::try_from(bytes.len())?.to_le_bytes());
        std::fs::write(&receipt.data_path, bytes)?;
        std::fs::write(&receipt.index_path, changed_index)?;
        let rebuilt = cache::build(root, "train", &shards)?;
        assert_eq!(std::fs::read(rebuilt.data_path)?, data);
        assert_eq!(std::fs::read(rebuilt.index_path)?, index);
    }
    let mut changed_index = index.clone();
    changed_index[..4].copy_from_slice(&99u32.to_le_bytes());
    std::fs::write(&receipt.index_path, changed_index)?;
    cache::build(root, "train", &shards)?;
    assert_eq!(std::fs::read(&receipt.index_path)?, index);
    let marker = receipt.data_path.with_extension("json");
    let mut legacy = files::read_json(&marker)?;
    legacy.as_object_mut().unwrap().remove("data_sha256");
    files::atomic_json(&marker, &legacy)?;
    cache::build(root, "train", &shards)?;
    let sealed = files::read_json(&marker)?;
    assert_eq!(sealed["data_sha256"], files::sha256(&receipt.data_path)?);
    assert_eq!(sealed["index_sha256"], files::sha256(&receipt.index_path)?);
    assert_eq!(
        sealed["source_sha256"][source.to_str().unwrap()],
        files::sha256(&source)?
    );
    Ok(())
}
#[test]
fn teacher_markers_are_refused_before_ppo_and_incomplete_contracts_never_build() -> Result<()> {
    let root = tempfile::tempdir()?;
    for value in [
        json!({"shadow_bc_version":1}),
        json!({"llm_bc_version":1}),
        json!({"ppo_eligible":false}),
    ] {
        assert!(contracts::reject_policy_gradient(&value).is_err());
    }
    for value in [json!({"shadow_bc_version":1}), json!({"llm_bc_version":1})] {
        files::atomic_json(&root.path().join("meta.json"), &value)?;
        assert!(cache::build(root.path(), "train", &[]).is_err());
        assert!(!root.path().join(".trajectory-cache-v2").exists());
    }
    contracts::reject_policy_gradient(&json!({"policy_provenance_version":2}))?;
    Ok(())
}
