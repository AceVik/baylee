use super::*;

struct Fixture {
    root: PathBuf,
    model: Selected,
    manifest: Value,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("baylee-seat4-selection-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&root).unwrap();
        let metadata = serde_json::json!({"encoder_version":6,"model_version":3,"trajectory_version":2,"sampling_version":2,"structure_sha256":"catalog","structure_version":1,"dsl_version":2});
        std::fs::write(
            root.join("model.onnx.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
        for name in [
            "net.pt",
            "model.onnx",
            "model.k0.onnx",
            "model.k1.onnx",
            "card_structure.json",
        ] {
            std::fs::write(root.join(name), name).unwrap();
        }
        let hashes: BTreeMap<_, _> = std::fs::read_dir(&root)
            .unwrap()
            .map(|file| {
                let file = file.unwrap();
                (
                    file.file_name().to_string_lossy().into_owned(),
                    digest(&file.path()).unwrap(),
                )
            })
            .collect();
        let model = Selected::fixed(&root).unwrap();
        let manifest = serde_json::json!({"schema_version":1,"model":root,"label":"R02","score":0.2,"benchmark_id":"suite","sha256":hashes});
        let path = root.join("selection.json");
        Self {
            root,
            model,
            manifest,
            path,
        }
    }
    fn write(&self) {
        std::fs::write(&self.path, serde_json::to_vec(&self.manifest).unwrap()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn selection_accepts_initial_and_only_strictly_better_same_benchmark() {
    let mut f = Fixture::new();
    f.write();
    let current = proposal(&f.path, &f.model, 1).unwrap();
    assert_eq!(current.label, "R02");
    assert!(proposal(&f.path, &current, 1).is_err());
    f.manifest["score"] = serde_json::json!(0.1);
    f.write();
    assert!(proposal(&f.path, &current, 1).is_err());
    f.manifest["score"] = serde_json::json!(0.3);
    f.manifest["benchmark_id"] = serde_json::json!("other");
    f.write();
    assert!(proposal(&f.path, &current, 1).is_err());
    f.manifest["benchmark_id"] = serde_json::json!("suite");
    f.write();
    assert!(proposal(&f.path, &current, 1).is_ok());
}
#[test]
fn selection_rejects_corruption_missing_files_nonfinite_and_contract_changes() {
    let mut f = Fixture::new();
    f.write();
    std::fs::write(f.root.join("model.k0.onnx"), "corrupt").unwrap();
    assert!(proposal(&f.path, &f.model, 1).is_err());
    std::fs::write(f.root.join("model.k0.onnx"), "model.k0.onnx").unwrap();
    std::fs::remove_file(f.root.join("model.k1.onnx")).unwrap();
    assert!(proposal(&f.path, &f.model, 1).is_err());
    std::fs::write(f.root.join("model.k1.onnx"), "model.k1.onnx").unwrap();
    f.manifest["score"] = Value::Null;
    f.write();
    assert!(proposal(&f.path, &f.model, 1).is_err());
    f.manifest["score"] = serde_json::json!(0.2);
    let nan = serde_json::to_string(&f.manifest)
        .unwrap()
        .replace("\"score\":0.2", "\"score\":NaN");
    std::fs::write(&f.path, nan).unwrap();
    assert!(proposal(&f.path, &f.model, 1).is_err());
}

#[test]
fn selection_rejects_contract_change_and_missing_required_hash() {
    let mut f = Fixture::new();
    f.manifest["sha256"]
        .as_object_mut()
        .unwrap()
        .remove("model.k0.onnx");
    f.write();
    assert!(proposal(&f.path, &f.model, 1).is_err());
    f.manifest["sha256"]["model.k0.onnx"] =
        serde_json::json!(digest(&f.root.join("model.k0.onnx")).unwrap());
    let path = f.root.join("model.onnx.json");
    let mut meta: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    meta["structure_sha256"] = serde_json::json!("changed-catalog");
    std::fs::write(&path, serde_json::to_vec(&meta).unwrap()).unwrap();
    f.manifest["sha256"]["model.onnx.json"] = serde_json::json!(digest(&path).unwrap());
    f.write();
    assert!(proposal(&f.path, &f.model, 1).is_err());
}

fn v5_metadata(revision: u64) -> Value {
    serde_json::json!({"encoder_version":7,"model_version":5,"trajectory_version":2,
        "sampling_version":2,"architecture":format!("baylee_ai_v5_relational_v{revision}"),
        "d_model":640,"n_heads":10,"n_layers":12,
        "config":{"d_model":640,"n_heads":10,"n_layers":12,
            "relational_revision":revision,"relational_steps":2,"relational_width":160}})
}

#[test]
fn candidate_context_is_bound_and_cannot_replace_a_legacy_seat_model() {
    let mut f = Fixture::new();
    let mut meta = v5_metadata(2);
    set_metadata(&mut f, &meta);
    let legacy = Selected::fixed(&f.root).unwrap();
    meta["architecture"] = serde_json::json!("baylee_ai_v5_relational_v2_candidate_v2");
    meta["config"]["candidate_context_revision"] = serde_json::json!(2);
    set_metadata(&mut f, &meta);
    assert!(Selected::fixed(&f.root).is_ok());
    assert!(proposal(&f.path, &legacy, 1).is_err());
    meta["config"]["candidate_context_revision"] = serde_json::json!(1);
    set_metadata(&mut f, &meta);
    assert!(Selected::fixed(&f.root).is_err());
    meta["architecture"] = serde_json::json!("baylee_ai_v5_relational_v2");
    meta["config"]["candidate_context_revision"] = serde_json::json!(2);
    set_metadata(&mut f, &meta);
    assert!(Selected::fixed(&f.root).is_err());
}

fn set_metadata(f: &mut Fixture, metadata: &Value) {
    let path = f.root.join("model.onnx.json");
    std::fs::write(&path, serde_json::to_vec(metadata).unwrap()).unwrap();
    f.manifest["sha256"]["model.onnx.json"] = serde_json::json!(digest(&path).unwrap());
    f.write();
}

#[test]
fn v5_fixed_and_proposals_accept_both_explicit_revisions_and_bind_dimensions() {
    for revision in [1, 2] {
        let mut f = Fixture::new();
        let mut meta = v5_metadata(revision);
        set_metadata(&mut f, &meta);
        let current = Selected::fixed(&f.root).unwrap();
        assert!(proposal(&f.path, &current, 1).is_ok());
        for key in ["relational_width", "relational_steps"] {
            meta["config"][key] = serde_json::json!(3);
            set_metadata(&mut f, &meta);
            assert!(proposal(&f.path, &current, 1).is_err());
            meta = v5_metadata(revision);
        }
        for (key, value) in [("d_model", 320), ("n_heads", 20), ("n_layers", 8)] {
            meta = v5_metadata(revision);
            meta[key] = serde_json::json!(value);
            meta["config"][key] = serde_json::json!(value);
            set_metadata(&mut f, &meta);
            assert!(proposal(&f.path, &current, 1).is_err());
        }
        meta = v5_metadata(3 - revision);
        set_metadata(&mut f, &meta);
        assert!(proposal(&f.path, &current, 1).is_err());
    }
}

#[test]
fn v5_refuses_unknown_architecture_revision_and_malformed_dimensions() {
    let mut f = Fixture::new();
    for (key, value) in [
        (
            "architecture",
            serde_json::json!("baylee_ai_v5_relational_v3"),
        ),
        ("encoder_version", serde_json::json!(6)),
        ("d_model", serde_json::json!(0)),
    ] {
        let mut meta = v5_metadata(2);
        meta[key] = value;
        set_metadata(&mut f, &meta);
        assert!(Selected::fixed(&f.root).is_err());
    }
    for (key, value) in [
        ("relational_revision", 1),
        ("relational_steps", 5),
        ("relational_width", 0),
        ("d_model", 320),
    ] {
        let mut meta = v5_metadata(2);
        meta["config"][key] = serde_json::json!(value);
        set_metadata(&mut f, &meta);
        assert!(Selected::fixed(&f.root).is_err());
    }
}

#[test]
fn encoder8_never_enters_live_model_selection_for_any_model_family() {
    let mut f = Fixture::new();
    let current = f.model.clone();
    for model_version in [3, 5] {
        let mut meta = v5_metadata(2);
        meta["encoder_version"] = serde_json::json!(8);
        meta["model_version"] = serde_json::json!(model_version);
        set_metadata(&mut f, &meta);
        let error = Selected::fixed(&f.root).unwrap_err();
        assert!(error.to_string().contains("offline-only"));
        assert!(
            proposal(&f.path, &current, 1)
                .unwrap_err()
                .to_string()
                .contains("offline-only")
        );
    }
}

#[test]
fn encoder9_is_explicitly_offline_only_before_live_model_loading() {
    let mut fixture = Fixture::new();
    let mut meta = v5_metadata(2);
    meta["encoder_version"] = serde_json::json!(9);
    meta["global_feat_width"] = serde_json::json!(82);
    set_metadata(&mut fixture, &meta);
    assert!(
        Selected::fixed(&fixture.root)
            .unwrap_err()
            .to_string()
            .contains("offline-only")
    );
}
