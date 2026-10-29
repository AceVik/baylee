use super::*;
use flate2::{Compression, write::GzEncoder};
use sha2::Digest as _;

struct Fixture {
    root: tempfile::TempDir,
    ledger: Option<Ledger>,
    job: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        files::atomic_json(
            &root.path().join("run.json"),
            &json!({"collection_contract":{"purpose":"failure_terminal_research_v1","version":2},
            "training_data":false,"planned_deals":3,"deal_offset":0}),
        )
        .unwrap();
        fs::write(root.path().join("games.jsonl"), b"").unwrap();
        let attempts = (0..3)
            .map(|deal| Attempt {
                deal,
                seed: 100 + deal / 2,
                focal_seat: deal as usize % 2,
                policy_seed: 77,
                deck_keys: vec!["left".into(), "right".into()],
            })
            .collect();
        let producer = process::Identity {
            pid: u32::MAX,
            start_ticks: 1,
            boot_id: "fixture-dead-producer".into(),
        };
        let ledger = Ledger::create_for(root.path(), attempts, producer.clone()).unwrap();
        let job = root.path().join("job.json");
        files::atomic_json(
            &job,
            &json!({"guard":"rust-owned-group-v1","status":"quarantined","cleanup_verified":true,
            "child_identity":producer,"reason":"memory_limit"}),
        )
        .unwrap();
        Self {
            root,
            ledger: Some(ledger),
            job,
        }
    }
    fn ledger(&self) -> &Ledger {
        self.ledger.as_ref().unwrap()
    }
    fn close(&mut self) {
        self.ledger.take();
    }
    fn finalize(&self) -> Result<Value> {
        finalize(self.root.path(), &self.job)
    }
    fn game(&self, deal: u64) -> Value {
        fs::create_dir_all(self.root.path().join("records")).unwrap();
        let bytes = b"{\"kind\":\"header\"}\n";
        let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
        gz.write_all(bytes).unwrap();
        let compressed = gz.finish().unwrap();
        fs::write(self.root.path().join("records/game.gz"), &compressed).unwrap();
        let episode = json!({"kind":"research_termination","termination_kind":"unknown","training_utility":null,"bootstrap":false});
        let row = json!({"i":deal,"seed":100+deal/2,"focal_seat":deal%2,"policy_seed":77,
            "source_run_sha256":self.ledger().plan.source_run_sha256,"outcome":episode,"research_episode":episode,
            "shard":"game.gz","offset":0,"len":compressed.len(),"record_sha256":files::hex(&sha2::Sha256::digest(bytes))});
        files::append_json(&self.root.path().join("games.jsonl"), &row).unwrap();
        row
    }
}

#[test]
fn fixed_denominator_keeps_recorded_unknown_and_not_started_separate() {
    let mut fixture = Fixture::new();
    fixture.ledger().claim(0, 0).unwrap();
    fixture.ledger().claim(1, 1).unwrap();
    let row = fixture.game(0);
    fixture.ledger().completed(&row).unwrap();
    fixture.close();
    let ledger = fixture.finalize().unwrap();
    assert_eq!(
        ledger["counts"],
        json!({"planned":3,"assigned":2,"recorded":1,"operational_unknown":1,"not_started":1})
    );
    let rows = ledger["attempts"].as_array().unwrap();
    assert_eq!(rows[0]["status"], "recorded");
    assert_eq!(rows[1]["status"], "operational_unknown");
    assert_eq!(rows[1]["unknown_reason"], "process_memory_limit");
    assert!(rows[1]["training_utility"].is_null());
    assert_eq!(rows[1]["bootstrap"], false);
    assert_eq!(rows[2]["status"], "not_started");
    assert_eq!(rows[2]["assigned"], false);
    assert_eq!(ledger, fixture.finalize().unwrap());
}

