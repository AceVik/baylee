use super::*;
use baylee_train_ops::{guard, process, research_attempts};
use std::{fs::OpenOptions, sync::atomic::AtomicBool};

impl Fixture {
    fn finalized() -> Self {
        let mut fixture = Self::new();
        let source = fixture.root.join("source");
        fixture.run["planned_deals"] = json!(5);
        fixture.run["accepted_deck_keys"] = json!(["left", "right"]);
        files::atomic_json(&source.join("run.json"), &fixture.run).unwrap();
        let digest = files::sha256(&source.join("run.json")).unwrap();
        for row in &mut fixture.rows {
            row["source_run_sha256"] = json!(digest);
        }
        fixture.index();
        fs::remove_file(source.join("run.complete.json")).unwrap();
        OpenOptions::new()
            .append(true)
            .open(source.join("games.jsonl"))
            .unwrap()
            .write_all(b"{\"i\":3,\"orphan_index_row\":true}\n{\"i\":")
            .unwrap();
        fs::write(
            source.join("records/worker-orphan.partial"),
            b"interrupted gzip evidence",
        )
        .unwrap();
        let helper = source.join("finalized-converter-helper");
        std::os::unix::fs::symlink(std::env::current_exe().unwrap(), &helper).unwrap();
        let job = guard::Job {
            directory: fixture.root.join("guard-job"),
            cwd: std::env::current_dir().unwrap(),
            command: vec![
                helper.to_str().unwrap().into(),
                "--exact".into(),
                "research::tests::attempt_ledger::native_failed_worker_helper".into(),
            ],
            timeout_seconds: 10.0,
            memory_gib: 1.0,
            inputs: vec![source.join("run.json")],
        };
        assert!(!guard::run(&job, &AtomicBool::new(false)).unwrap());
        let receipt = files::read_json(&job.directory.join("job.json")).unwrap();
        assert_eq!(receipt["cleanup_verified"], true);
        assert_eq!(receipt["returncode"], 17);
        let identity: process::Identity =
            serde_json::from_value(receipt["child_identity"].clone()).unwrap();
        assert_ne!(process::identity(identity.pid).as_ref(), Some(&identity));
        research_attempts::finalize(&source, &job.directory.join("job.json")).unwrap();
        fixture
    }
}

#[test]
fn native_failed_worker_helper() {
    let invocation = PathBuf::from(std::env::args_os().next().unwrap());
    if invocation
        .file_name()
        .is_none_or(|name| name != "finalized-converter-helper")
    {
        return;
    }
    let root = invocation.parent().unwrap();
    let rows: Vec<Value> = fs::read_to_string(root.join("games.jsonl"))
        .unwrap()
        .lines()
        .take(3)
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let attempts = (0..5)
        .map(|deal| research_attempts::Attempt {
            deal,
            seed: 12,
            focal_seat: usize::try_from(deal % 2).unwrap(),
            policy_seed: 19,
            deck_keys: vec!["left".into(), "right".into()],
        })
        .collect();
    let ledger = research_attempts::Ledger::create(root, attempts).unwrap();
    for (worker, row) in rows.iter().enumerate() {
        ledger.claim(row["i"].as_u64().unwrap(), worker).unwrap();
        ledger.completed(row).unwrap();
    }
    ledger.claim(3, 3).unwrap();
    let failed =
        std::thread::spawn(|| Err::<(), _>("fixture worker failed before game publication"));
    assert!(failed.join().unwrap().is_err());
    ledger.worker_failed(3, "worker_error").unwrap();
    drop(ledger);
    std::process::exit(17);
}

