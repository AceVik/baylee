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