#[test]
fn rust_observation_panic_leaves_a_claimed_unknown_without_game_or_loss() {
    let mut fixture = Fixture::new();
    fixture.ledger().claim(0, 0).unwrap();
    let worker = std::thread::spawn(|| panic!("mock observation panic"));
    assert!(worker.join().is_err());
    fixture.ledger().worker_failed(0, "worker_panic").unwrap();
    fixture.close();
    let ledger = fixture.finalize().unwrap();
    assert_eq!(ledger["attempts"][0]["unknown_reason"], "worker_panic");
    assert!(ledger["attempts"][0]["training_utility"].is_null());
    assert!(ledger["attempts"][0]["game_receipt"].is_null());
    assert_eq!(ledger["counts"]["not_started"], 2);
}

#[test]
fn live_identity_and_unverified_group_cleanup_cannot_finalize() {
    let mut fixture = Fixture::new();
    let plan = &fixture.ledger().plan;
    let mut job = files::read_json(&fixture.job).unwrap();
    assert!(finalize::verified_guard(plan, &job, Some(&plan.producer_identity)).is_err());
    let recycled = process::Identity {
        start_ticks: 2,
        ..plan.producer_identity.clone()
    };
    assert!(finalize::verified_guard(plan, &job, Some(&recycled)).is_ok());
    job["cleanup_verified"] = json!(false);
    assert!(finalize::verified_guard(plan, &job, None).is_err());
    job["cleanup_verified"] = json!(true);
    job["status"] = json!("running");
    assert!(finalize::verified_guard(plan, &job, None).is_err());
    assert!(
        fixture.finalize().is_err(),
        "live ledger lock also forbids recovery"
    );
    fixture.close();
    assert!(fixture.finalize().is_ok());
}

#[test]
fn partial_publication_is_unknown_and_truncated_canonical_receipt_is_refused() {
    let mut fixture = Fixture::new();
    fixture.ledger().claim(0, 0).unwrap();
    let directory = attempt_path(fixture.root.path(), 0);
    fs::write(directory.join(".interrupted.partial"), b"{\"outcome\":").unwrap();
    fs::write(fixture.root.path().join("games.jsonl"), b"{\"i\":").unwrap();
    fs::create_dir_all(fixture.root.path().join("records")).unwrap();
    let orphan = fixture.root.path().join("records/interrupted.gz");
    fs::write(&orphan, b"partial gzip member").unwrap();
    fixture.close();
    let ledger = fixture.finalize().unwrap();
    assert_eq!(ledger["attempts"][0]["status"], "operational_unknown");
    assert_eq!(
        ledger["source_files"][orphan.to_str().unwrap()],
        files::sha256(&orphan).unwrap()
    );
    fs::write(&orphan, b"changed orphan evidence").unwrap();
    assert!(fixture.finalize().is_err());
    let mut corrupt = Fixture::new();
    corrupt.ledger().claim(0, 0).unwrap();
    fs::write(
        attempt_path(corrupt.root.path(), 0).join("game.json"),
        b"{\"i\":",
    )
    .unwrap();
    corrupt.close();
    assert!(corrupt.finalize().is_err());
    assert!(!corrupt.root.path().join("attempts.complete.json").exists());
}

#[test]
fn duplicate_claims_changed_records_and_changed_sealed_receipts_are_rejected() {
    let mut fixture = Fixture::new();
    fixture.ledger().claim(0, 0).unwrap();
    assert!(fixture.ledger().claim(0, 1).is_err());
    assert!(fixture.ledger().claim(3, 1).is_err());
    let mut row = fixture.game(0);
    row["record_sha256"] = json!("wrong");
    assert!(fixture.ledger().completed(&row).is_err());
    fixture.close();
    fixture.finalize().unwrap();
    let claim = attempt_path(fixture.root.path(), 0).join("claim.json");
    let mut changed = files::read_json(&claim).unwrap();
    changed["extra"] = json!("changed after sealing");
    files::atomic_json(&claim, &changed).unwrap();
    assert!(fixture.finalize().is_err());
}

