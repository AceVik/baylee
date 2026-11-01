use super::*;
struct Fixture {
    root: PathBuf,
    export: PathBuf,
    out: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
impl Fixture {
    fn new() -> Result<Self> {
        let root = std::env::temp_dir().join(format!("baylee-import-{}", uuid::Uuid::now_v7()));
        let export = root.join("export");
        let out = root.join("out");
        std::fs::create_dir_all(&export)?;
        Ok(Self { root, export, out })
    }
    fn write(&self, rows: &[Value], number: u64, mtime: u32) -> Result<String> {
        let raw = encode(rows)?;
        let file = File::create(self.export.join(format!("{number:06}.jsonl.gz")))?;
        let mut encoder = GzBuilder::new()
            .mtime(mtime)
            .write(file, Compression::default());
        encoder.write_all(&raw)?;
        encoder.finish()?;
        Ok(hash(&raw))
    }
    fn import(&self) -> Result<Value> {
        import_records(&self.export, &self.out, 0, None)
    }
    fn manifest(&self) -> Result<Value> {
        super::super::read_json(&self.out.join("manifest.json"))
    }
}
fn record() -> Vec<Value> {
    vec![
        json!({"kind":"header","record":1,"build":"test","hash":"0000000000000000","preset":{"seats":[{"controller":"Open"},{"controller":{"Human":{"account":0}}}]}}),
        json!({"kind":"declared_mind","n":0,"at":0,"seat":1,"mind":{"kind":"human"}}),
        json!({"kind":"input","n":1,"at":0,"seat":0,"by":"seat","action":"PassPriority","hash":"1111111111111111"}),
        json!({"kind":"input","n":2,"at":0,"seat":1,"by":"clock","action":"PassPriority","hash":"2222222222222222"}),
        json!({"kind":"input","n":3,"at":0,"seat":0,"by":"house","action":{"SetPriorityHold":"None"},"hash":"3333333333333333"}),
        json!({"kind":"input","n":4,"at":0,"seat":1,"by":"stand_in","action":"Concede","hash":"4444444444444444"}),
        json!({"kind":"end","n":5,"at":0,"winners":[0],"reason":"Conceded","turn":2}),
    ]
}
fn encode(rows: &[Value]) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    for row in rows {
        serde_json::to_writer(&mut raw, row)?;
        raw.push(b'\n');
    }
    Ok(raw)
}
#[test]
fn content_dedup_and_incremental_publication_are_immutable_and_idempotent() -> Result<()> {
    let f = Fixture::new()?;
    let digest = f.write(&record(), 1, 10)?;
    f.write(&record(), 2, 30)?;
    assert_eq!(f.import()?["records"], 1);
    let before = std::fs::read(f.out.join("manifest.json"))?;
    f.import()?;
    assert_eq!(before, std::fs::read(f.out.join("manifest.json"))?);
    let m = f.manifest()?;
    let item = &m["records"][&digest];
    assert_eq!(
        item["origins_by_seat"],
        json!([{"seat":1,"house":1},{"clock":1,"stand_in":1}])
    );
    assert_eq!(
        item["actions_by_seat"][0],
        json!({"decision":1,"automation":1})
    );
    assert_eq!(m["policy_targets"], "none_until_review");
    assert_eq!(m["ppo_eligible"], false);
    assert_eq!(item["split"], "train");
    assert_eq!(m["training_done"], false);
    let stored = f.out.join(item["path"].as_str().unwrap());
    assert_eq!(decompress(&stored)?, encode(&record())?);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(stored)?.permissions().mode() & 0o222, 0);
    }
    assert!(!serde_json::to_string(&m)?.contains("000001"));
    Ok(())
}
#[test]
fn an_invalid_batch_preserves_the_existing_manifest_and_records() -> Result<()> {
    let f = Fixture::new()?;
    f.write(&record(), 1, 10)?;
    f.import()?;
    let before = std::fs::read(f.out.join("manifest.json"))?;
    let mut rows = record();
    rows.pop();
    f.write(&rows, 2, 0)?;
    assert!(f.import().is_err());
    assert_eq!(before, std::fs::read(f.out.join("manifest.json"))?);
    assert_eq!(std::fs::read_dir(f.out.join("records"))?.count(), 1);
    Ok(())
}
#[test]
fn private_or_malformed_records_are_refused_without_inventing_policy_probabilities() -> Result<()> {
    let mut variants = Vec::new();
    let mut r = record();
    r[1]["at"] = json!(1_791_000_000);
    variants.push(r);
    let mut r = record();
    r[1]["game_id"] = json!("private");
    variants.push(r);
    let mut r = record();
    r[0]["preset"]["seats"][1]["controller"]["Human"]["account"] = json!(41);
    variants.push(r);
    let mut r = record();
    r[0]["preset"]["seats"]
        .as_array_mut()
        .unwrap()
        .push(json!({"controller":"Open"}));
    variants.push(r);
    let mut r = record();
    r[0]["build"] = json!("01a1728e-df52-7476-a918-efb7fc77fbe2");
    variants.push(r);
    let mut r = record();
    r[2]["n"] = json!(true);
    variants.push(r);
    let mut r = record();
    r[2]["action"] = json!({"PassPriority":null});
    variants.push(r);
    for r in variants {
        assert!(validate(&encode(&r)?, 0).is_err());
    }
    assert!(strict_json::parse(br#"{"x":1,"x":2}"#).is_err());
    assert!(strict_json::parse(br#"{"x":{"y":1,"y":2}}"#).is_err());
    assert!(strict_json::parse(b"NaN").is_err());
    Ok(())
}
#[test]
fn anonymous_exclusions_are_sticky_and_corrupt_immutable_content_fails_closed() -> Result<()> {
    let f = Fixture::new()?;
    let digest = f.write(&record(), 1, 0)?;
    let exclude = f.root.join("exclude.txt");
    std::fs::write(&exclude, format!("{digest} deterministic_smoke\n"))?;
    assert_eq!(
        import_records(&f.export, &f.out, 0, Some(&exclude))?["records"],
        0
    );
    f.import()?;
    assert_eq!(
        f.manifest()?["excluded"],
        json!({digest:"deterministic_smoke"})
    );
    let g = Fixture::new()?;
    let digest = g.write(&record(), 1, 0)?;
    g.import()?;
    let before = std::fs::read(g.out.join("manifest.json"))?;
    assert!(import_records(&g.export, &g.out, 1, None).is_err());
    let stored = g.out.join(format!("records/{digest}.jsonl.gz"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stored, std::fs::Permissions::from_mode(0o644))?;
    }
    std::fs::write(&stored, b"corrupt")?;
    assert!(g.import().is_err());
    assert_eq!(before, std::fs::read(g.out.join("manifest.json"))?);
    Ok(())
}
#[test]
fn house_is_excluded_and_unknown_mixed_or_missing_roles_are_quarantined() -> Result<()> {
    let f = Fixture::new()?;
    let mut house = record();
    house[1]["mind"] = json!({"kind":"house","level":"steady"});
    f.write(&house, 1, 0)?;
    let mut unknown = record();
    unknown[1]["mind"] = json!({"kind":"undeclared"});
    f.write(&unknown, 2, 0)?;
    let mut mixed = record();
    mixed.insert(
        6,
        json!({"kind":"declared_mind","at":0,"seat":1,"mind":{"kind":"house","level":"steady"}}),
    );
    for (i, row) in mixed.iter_mut().skip(1).enumerate() {
        row["n"] = json!(i);
    }
    f.write(&mixed, 3, 0)?;
    let mut missing = record();
    missing.remove(1);
    for (i, row) in missing.iter_mut().skip(1).enumerate() {
        row["n"] = json!(i);
    }
    f.write(&missing, 4, 0)?;
    let result = f.import()?;
    assert_eq!(result["records"], 0);
    assert_eq!(result["excluded_house"], 1);
    assert_eq!(result["quarantined"], 3);
    Ok(())
}
#[test]
fn unsafe_export_and_output_paths_are_rejected() -> Result<()> {
    let f = Fixture::new()?;
    f.write(&record(), 1, 0)?;
    assert!(import_records(&f.export, &f.export.join("nested"), 0, None).is_err());
    assert!(import_records(&f.export, &f.export, 0, None).is_err());
    std::fs::write(f.export.join("unexpected.txt"), b"x")?;
    assert!(f.import().is_err());
    Ok(())
}
