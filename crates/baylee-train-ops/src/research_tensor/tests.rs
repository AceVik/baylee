use super::*;
use crate::research_cache::tests::support;
use serde_json::json;
use std::{
    fs,
    io::{Cursor, Read},
};

fn cached(zero: bool) -> support::Fixture {
    let fixture = support::Fixture::new(zero);
    crate::research_cache::build(&fixture.converted, &fixture.cache, &fixture.expected).unwrap();
    fixture
}
fn open(f: &support::Fixture) -> Session {
    Session::open(
        &f.cache,
        &f.expected,
        &crate::files::sha256(&f.cache.join("research-cache.manifest.json")).unwrap(),
    )
    .unwrap()
}
fn field<'a>(batch: &'a Batch, name: &str) -> (&'a Value, &'a [u8]) {
    let a = batch.descriptor["arrays"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    let start = a["offset"].as_u64().unwrap() as usize;
    let len = a["nbytes"].as_u64().unwrap() as usize;
    (a, &batch.payload[start..start + len])
}
fn pins(f: &support::Fixture) -> (std::path::PathBuf, String) {
    let cache = crate::files::read_json(&f.cache.join("research-cache.manifest.json")).unwrap();
    let c = &cache["source_conversion_manifest"];
    let source = &c["source_runs"][0];
    let ledger = &source["finalized_attempt_ledger"];
    let job = crate::files::read_json(Path::new(ledger["guard_job"].as_str().unwrap())).unwrap();
    let path = f.source.parent().unwrap().join("pins.json");
    crate::files::atomic_json(&path,&json!({"schema":"failure_terminal_research_input_pins_v1","cache_dir":f.cache,
        "conversion_manifest_sha256":f.expected,"cache_manifest_sha256":crate::files::sha256(&f.cache.join("research-cache.manifest.json")).unwrap(),
        "source_checkpoint":{"path":f.source.parent().unwrap().join("source.pt"),"sha256":c["learner_model_identity"]["source_checkpoint_sha256"]},
        "learner_model_identity":c["learner_model_identity"],"catalog":c["catalog"],"producers":[{"source_run_sha256":source["source_run_sha256"],
        "guard_job":ledger["guard_job"],"guard_job_sha256":ledger["guard_job_sha256"],"executable":job["executable"],"executable_sha256":job["executable_sha256"],"collection_contract":source["collection_contract"]}]})).unwrap();
    let digest = crate::files::sha256(&path).unwrap();
    (path, digest)
}

#[test]
fn recurrent_arrays_keep_passive_scalar_pick_order_and_research_return() {
    let f = cached(false);
    let mut session = open(&f);
    assert_eq!(session.metadata()["attempt_counts"]["planned"], 5);
    assert_eq!(session.metadata()["genuine_choices"], 2);
    assert_eq!(session.metadata()["actor_candidates"], 4);
    assert_eq!(session.metadata()["research_input_admitted"], false);
    let batch = session.batch(0, 0, 5).unwrap();
    assert_eq!(field(&batch, "is_decision").1, [0, 1, 1, 1, 0]);
    assert_eq!(field(&batch, "has_logprob").1, [0, 1, 1, 0, 0]);
    assert_eq!(field(&batch, "actor_candidate").1, [0, 1, 1, 0, 0]);
    assert_eq!(field(&batch, "actor_mask").1, [0, 0, 1, 0, 0]);
    assert_eq!(field(&batch, "training_terminal").1, [0, 0, 0, 0, 1]);
    assert_eq!(field(&batch, "cand_mask").1, [0, 0, 1, 1, 1, 1, 0, 0, 0, 0]);
    assert_eq!(field(&batch, "pair_mask").1, [0, 0, 0, 0, 0]);
    assert_eq!(field(&batch, "training_utility").1, [0; 20]);
    assert_eq!(
        batch.descriptor["identity"]["returns"]["mtg_outcome"],
        Value::Null
    );
    assert_eq!(field(&batch, "pick_sequence").0["shape"], json!([5, 2]));
    assert_eq!(
        field(&batch, "pick_sequence").1[16..32],
        [1_i64.to_le_bytes(), 0_i64.to_le_bytes()].concat()
    );
    assert_eq!(
        field(&batch, "pick_sequence_mask").1,
        [0, 0, 1, 1, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(field(&batch, "cand_free_mask").1, [0; 40]);
    for name in ["outcome", "aux", "is_terminal"] {
        assert!(
            !batch.descriptor["arrays"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["name"] == name)
        );
    }
    assert!(session.batch(0, 4, 2).is_err());
    assert!(session.batch(0, 0, 0).is_err());
    assert!(session.batch(2, 0, 1).is_err());
    let natural = session.batch(1, 0, 5).unwrap();
    assert_eq!(
        natural.descriptor["identity"]["returns"]["training_utility"],
        1.0
    );
}

#[test]
fn native_input_admission_is_separate_bound_and_requires_original_producer_evidence() {
    let f = cached(false);
    let (path, digest) = pins(&f);
    let out = f.source.parent().unwrap().join("admission.json");
    let receipt = admission::admit(&path, &digest, &out).unwrap();
    assert_eq!(receipt["frames"], 10);
    assert_eq!(receipt["genuine_choices"], 2);
    assert_eq!(receipt["learner_histories"], 2);
    assert_eq!(receipt["ordinary_ppo_admitted"], false);
    assert_eq!(receipt["numerical_replay_required"], true);
    assert_eq!(
        admission::open(&path, &digest, &out).unwrap().metadata()["research_input_admitted"],
        true
    );
    assert!(admission::admit(&path, &digest, &out).is_err());
    assert!(admission::verify(&path, &"0".repeat(64), &out).is_err());
    let mut changed = receipt.clone();
    changed["numerical_replay_required"] = json!(false);
    crate::files::atomic_json(&out, &changed).unwrap();
    assert!(admission::verify(&path, &digest, &out).is_err());
    crate::files::atomic_json(&out, &receipt).unwrap();
    fs::write(
        f.source.parent().unwrap().join("producer-binary"),
        b"changed executable",
    )
    .unwrap();
    assert!(admission::verify(&path, &digest, &out).is_err());
}

#[test]
fn pinned_checkpoint_catalog_roles_and_source_completion_refuse_tampering() {
    let f = cached(false);
    let (path, digest) = pins(&f);
    let original = fs::read(&path).unwrap();
    for key in ["learner_model_identity", "catalog"] {
        let mut changed: Value = serde_json::from_slice(&original).unwrap();
        changed[key] = Value::Null;
        crate::files::atomic_json(&path, &changed).unwrap();
        let sha = crate::files::sha256(&path).unwrap();
        assert!(
            admission::admit(&path, &sha, &f.source.parent().unwrap().join("never.json")).is_err()
        );
    }
    fs::write(&path, &original).unwrap();
    assert!(Session::open(&f.cache, &f.expected, &"0".repeat(64)).is_err());
    fs::remove_file(f.source.join("attempts.complete.json")).unwrap();
    assert!(
        admission::admit(
            &path,
            &digest,
            &f.source.parent().unwrap().join("never.json")
        )
        .is_err()
    );
}

#[test]
fn zero_target_session_preserves_all_denominators_and_cannot_make_rows() {
    let f = cached(true);
    let mut s = open(&f);
    assert_eq!(s.metadata()["frames"], 0);
    assert_eq!(s.metadata()["attempt_counts"]["operational_unknown"], 4);
    assert_eq!(s.metadata()["attempt_counts"]["not_started"], 1);
    assert!(s.batch(0, 0, 1).is_err());
}

#[test]
fn long_lived_wire_session_produces_multiple_bounded_binary_responses() {
    let f = cached(false);
    let mut wire = Vec::new();
    serve(
        open(&f),
        Cursor::new(
            b"{\"history\":0,\"start\":0,\"length\":2}\n{\"history\":0,\"start\":2,\"length\":3}\n",
        ),
        &mut wire,
    )
    .unwrap();
    let newline = wire.iter().position(|b| *b == b'\n').unwrap();
    let handshake: Value = serde_json::from_slice(&wire[..newline]).unwrap();
    assert_eq!(handshake["frames"], 10);
    let mut cursor = Cursor::new(&wire[newline + 1..]);
    for (start, length) in [(0, 2), (2, 3)] {
        let mut n = [0; 8];
        cursor.read_exact(&mut n).unwrap();
        let mut descriptor = vec![0; u64::from_le_bytes(n) as usize];
        cursor.read_exact(&mut descriptor).unwrap();
        let descriptor: Value = serde_json::from_slice(&descriptor).unwrap();
        assert_eq!(descriptor["identity"]["start"], start);
        assert_eq!(descriptor["identity"]["length"], length);
        cursor.read_exact(&mut n).unwrap();
        let mut payload = vec![0; u64::from_le_bytes(n) as usize];
        cursor.read_exact(&mut payload).unwrap();
        assert_eq!(descriptor["payload_bytes"], payload.len());
    }
    assert_eq!(cursor.position() as usize, wire.len() - newline - 1);
    assert!(
        serve(
            open(&f),
            Cursor::new(b"{\"history\":0,\"start\":0,\"length\":2,\"drop_passive\":true}\n"),
            Vec::new()
        )
        .is_err()
    );
}

#[test]
fn strict_tensor_packer_rejects_nonfinite_overflow_dangling_edges_and_missing_masks() {
    let f = cached(false);
    let reader = crate::research_cache::Reader::open(&f.cache, &f.expected).unwrap();
    let cat = catalog::Catalog::load(&f.cache, reader.metadata()).unwrap();
    assert!(cat.card(1, 2).is_err());
    assert_eq!(cat.entity(1, 0, 1).unwrap(), cat.card(1, 0).unwrap());
    let original: Value = serde_json::from_slice(&f.raw[1]).unwrap();
    for (key, value) in [
        ("cand_entity_idx", json!([0, -1])),
        ("cand_rel_seat_idx", json!([8, -1])),
    ] {
        let mut row = original.clone();
        row["features"][key] = value;
        assert!(frame::pack(&row, &cat).is_err());
    }
    let mut row = original.clone();
    row["features"]["state"]["edges"] = json!([[0, 10, 1]]);
    assert!(frame::pack(&row, &cat).is_err());
    let mut row = original.clone();
    row["features"]["target"]["free_mask"] = json!([]);
    assert!(frame::pack(&row, &cat).is_err());
    let mut row = original;
    row["features"]["cand_feat"][0][0] = json!(1e100);
    assert!(frame::pack(&row, &cat).is_err());
}