#[test]
fn indexed_games_outside_plan_or_without_assignment_cannot_disappear() {
    for deal in [0, 99] {
        let mut fixture = Fixture::new();
        // A fully serialized row is evidence of a produced game, even when a
        // corrupted source dropped its assignment or changed its deal ID.
        files::append_json(&fixture.root.path().join("games.jsonl"), &json!({"i":deal})).unwrap();
        fixture.close();
        assert!(fixture.finalize().is_err());
        assert!(!fixture.root.path().join("attempts.complete.json").exists());
    }
    for name in ["0", "00000000000000000099"] {
        let mut fixture = Fixture::new();
        fs::create_dir_all(fixture.root.path().join("attempts").join(name)).unwrap();
        fixture.close();
        assert!(
            fixture.finalize().is_err(),
            "aliased or unplanned assignment evidence must not disappear either"
        );
    }
}

#[test]
fn missing_completion_seal_recovers_only_byte_identical_recomputed_ledger() {
    for changed in [None, Some("ledger"), Some("source")] {
        let mut fixture = Fixture::new();
        fixture.ledger().claim(0, 0).unwrap();
        fixture.close();
        let original = fixture.finalize().unwrap();
        let output = fixture.root.path().join("attempts.final.json");
        let receipt = fixture.root.path().join("attempts.complete.json");
        let before = fs::read(&output).unwrap();
        fs::remove_file(&receipt).unwrap();
        match changed {
            Some("ledger") => {
                let mut ledger = original.clone();
                ledger["attempts"][0]["training_utility"] = json!(0);
                files::atomic_json(&output, &ledger).unwrap();
            }
            Some("source") => {
                fs::write(fixture.root.path().join("games.jsonl"), b"{\"i\":").unwrap();
            }
            _ => {}
        }
        let result = fixture.finalize();
        if changed.is_some() {
            assert!(result.is_err());
            assert!(!receipt.exists());
        } else {
            assert_eq!(result.unwrap(), original);
            assert_eq!(fs::read(&output).unwrap(), before);
            assert!(receipt.is_file());
        }
    }
}

#[test]
fn read_only_verification_requires_existing_completion_and_preserves_evidence() {
    let mut fixture = Fixture::new();
    fixture.ledger().claim(0, 0).unwrap();
    fixture.close();
    assert!(verify_finalized(fixture.root.path()).is_err());
    assert!(!fixture.root.path().join("attempts.final.json").exists());
    let expected = fixture.finalize().unwrap();
    let output = fixture.root.path().join("attempts.final.json");
    let receipt = fixture.root.path().join("attempts.complete.json");
    let before = fs::read(&output).unwrap();
    assert_eq!(verify_finalized(fixture.root.path()).unwrap(), expected);
    assert_eq!(fs::read(&output).unwrap(), before);
    fs::remove_file(&receipt).unwrap();
    assert!(verify_finalized(fixture.root.path()).is_err());
    assert!(
        !receipt.exists(),
        "verification must never recover the missing seal"
    );
}

#[test]
fn read_only_verification_recomputes_sealed_denominators_and_cleanup() {
    for changed in ["ledger", "claim", "guard", "completion", "orphan"] {
        let mut fixture = Fixture::new();
        fixture.ledger().claim(0, 0).unwrap();
        fixture.close();
        fixture.finalize().unwrap();
        let root = fixture.root.path();
        match changed {
            "ledger" => {
                let output = root.join("attempts.final.json");
                let mut ledger = files::read_json(&output).unwrap();
                ledger["counts"]["assigned"] = json!(0);
                files::atomic_json(&output, &ledger).unwrap();
                let receipt = root.join("attempts.complete.json");
                let mut completion = files::read_json(&receipt).unwrap();
                completion["ledger_sha256"] = json!(files::sha256(&output).unwrap());
                files::atomic_json(&receipt, &completion).unwrap();
            }
            "claim" => {
                let path = attempt_path(root, 0).join("claim.json");
                let mut claim = files::read_json(&path).unwrap();
                claim["worker"] = json!(99);
                files::atomic_json(&path, &claim).unwrap();
            }
            "guard" => {
                let mut guard = files::read_json(&fixture.job).unwrap();
                guard["cleanup_verified"] = json!(false);
                files::atomic_json(&fixture.job, &guard).unwrap();
            }
            "completion" => fs::write(root.join("attempts.complete.json"), b"{}").unwrap(),
            "orphan" => {
                fs::create_dir_all(root.join("records")).unwrap();
                fs::write(root.join("records/new.partial"), b"unsealed new evidence").unwrap();
            }
            _ => unreachable!(),
        }
        assert!(verify_finalized(root).is_err(), "{changed}");
    }
}

