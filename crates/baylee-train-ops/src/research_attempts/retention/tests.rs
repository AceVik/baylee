use super::*;
use flate2::{Compression, write::GzEncoder};

struct Table {
    root: tempfile::TempDir,
    job: guard::Job,
    wrapper: PathBuf,
}

impl Table {
    fn new(mode: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        files::atomic_json(
            &root.path().join("run.json"),
            &json!({
            "collection_contract":{"purpose":"failure_terminal_research_v1","version":2},
            "training_data":false,"planned_deals":3,"deal_offset":0}),
        )
        .unwrap();
        fs::write(root.path().join("games.jsonl"), b"").unwrap();
        fs::write(root.path().join("mode"), mode).unwrap();
        fs::write(root.path().join("sealed-input"), "original").unwrap();
        for name in ["research-child-helper", "research-wrapper-helper"] {
            std::os::unix::fs::symlink(std::env::current_exe().unwrap(), root.path().join(name))
                .unwrap();
        }
        let job = job(root.path());
        let wrapper = root.path().join("research-wrapper-helper");
        Self { root, job, wrapper }
    }
}

fn job(root: &Path) -> guard::Job {
    guard::Job {
        directory: root.join("guard"),
        cwd: std::env::current_dir().unwrap(),
        command: vec![
            root.join("research-child-helper").to_str().unwrap().into(),
            "--exact".into(),
            "research_attempts::retention::tests::native_child".into(),
            "--".into(),
            "--collection-purpose".into(),
            "failure_terminal_research_v1".into(),
        ],
        timeout_seconds: if fs::read_to_string(root.join("mode")).unwrap() == "complete" {
            10.
        } else {
            1.
        },
        memory_gib: 1.,
        inputs: vec![root.join("sealed-input")],
    }
}

fn completed(ledger: &Ledger, root: &Path, deal: u64) {
    fs::create_dir_all(root.join("records")).unwrap();
    let raw = b"{\"kind\":\"header\"}\n";
    let mut gzip = GzEncoder::new(Vec::new(), Compression::fast());
    gzip.write_all(raw).unwrap();
    let compressed = gzip.finish().unwrap();
    fs::write(root.join("records/game.gz"), &compressed).unwrap();
    let episode = json!({"kind":"research_termination","termination_kind":"unknown",
        "training_utility":null,"bootstrap":false});
    let row = json!({"i":deal,"seed":100+deal/2,"focal_seat":deal%2,"policy_seed":77,
        "source_run_sha256":ledger.plan.source_run_sha256,"outcome":episode,"research_episode":episode,
        "shard":"game.gz","offset":0,"len":compressed.len(),
        "record_sha256":files::hex(&sha2::Sha256::digest(raw))});
    files::append_json(&root.join("games.jsonl"), &row).unwrap();
    ledger.completed(&row).unwrap();
}

#[test]
fn native_child() {
    let invocation = PathBuf::from(std::env::args_os().next().unwrap());
    if invocation
        .file_name()
        .is_none_or(|name| name != "research-child-helper")
    {
        return;
    }
    let root = invocation.parent().unwrap();
    let mode = fs::read_to_string(root.join("mode")).unwrap();
    if mode != "missing" {
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
        completed(&ledger, root, 0);
        ledger.claim(1, 1).unwrap();
        match mode.as_str() {
            "complete" => {
                completed(&ledger, root, 1);
                ledger.claim(2, 2).unwrap();
                completed(&ledger, root, 2);
                return;
            }
            "worker_error" => {
                ledger.worker_failed(1, "worker_error").unwrap();
                panic!("fixture worker error");
            }
            "input_mutation" => fs::write(root.join("sealed-input"), "changed").unwrap(),
            "malformed" => fs::write(attempt_path(root, 1).join("claim.json"), "{}").unwrap(),
            _ => {}
        }
        // Keep the live ledger lock until the real native guard kills this process.
        loop {
            std::thread::park();
        }
    }
    loop {
        std::thread::park();
    }
}

#[test]
fn native_wrapper() {
    let invocation = PathBuf::from(std::env::args_os().next().unwrap());
    if invocation
        .file_name()
        .is_none_or(|name| name != "research-wrapper-helper")
    {
        return;
    }
    let root = invocation.parent().unwrap();
    collect_retaining_deadline(&job(root), root, &AtomicBool::new(false)).unwrap();
}