#[test]
fn failed_worker_partial_index_and_not_started_preserve_every_denominator_without_targets() {
    let fixture = Fixture::finalized();
    let source = fixture.root.join("source");
    let original_index = fs::read(source.join("games.jsonl")).unwrap();
    let out = fixture.root.join("converted-ledger");
    convert(&Args {
        runs: vec![],
        attempt_ledger_runs: vec![source.clone()],
        out: out.clone(),
    })
    .unwrap();
    assert_eq!(
        fs::read(source.join("games.jsonl")).unwrap(),
        original_index
    );
    assert!(!source.join("run.complete.json").exists());
    assert!(
        source::Run::load(&source).is_err(),
        "old adapter cannot reinterpret a partial collection"
    );
    let manifest = files::read_json(&out.join("research.manifest.json")).unwrap();
    assert_eq!(
        manifest["schema"],
        "failure_terminal_research_conversion_v2"
    );
    assert_eq!(
        manifest["attempt_denominator_contract"],
        "failure_terminal_attempt_ledger_v1"
    );
    assert_eq!(
        manifest["attempt_counts"],
        json!({"planned":5,"assigned":4,"recorded":3,
        "operational_unknown":1,"not_started":1,"recorded_unknown":1,"target_episodes":2})
    );
    assert_eq!(manifest["unknown_attempts"], 2);
    assert_eq!(manifest["ppo_eligible"], false);
    let attempts: Vec<Value> = fs::read_to_string(out.join("attempts.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(attempts.len(), 5);
    for (deal, row) in attempts.iter().enumerate() {
        assert_eq!(row["source_index"], deal);
        assert_eq!(row["planned_attempt"]["focal_seat"], deal % 2);
    }
    assert_eq!(attempts[1]["source_attempt_status"], "recorded");
    assert_eq!(attempts[1]["returns"]["termination_kind"], "unknown");
    assert_eq!(attempts[3]["source_attempt_status"], "operational_unknown");
    assert_eq!(
        attempts[3]["source_attempt_ledger_entry"]["unknown_reason"],
        "worker_error"
    );
    assert!(attempts[3]["returns"]["training_utility"].is_null());
    assert!(attempts[3]["returns"]["mtg_outcome"].is_null());
    assert_eq!(attempts[4]["source_attempt_status"], "not_started");
    assert_eq!(attempts[4]["assigned"], false);
    assert!(attempts[4]["returns"].is_null());
    for row in [&attempts[1], &attempts[3], &attempts[4]] {
        assert!(row["episode_file"].is_null());
        assert_eq!(row["frames"], 0);
    }
    assert_eq!(fs::read_dir(out.join("episodes")).unwrap().count(), 2);
    assert!(baylee_train_ops::trajectory::cache::build(&out, "train", &[]).is_err());
}

#[test]
fn finalized_adapter_rejects_missing_seals_changed_evidence_and_resealed_false_ledger() {
    let fixture = Fixture::finalized();
    let source = fixture.root.join("source");
    let bound = source::Run::load_finalized(&source).unwrap();
    let paths = [
        source.join("attempts.complete.json"),
        source.join("attempts.final.json"),
        fixture.root.join("guard-job/job.json"),
        source.join("records/worker-orphan.partial"),
        source.join("attempts/00000000000000000000/game.json"),
    ];
    for path in paths {
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"changed source bytes").unwrap();
        assert!(
            source::Run::load_finalized(&source).is_err(),
            "{}",
            path.display()
        );
        assert!(bound.recheck().is_err());
        fs::write(&path, original).unwrap();
    }
    let output = source.join("attempts.final.json");
    let mut ledger = files::read_json(&output).unwrap();
    ledger["attempts"][3]["training_utility"] = json!(0.0);
    files::atomic_json(&output, &ledger).unwrap();
    let receipt = source.join("attempts.complete.json");
    let mut seal = files::read_json(&receipt).unwrap();
    seal["ledger_sha256"] = json!(files::sha256(&output).unwrap());
    files::atomic_json(&receipt, &seal).unwrap();
    assert!(source::Run::load_finalized(&source).is_err());
}

#[test]
fn denominator_adapter_selection_is_explicit_and_cannot_mix_paths() {
    for options in [
        vec![],
        vec!["--runs", "ordinary", "--attempt-ledger-runs", "ledger"],
    ] {
        let mut arguments = vec!["research", "--out", "new-output"];
        arguments.extend(options);
        assert!(Args::try_parse_from(arguments).is_err());
    }
    for flag in ["--runs", "--attempt-ledger-runs"] {
        assert!(Args::try_parse_from(["research", flag, "source", "--out", "new-output"]).is_ok());
    }
}
