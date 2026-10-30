use super::*;
use crate::{files, trajectory::cache};
use anyhow::Result;
use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Write,
    path::Path,
    sync::atomic::AtomicBool,
};

fn shard(path: &Path, game: u32, rows: u32, broken: Option<&str>) -> Result<()> {
    let mut gz = GzEncoder::new(File::create(path)?, Compression::default());
    for cursor in 0..rows {
        let actual = if broken == Some("cursor") && cursor == 2 {
            1
        } else {
            cursor
        };
        serde_json::to_writer(
            &mut gz,
            &json!({"game_id":game,"seat":0,"step_cursor":actual,"is_decision":cursor>0,"is_terminal":cursor==rows-1 && broken!=Some("terminal")}),
        )?;
        gz.write_all(b"\n")?;
    }
    gz.finish()?;
    if broken == Some("gzip") {
        let file = File::options().write(true).open(path)?;
        file.set_len(file.metadata()?.len() - 8)?;
    }
    Ok(())
}
fn fixture(root: &Path) -> Result<(std::path::PathBuf, Value, Value)> {
    let source = root.join("source");
    fs::create_dir(&source)?;
    fs::create_dir(source.join("train"))?;
    shard(&source.join("train/a.gz"), 1, 7, None)?;
    for asset in publish::ASSETS {
        fs::write(
            source.join(asset),
            if asset == "card_structure.json" {
                b"{\"roots\":{}}".as_slice()
            } else {
                b""
            },
        )?;
    }
    let header = json!({"structure_sha256":"public","source_runs":["/runs/train"],"validation_runs":["/runs/heldout"],"train_shards":["a.gz"],"val_shards":[],"unencoded_decisions":0,"games_rejected":0});
    files::atomic_json(&source.join("meta.json"), &header)?;
    let entry = json!({"game_id":1,"steps":7,"split":"train","shard":"a.gz","source_shard":"/runs/train/records/a.gz","structure_sha256":"public","replay_hashes_checked":true,"unencoded_decisions":0});
    Ok((source, header, entry))
}
#[test]
fn seeded_source_balance_conserves_every_complete_game() -> Result<()> {
    let mut entries = (0..100)
        .map(|i| json!({"game_id":i,"steps":10,"source_shard":"/runs/duels/records/a.gz"}))
        .collect::<Vec<_>>();
    entries.extend(
        (100..110)
            .map(|i| json!({"game_id":i,"steps":100,"source_shard":"/runs/eight/records/a.gz"})),
    );
    let first = select_entries(&entries, 500, 2718)?;
    let mut reversed = entries.clone();
    reversed.reverse();
    assert_eq!(first, select_entries(&reversed, 500, 2718)?);
    assert_ne!(first, select_entries(&entries, 500, 2719)?);
    let mut seen = Vec::<u64>::new();
    let mut seed = 2718;
    while !entries.is_empty() {
        let selected = select_entries(&entries, 500, seed)?;
        let frames = selected
            .iter()
            .map(|v| v["steps"].as_u64().unwrap())
            .sum::<u64>();
        if entries
            .iter()
            .map(|v| v["steps"].as_u64().unwrap())
            .sum::<u64>()
            >= 500
        {
            assert!((500..600).contains(&frames));
        }
        let duel = selected
            .iter()
            .filter(|v| v["game_id"].as_u64().unwrap() < 100)
            .map(|v| v["steps"].as_u64().unwrap())
            .sum::<u64>();
        assert!(duel.abs_diff(frames - duel) <= 110);
        let ids = selected
            .iter()
            .map(|v| v["game_id"].as_u64().unwrap())
            .collect::<BTreeSet<_>>();
        seen.extend(&ids);
        entries.retain(|v| !ids.contains(&v["game_id"].as_u64().unwrap()));
        seed += 1;
    }
    seen.sort_unstable();
    assert_eq!(seen, (0_u64..110).collect::<Vec<_>>());
    assert!(select_entries(&[], 500, 1)?.is_empty());
    assert!(select_entries(&[json!({"game_id":1,"steps":0})], 500, 1).is_err());
    Ok(())
}
#[test]
fn ready_means_reusable_complete_immutable_game_cache() -> Result<()> {
    let root = tempfile::tempdir()?;
    let (source, header, entry) = fixture(root.path())?;
    let output = root.path().join("package");
    let marker = publish(&source, &output, &[entry], &header)?;
    assert_eq!(marker["frames"], 7);
    assert_eq!(
        marker["metadata_sha256"],
        files::sha256(&output.join("meta.json"))?
    );
    let a = cache::build(&output, "train", &["a.gz".into()])?;
    let modified = a.data_path.metadata()?.modified()?;
    let b = cache::build(&output, "train", &["a.gz".into()])?;
    assert_eq!(modified, b.data_path.metadata()?.modified()?);
    assert_eq!(
        verify_index(&b.index_path, &BTreeSet::from([1]))?,
        (BTreeSet::from([1]), 7)
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            source.join("train/a.gz").metadata()?.ino(),
            output.join("train/a.gz").metadata()?.ino()
        );
    }
    let before = fs::read(output.join("ready.json"))?;
    assert!(publish(&source, &output, &[], &header).is_err());
    assert_eq!(before, fs::read(output.join("ready.json"))?);
    Ok(())
}
#[test]
fn corrupt_partial_validation_or_miscounted_packages_never_publish() -> Result<()> {
    for change in [
        json!({"split":"val"}),
        json!({"replay_hashes_checked":false}),
        json!({"structure_sha256":"wrong"}),
        json!({"unencoded_decisions":1}),
        json!({"shard":"../a.gz"}),
        json!({"game_id":77}),
        json!({"steps":8}),
    ] {
        let root = tempfile::tempdir()?;
        let (source, header, mut entry) = fixture(root.path())?;
        entry
            .as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        let output = root.path().join("bad");
        assert!(
            publish(&source, &output, &[entry], &header).is_err(),
            "{change}"
        );
        assert!(!output.join("ready.json").exists());
    }
    for failure in ["gzip", "cursor", "terminal"] {
        let root = tempfile::tempdir()?;
        let (source, header, entry) = fixture(root.path())?;
        shard(&source.join("train/a.gz"), 1, 7, Some(failure))?;
        let output = root.path().join("bad");
        assert!(
            publish(&source, &output, &[entry], &header).is_err(),
            "{failure}"
        );
        assert!(!output.join("ready.json").exists());
    }
    Ok(())
}
#[test]
fn recovery_excludes_unclosed_last_shard_and_preserves_failed_attempt() -> Result<()> {
    let root = tempfile::tempdir()?;
    let (base, header, _) = fixture(root.path())?;
    let source = root.path().join("partial");
    fs::create_dir(&source)?;
    fs::create_dir(source.join("train"))?;
    shard(&source.join("train/a.gz"), 1, 3, None)?;
    shard(&source.join("train/b.gz"), 2, 4, None)?;
    fs::write(source.join("train/c.jsonl.gz"), b"unclosed")?;
    // Recovery recognizes the historical closed JSON gzip shard suffix.
    fs::rename(source.join("train/a.gz"), source.join("train/a.jsonl.gz"))?;
    fs::rename(source.join("train/b.gz"), source.join("train/b.jsonl.gz"))?;
    let run = root.path().join("run");
    fs::create_dir(&run)?;
    fs::write(
        run.join("games.jsonl"),
        "{\"outcome\":{\"kind\":\"won\"},\"shard\":\"records.gz\",\"offset\":0,\"len\":1}\n{\"outcome\":{\"kind\":\"draw\"},\"shard\":\"records.gz\",\"offset\":1,\"len\":1}\n",
    )?;
    let failure = root.path().join("failed.json");
    let template = root.path().join("template.json");
    files::atomic_json(
        &failure,
        &json!({"status":"quarantined","reason":"memory_limit","executable_sha256":"fixed-tool","command":["convert4","--out",source,"--runs",run,"--val-runs",""]}),
    )?;
    files::atomic_json(
        &template,
        &json!({"status":"complete","executable_sha256":"fixed-tool","command":["convert4","--out",base]}),
    )?;
    fs::write(root.path().join("stderr.log"), b"memory pressure")?;
    let before = fs::read(&failure)?;
    let output = root.path().join("recovered");
    let result = recover(&Recovery {
        failed_job: failure.clone(),
        template_job: template,
        out: output.clone(),
        shards: 8,
        max_shard_mib: 100,
    })?;
    assert_eq!(result["games"], 2);
    assert_eq!(result["frames"], 7);
    assert_eq!(before, fs::read(failure)?);
    let meta = files::read_json(&output.join("meta.json"))?;
    assert_eq!(meta["structure_sha256"], header["structure_sha256"]);
    assert_eq!(meta["recovery"]["excluded_shards"], json!(["c.jsonl.gz"]));
    let cache = cache::build(
        &output,
        "train",
        &["a.jsonl.gz".into(), "b.jsonl.gz".into()],
    )?;
    assert_eq!(cache.records, 7);
    Ok(())
}
#[test]
#[cfg(target_os = "linux")]
fn consumer_manifest_is_explicit_guarded_and_does_not_repeat_completed_work() -> Result<()> {
    let root = tempfile::tempdir()?;
    let (source, header, entry) = fixture(root.path())?;
    let packages = root.path().join("packages");
    fs::create_dir(&packages)?;
    publish(&source, &packages.join("package-0000"), &[entry], &header)?;
    files::atomic_json(&packages.join("complete.json"), &json!({"packages":1}))?;
    let producer = root.path().join("producer.json");
    files::atomic_json(&producer, &json!({"status":"complete"}))?;
    let validation = root.path().join("validation");
    fs::create_dir(&validation)?;
    fs::write(validation.join("meta.json"), b"{\"val_shards\":[]}")?;
    for asset in publish::ASSETS {
        fs::copy(source.join(asset), validation.join(asset))?;
    }
    let initial = root.path().join("last.pt");
    fs::write(&initial, b"opaque test checkpoint; no learning claim")?;
    let output = root.path().join("trained");
    let plan = root.path().join("consumer.json");
    files::atomic_json(
        &plan,
        &json!({"version":1,"packages":packages,"producer_job":producer,"validation_data":validation,"output":output,"cwd":std::env::current_dir()?,"initial_checkpoint":initial,"train_command":["/usr/bin/cp","-r","{checkpoint}","{package}","{validation_data}","{output}"],"timeout_seconds":10.0,"job_timeout_seconds":5.0,"memory_gib":1.0,"max_packages":1,"gpu_lock":root.path().join("gpu-work.lock")}),
    )?;
    let historic = packages.join("package-0000/consumed.json");
    fs::write(&historic, b"{\"historical\":true}")?;
    let original_receipt = fs::read(&historic)?;
    consume(&plan, &AtomicBool::new(false))?;
    assert_eq!(original_receipt, fs::read(&historic)?);
    let before = fs::read(output.join("progress.json"))?;
    let seal = files::sha256(&plan)?;
    let experiment_receipt = packages
        .join("package-0000/consumption")
        .join(format!("{seal}.json"));
    let expected_receipt = fs::read(&experiment_receipt)?;
    // Crash after the progress commit but before either receipt publication.
    fs::remove_file(&experiment_receipt)?;
    consume(&plan, &AtomicBool::new(false))?;
    assert_eq!(before, fs::read(output.join("progress.json"))?);
    assert_eq!(expected_receipt, fs::read(&experiment_receipt)?);
    assert_eq!(original_receipt, fs::read(&historic)?);
    fs::remove_file(&experiment_receipt)?;
    fs::remove_file(&historic)?;
    consume(&plan, &AtomicBool::new(false))?;
    assert_eq!(expected_receipt, fs::read(&experiment_receipt)?);
    assert_eq!(expected_receipt, fs::read(&historic)?);
    assert!(packages.join("package-0000/consumed.json").is_file());
    let job_path = output.join("jobs/package-0000/job.json");
    let job = files::read_json(&job_path)?;
    for change in [
        json!({"status":"quarantined"}),
        json!({"cleanup_verified":false}),
        json!({"returncode":1}),
        json!({"inputs":{}}),
        json!({"command":["other-command"]}),
        json!({"child_identity":crate::process::identity(std::process::id()),"pid":std::process::id()}),
    ] {
        let mut invalid = job.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        files::atomic_json(&job_path, &invalid)?;
        fs::remove_file(&experiment_receipt)?;
        assert!(consume(&plan, &AtomicBool::new(false)).is_err(), "{change}");
        assert!(!experiment_receipt.exists());
        assert_eq!(before, fs::read(output.join("progress.json"))?);
        files::atomic_json(&job_path, &job)?;
        consume(&plan, &AtomicBool::new(false))?;
    }
    // A progress name cannot route recovery outside either owned root.
    let mut progress = files::read_json(&output.join("progress.json"))?;
    progress["consumed"] = json!(["../package-0000"]);
    files::atomic_json(&output.join("progress.json"), &progress)?;
    assert!(consume(&plan, &AtomicBool::new(false)).is_err());
    fs::write(output.join("progress.json"), &before)?;
    let mut altered = files::read_json(&plan)?;
    altered["max_packages"] = json!(2);
    files::atomic_json(&plan, &altered)?;
    assert!(consume(&plan, &AtomicBool::new(false)).is_err());
    assert_eq!(before, fs::read(output.join("progress.json"))?);
    Ok(())
}