#[test]
fn real_deadline_accounting_allows_next_independent_pipeline_command() {
    let first = Table::new("deadline");
    let second = Table::new("complete");
    let pipeline = tempfile::tempdir().unwrap();
    let stages: Vec<_> = [(&first, "first"), (&second, "second")]
        .into_iter()
        .map(|(table, name)| {
            json!({"name":name,"command":[table.wrapper,"--exact",
            "research_attempts::retention::tests::native_wrapper"],
            "timeout_seconds":20.,"memory_gib":1.,
            "required_outputs":[table.root.path().join("collection-accounting.json")]})
        })
        .collect();
    let manifest = pipeline.path().join("plan.json");
    files::atomic_json(
        &manifest,
        &json!({"version":1,"root":pipeline.path().join("execution"),
        "cwd":std::env::current_dir().unwrap(),"total_timeout_seconds":50.,"stages":stages}),
    )
    .unwrap();
    if let Err(error) = crate::pipeline::run(&manifest, &AtomicBool::new(false)) {
        panic!(
            "{error:#}; second inner guard={:?}; child stderr={:?}; wrapper stderr={:?}",
            fs::read_to_string(second.job.directory.join("job.json")),
            fs::read_to_string(second.job.directory.join("stderr.log")),
            fs::read_to_string(pipeline.path().join("execution/jobs/second/stderr.log"))
        );
    }
    let receipt = files::read_json(&first.root.path().join("collection-accounting.json")).unwrap();
    assert_eq!(receipt["status"], "deadline_accounted");
    assert_eq!(receipt["collection_complete"], false);
    assert_eq!(receipt["guard_status"], "quarantined");
    assert_eq!(
        receipt["counts"],
        json!({"planned":3,"assigned":2,"recorded":1,
        "operational_unknown":1,"not_started":1})
    );
    let ledger = verify_finalized(first.root.path()).unwrap();
    assert_eq!(ledger["attempts"][1]["unknown_reason"], "process_deadline");
    assert!(ledger["attempts"][1]["training_utility"].is_null());
    let receipt = files::read_json(&second.root.path().join("collection-accounting.json")).unwrap();
    assert_eq!(receipt["status"], "collection_complete");
    assert_eq!(receipt["collection_complete"], true);
}

#[test]
fn retained_mode_still_rejects_worker_errors_changed_inputs_and_invalid_ledgers() {
    for mode in ["worker_error", "input_mutation", "malformed", "missing"] {
        let table = Table::new(mode);
        assert!(
            collect_retaining_deadline(&table.job, table.root.path(), &AtomicBool::new(false))
                .is_err(),
            "{mode}"
        );
        assert!(
            !table
                .root
                .path()
                .join("collection-accounting.json")
                .exists(),
            "{mode}"
        );
        if mode == "input_mutation" {
            let guard = files::read_json(&table.job.directory.join("job.json")).unwrap();
            assert_eq!(
                guard["reason"], "deadline",
                "must reject mutation hidden by deadline reason"
            );
            assert!(verify_finalized(table.root.path()).is_ok());
        }
    }
}

#[test]
fn retained_deadline_rejects_memory_interruption_and_unverified_cleanup() {
    let table = Table::new("deadline");
    let receipt =
        collect_retaining_deadline(&table.job, table.root.path(), &AtomicBool::new(false)).unwrap();
    let ledger = verify_finalized(table.root.path()).unwrap();
    let guard = files::read_json(&table.job.directory.join("job.json")).unwrap();
    for (key, value) in [
        ("reason", json!("memory_limit")),
        ("reason", json!("interrupted")),
        ("cleanup_verified", json!(false)),
        ("status", json!("running")),
        ("returncode", json!(1)),
        ("peak_pss_bytes", json!(2_147_483_648_u64)),
    ] {
        let mut changed = guard.clone();
        changed[key] = value;
        assert!(deadline(&table.job, &changed, &ledger).is_err(), "{key}");
    }
    let mut changed = ledger.clone();
    changed["attempts"][1]["unknown_reason"] = json!("worker_panic");
    assert!(deadline(&table.job, &guard, &changed).is_err());
    assert!(account(&table.job, table.root.path(), &AtomicBool::new(true), false).is_err());
    assert_eq!(
        files::read_json(&table.root.path().join("collection-accounting.json")).unwrap(),
        receipt
    );
}