#[test]
fn native_claim_and_wait_helper() {
    let invocation = PathBuf::from(std::env::args_os().next().unwrap());
    if invocation
        .file_name()
        .is_none_or(|name| name != "research-child-helper")
    {
        return;
    }
    let root = invocation.parent().unwrap();
    let attempts = (0..3)
        .map(|deal| Attempt {
            deal,
            seed: 100 + deal / 2,
            focal_seat: deal as usize % 2,
            policy_seed: 77,
            deck_keys: vec!["left".into(), "right".into()],
        })
        .collect();
    let ledger = Ledger::create(root, attempts).unwrap();
    ledger.claim(0, 0).unwrap();
    // The external native guard must terminate this process. No engine, model,
    // network or GPU work runs in this fixture.
    loop {
        std::thread::park();
    }
}

#[test]
fn bounded_native_child_kill_is_cleaned_and_finalized_as_assigned_unknown() {
    let root = tempfile::tempdir().unwrap();
    files::atomic_json(
        &root.path().join("run.json"),
        &json!({"collection_contract":{"purpose":"failure_terminal_research_v1","version":2},
        "training_data":false,"planned_deals":3,"deal_offset":0}),
    )
    .unwrap();
    fs::write(root.path().join("games.jsonl"), b"").unwrap();
    let helper = root.path().join("research-child-helper");
    std::os::unix::fs::symlink(std::env::current_exe().unwrap(), &helper).unwrap();
    let job = crate::guard::Job {
        directory: root.path().join("guard-job"),
        cwd: std::env::current_dir().unwrap(),
        command: vec![
            helper.to_str().unwrap().to_owned(),
            "--exact".into(),
            "research_attempts::tests::native_claim_and_wait_helper".into(),
            "--".into(),
            "--collection-purpose".into(),
            "failure_terminal_research_v1".into(),
        ],
        timeout_seconds: 1.0,
        memory_gib: 1.0,
        inputs: vec![root.path().join("run.json")],
    };
    assert!(
        collect(
            &job,
            root.path(),
            &std::sync::atomic::AtomicBool::new(false)
        )
        .is_err()
    );
    let guard = files::read_json(&job.directory.join("job.json")).unwrap();
    assert_eq!(guard["reason"], "deadline");
    assert_eq!(guard["cleanup_verified"], true);
    assert!(
        guard["returncode"].is_null(),
        "child must have been killed, not completed"
    );
    let producer: process::Identity =
        serde_json::from_value(guard["child_identity"].clone()).unwrap();
    assert_ne!(process::identity(producer.pid).as_ref(), Some(&producer));
    let ledger = files::read_json(&root.path().join("attempts.final.json")).unwrap();
    assert_eq!(
        ledger["counts"],
        json!({"planned":3,"assigned":1,"recorded":0,"operational_unknown":1,"not_started":2})
    );
    assert_eq!(ledger["attempts"][0]["unknown_reason"], "process_deadline");
    assert!(ledger["attempts"][0]["training_utility"].is_null());
    assert!(root.path().join("attempts.complete.json").is_file());
}