#[test]
fn producer_publishes_complete_tail_and_rejects_validation_source_overlap() -> Result<()> {
    let root = tempfile::tempdir()?;
    let (source, header, entry) = fixture(root.path())?;
    fs::create_dir(source.join("ready"))?;
    files::atomic_json(&source.join("ready/game.json"), &entry)?;
    files::atomic_json(&source.join("dataset_header.json"), &header)?;
    let job = root.path().join("converter-job.json");
    files::atomic_json(&job, &json!({"status":"complete"}))?;
    let validation = root.path().join("validation");
    fs::create_dir(&validation)?;
    files::atomic_json(&validation.join("meta.json"), &header)?;
    let mut args = Producer {
        source,
        source_job: job,
        out: root.path().join("packages"),
        validation_data: validation.clone(),
        frames: 500_000,
        ahead: 2,
        seed: 2718,
        timeout: 5.0,
        allow_replay_quarantined: false,
    };
    produce(&args, &AtomicBool::new(false))?;
    assert!(args.out.join("package-0000/ready.json").is_file());
    assert_eq!(
        files::read_json(&args.out.join("complete.json"))?["games"],
        1
    );
    let completed = fs::read(args.out.join("complete.json"))?;
    let mut overlap = header;
    overlap["validation_runs"] = json!(["/runs/train"]);
    files::atomic_json(&validation.join("meta.json"), &overlap)?;
    args.out = root.path().join("rejected");
    assert!(produce(&args, &AtomicBool::new(false)).is_err());
    assert!(!args.out.join("package-0000/ready.json").exists());
    assert_eq!(
        completed,
        fs::read(root.path().join("packages/complete.json"))?
    );
    Ok(())
}
