use super::*;
use std::{fs, io::Write};
fn fixture(root: &Path, version: u32) -> Result<std::path::PathBuf> {
    let shard = root.join("shard-00");
    fs::create_dir(&shard)?;
    files::atomic_json(
        &root.join("dataset.json"),
        &json!({"encoder_version":version,"meta_cols":["pending_kind","n_opts","chosen"],"pending_kinds":["priority","target"],"ids":{"ledger_rows":2},"shards":[{"dir":"shard-00","samples":2,"entities":4,"opts":6}]}),
    )?;
    let mut cards = fs::File::create(shard.join("ent_card.i32"))?;
    for n in [1_i32, 2, 3, 1] {
        cards.write_all(&n.to_le_bytes())?;
    }
    let mut meta = fs::File::create(shard.join("meta.i32"))?;
    for n in [0_i32, 5, 1, 0, 1, 0] {
        meta.write_all(&n.to_le_bytes())?;
    }
    for (name, offsets) in [
        ("ent_off.i64", [0_i64, 3, 4]),
        ("opt_off.i64", [0_i64, 5, 6]),
    ] {
        let mut file = fs::File::create(shard.join(name))?;
        for offset in offsets {
            file.write_all(&offset.to_le_bytes())?;
        }
    }
    let mut options = fs::File::create(shard.join("opt.i16"))?;
    for row in [
        [1_i16, 0, 1],
        [2, 1, 0],
        [2, 0, 4],
        [1, 0, 3],
        [1, 2, 1],
        [1, 0, 1],
    ] {
        for n in row {
            options.write_all(&n.to_le_bytes())?;
        }
    }
    let inventory = root.join("inventory.json");
    files::atomic_json(
        &inventory,
        &json!([{"card":0,"name":"Alpha","abilities":[{"kind":"mana","position":4}]},{"card":1,"name":"Beta","abilities":[{"kind":"activated","position":0}]}]),
    )?;
    Ok(inventory)
}
#[test]
fn streamed_v2_and3_counts_exclude_mana_tokens_and_forced_decisions() -> Result<()> {
    for version in [2, 3] {
        let root = tempfile::tempdir()?;
        let names = fixture(root.path(), version)?;
        let report = analyze(root.path(), Some(&names))?;
        assert_eq!(report["priority_decisions"], 1);
        assert_eq!(report["rows"].as_array().unwrap().len(), 2);
        assert_eq!(report["rows"][0]["name"], "Alpha");
        assert_eq!(report["rows"][0]["offered"], 1);
        assert_eq!(report["rows"][0]["taken"], 0);
        assert_eq!(report["rows"][1]["name"], "Beta");
        assert_eq!(report["rows"][1]["use"], "ability 0");
        assert_eq!(report["rows"][1]["taken"], 1);
        let out = root.path().join("report");
        super::report(root.path(), &out, Some(&names), 1)?;
        assert!(fs::read_to_string(out.join("usage.md"))?.contains("diagnostic signal"));
    }
    Ok(())
}
#[test]
fn malformed_columns_offsets_and_chosen_actions_fail() -> Result<()> {
    for case in ["size", "offset", "chosen"] {
        let root = tempfile::tempdir()?;
        let names = fixture(root.path(), 3)?;
        let shard = root.path().join("shard-00");
        match case {
            "size" => fs::write(shard.join("opt.i16"), [])?,
            "offset" => {
                let mut data = fs::read(shard.join("ent_off.i64"))?;
                data[8..16].copy_from_slice(&99_i64.to_le_bytes());
                fs::write(shard.join("ent_off.i64"), data)?;
            }
            _ => {
                let mut data = fs::read(shard.join("meta.i32"))?;
                data[8..12].copy_from_slice(&99_i32.to_le_bytes());
                fs::write(shard.join("meta.i32"), data)?;
            }
        }
        assert!(analyze(root.path(), Some(&names)).is_err(), "{case}");
    }
    Ok(())
}
