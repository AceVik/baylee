use std::fs;
use std::path::Path;

use serde_json::json;

use super::replay::findings;
use super::*;

/// An eval4 summary of two deals, as eval4 writes one.
fn summary(house_fallbacks: u64, net_answers: u64) -> serde_json::Value {
    json!({
        "deals": 2, "refused": 1, "house_fallbacks": house_fallbacks, "stalls": 1,
        "net_answers": net_answers, "net_answer_caps": 1, "evaluator_build": "abc1234",
        "deal_details": [
            {"deal": 0, "seed": 1000, "table": 0, "focal_seat": 0, "net_capped": false,
             "base_capped": null, "refused": 1, "stalls": 1,
             "fallback_events": [
                {"seat": 0, "reason": "engine_refusal", "action": {"CastSpell": {"card": "ObjectId(3#0)"}},
                 "decision_seq": 12, "error": "IllegalAction(\"not castable\")", "accepted": false},
                {"seat": 0, "reason": "outer_repeated_view", "view_hash": "ab", "visits": 5},
                {"seat": 0, "diagnostic": {"reason": "repeated_view"}, "action": "PassPriority"},
                {"seat": 0, "diagnostic": {"reason": "inference_or_decode"}, "action": "PassPriority"}
             ],
             "search": {"refused": {"Continuation(\"activating_abilities\")": 3,
                                    "Continuation(\"pending_plan\")": 2}}},
            {"deal": 1, "seed": 1000, "table": 0, "focal_seat": 1, "net_capped": true,
             "base_capped": false, "refused": 0, "stalls": 0, "fallback_events": [],
             "search": null}
        ]
    })
}

fn job() -> Job {
    Job::of(
        [
            "/rt/eval4",
            "--model",
            "m",
            "--deals",
            "512",
            "--threads",
            "16",
            "--out",
            "/r/x.json",
            "--first-seed",
            "1000",
        ]
        .map(str::to_owned)
        .to_vec(),
        Path::new("/nowhere"),
    )
}

fn find(report: &Report, kind: Kind) -> Vec<&Entry> {
    report.entries.iter().filter(|e| e.kind == kind).collect()
}

#[test]
fn an_eval4_summary_lists_every_counted_anomaly_with_its_deal_and_seed() {
    let report = from_eval4(
        &summary(10, 9_990),
        Path::new("/r/eval-x.json"),
        Some(&job()),
    );
    assert_eq!(report.run, "eval-x");
    assert_eq!(report.games, 2);
    let refusal = find(&report, Kind::EngineRefusal);
    assert_eq!(refusal.len(), 1);
    assert_eq!(
        (
            refusal[0].deal,
            refusal[0].seed,
            refusal[0].table,
            refusal[0].seat
        ),
        (Some(0), Some(1000), Some(0), Some(0))
    );
    assert_eq!(refusal[0].class, Class::New);
    assert!(
        refusal[0].what.contains("not castable"),
        "{}",
        refusal[0].what
    );
    let reproduce = refusal[0].reproduce.as_deref().unwrap();
    assert!(
        reproduce.ends_with("--deals 1 --deal-offset 0 --threads 1 --out reproduce-deal-0.json")
    );
    assert!(
        !reproduce.contains("512") && !reproduce.contains("/r/x.json"),
        "{reproduce}"
    );

    // Fallbacks are 0.1% of the net's answers: plain ones are expected, a
    // failed inference never is.
    let fallbacks = find(&report, Kind::HouseFallback);
    let class_of = |reason: &str| {
        fallbacks
            .iter()
            .find(|e| e.what.contains(reason))
            .map(|e| e.class)
    };
    assert_eq!(class_of("repeated_view"), Some(Class::Expected));
    assert_eq!(class_of("inference_or_decode"), Some(Class::New));
    assert_eq!(find(&report, Kind::Stall)[0].class, Class::Expected);

    let caps = find(&report, Kind::AnswerCap);
    assert_eq!(caps.len(), 1);
    assert_eq!((caps[0].deal, caps[0].seat), (Some(1), Some(1)));

    let roots = find(&report, Kind::RefusedRoot);
    let root = |reason: &str| roots.iter().find(|e| e.what.contains(reason)).unwrap();
    assert_eq!(root("activating_abilities").class, Class::Known);
    assert_eq!(root("activating_abilities").count, 3);
    assert_eq!(root("pending_plan").class, Class::Expected);
    assert_eq!(report.counts["refused_root"], 5);
    assert_eq!(report.counts["refused_root/known"], 3);
}

#[test]
fn fallbacks_above_the_threshold_are_worth_a_look() {
    let report = from_eval4(&summary(500, 1_000), Path::new("/r/eval-x.json"), None);
    assert!(
        find(&report, Kind::HouseFallback)
            .iter()
            .all(|e| e.class == Class::New)
    );
    assert_eq!(find(&report, Kind::Stall)[0].class, Class::New);
    assert!(report.entries.iter().all(|e| e.reproduce.is_none()));
}

#[test]
fn the_job_beside_a_result_is_found_by_its_out() {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("eval-y.json");
    fs::write(&result, "{}").unwrap();
    let job_dir = dir.path().join("execution-q1/jobs/y");
    fs::create_dir_all(&job_dir).unwrap();
    fs::write(job_dir.join("stderr.log"), "").unwrap();
    let records = dir.path().join("records-y");
    fs::write(
        job_dir.join("job.json"),
        serde_json::to_vec(&json!({"argv": [
            "/rt/v1/eval4", "--out", result, "--record-out", records
        ]}))
        .unwrap(),
    )
    .unwrap();
    let other = dir.path().join("execution-q1/jobs/other");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        other.join("job.json"),
        serde_json::to_vec(&json!({"argv": ["/rt/v2/eval4", "--out", "/elsewhere.json"]})).unwrap(),
    )
    .unwrap();
    let job = job_of(&result).unwrap();
    assert_eq!(job.runtime.as_deref(), Some(Path::new("/rt/v1")));
    assert_eq!(job.records.as_deref(), Some(records.as_path()));
    assert_eq!(job.stderr, Some(job_dir.join("stderr.log")));
    assert!(job_of(&dir.path().join("missing.json")).is_none());
}

#[test]
fn panics_in_a_log_are_new() {
    let found =
        panics_in("[eval4] deal 3\nthread 'w2' panicked at crates/engine/src/x.rs:4:5:\nboom\nok");
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].kind, found[0].class), (Kind::Panic, Class::New));
}

#[test]
fn a_run_lists_caps_panics_short_records_and_refusals() {
    let games = [
        json!({"i": 0, "seed": 7, "outcome": {"kind": "won", "seat": 0}}),
        json!({"i": 1, "seed": 7, "outcome": {"kind": "answer_cap"}}),
        json!({"i": 2, "seed": 8, "outcome": {"kind": "time_cap"}}),
        json!({"i": 3, "seed": 8, "outcome": {"kind": "panicked", "message": "index out of bounds"}}),
        json!({"i": 4, "seed": 9, "focal_seat": 1, "complete": false, "refused_answers": 1,
               "outcome": {"kind": "won", "seat": 1},
               "fallback_events": [{"seat": 1, "reason": "engine_refusal", "action": "PassPriority",
                                    "decision_seq": 4, "error": "MismatchedAction"}]}),
    ]
    .map(|g| g.to_string())
    .join("\n");
    let report = from_run(
        &games,
        Some(&json!({"evaluator_build": "b"})),
        Path::new("/x/run-a"),
    );
    assert_eq!(report.games, 5);
    assert_eq!(find(&report, Kind::AnswerCap)[0].deal, Some(1));
    assert_eq!(find(&report, Kind::NoEnd).len(), 2);
    assert!(
        find(&report, Kind::Panic)[0]
            .what
            .contains("index out of bounds")
    );
    let refusal = find(&report, Kind::EngineRefusal);
    assert_eq!((refusal[0].deal, refusal[0].seat), (Some(4), Some(1)));
    assert_eq!(report.totals["refused_answers"], 1);
}

#[test]
fn a_ladder_names_its_seat_failures_and_knows_the_sign_in_limit() {
    let dir = tempfile::tempdir().unwrap();
    for (game, log) in [
        (0, "fine"),
        (
            2,
            "Error: sign in as a guest: the gateway answered 429 (too many attempts)",
        ),
        (3, "thread 'main' panicked at crates/seat/src/a.rs:1:1"),
    ] {
        let at = dir.path().join(format!("game-{game:04}"));
        fs::create_dir_all(&at).unwrap();
        fs::write(at.join("seat4.stderr.log"), log).unwrap();
    }
    let ladder = json!({"games": [
        {"game": 0, "phase": "game-complete", "timed_out": false, "failure": null, "seconds": 9.0},
        {"game": 1, "phase": null, "timed_out": true, "failure": null, "seconds": 900.0},
        {"game": 2, "error": "seat4 ended before it opened a room"},
        {"game": 3, "phase": "game-complete", "timed_out": false, "failure": "lost the socket"},
        {"game": 4, "not_started": "stopped"}
    ], "summary": {"x": {"games": 4}}});
    let report = from_ladder(&ladder, dir.path());
    assert_eq!(report.games, 4);
    assert_eq!(find(&report, Kind::SeatTimeout)[0].deal, Some(1));
    let errors = find(&report, Kind::SeatError);
    assert_eq!(errors.len(), 2);
    assert_eq!(
        errors.iter().find(|e| e.deal == Some(2)).unwrap().class,
        Class::Known
    );
    assert_eq!(
        errors.iter().find(|e| e.deal == Some(3)).unwrap().class,
        Class::New
    );
    assert_eq!(find(&report, Kind::Panic)[0].deal, Some(3));
    assert!(find(&report, Kind::NoEnd).is_empty());
}

#[test]
fn the_sample_is_fixed_by_seed_and_deal_and_about_one_in_sixty_four() {
    let chosen = (0..64_000_u64)
        .filter(|deal| sampled(1_130_301_000_000_000_000 + deal / 2, *deal, 64))
        .count();
    assert!((800..1_200).contains(&chosen), "{chosen}");
    assert_eq!(sampled(5, 6, 64), sampled(5, 6, 64));
    assert!((0..100).all(|deal| sampled(9, deal, 1)));
    assert!((0..100).all(|deal| !sampled(9, deal, 0)));
}

#[test]
fn replay_findings_tell_the_boundary_and_another_build_from_a_bug() {
    let rejected = [
        json!({"game_id": 1, "reason": "record written before activator boundary eb40dc593: written by a"}),
        json!({"game_id": 2, "reason": "replay audit: Diverged { n: Some(4), recorded: \"1\", replayed: \"2\" }"}),
        json!({"game_id": 3, "reason": "unencoded decisions", "count": 2}),
        json!({"game_id": 3, "reason": "replay audit: Refused { n: 9 }"}),
    ];
    let places = [(10, 100, Some(0)), (11, 101, Some(1)), (12, 102, None)];
    let same = findings(&rejected, "", None, &places, Some(true));
    assert_eq!(same.len(), 3, "the encoding rejection is not the replay's");
    assert_eq!(
        (same[0].kind, same[0].class),
        (Kind::Boundary, Class::Known)
    );
    assert_eq!(
        (same[1].kind, same[1].class, same[1].deal, same[1].seed),
        (Kind::ReplayDivergence, Class::New, Some(11), Some(101))
    );
    assert_eq!(
        (same[2].kind, same[2].class),
        (Kind::ReplayRefusal, Class::New)
    );
    let other = findings(&rejected, "", None, &places, Some(false));
    assert_eq!(other[0].class, Class::Known);
    assert_eq!(other[1].class, Class::Expected);
    assert!(other[1].what.contains("another build"));
    let panicked = findings(
        &[],
        "thread 'w0' panicked at engine.rs:1:1",
        Some(3),
        &places,
        Some(false),
    );
    assert_eq!(
        (panicked[0].kind, panicked[0].class, panicked[0].deal),
        (Kind::ReplayPanic, Class::New, Some(12))
    );
}

#[test]
fn the_harvest_adds_a_place_once_however_many_runs_met_it() {
    let dir = tempfile::tempdir().unwrap();
    let list = dir.path().join("bug-harvest.md");
    fs::write(
        &list,
        "# Bug harvest\n\n| Date | Run | Kind | Seed / deal | What | Status |\n| --- | --- | --- | --- | --- | --- |\n",
    )
    .unwrap();
    let report = |name: &str| {
        let mut report = Report::new(Path::new(&format!("/r/{name}.json")), "eval4");
        report.entries.push(
            Entry::new(
                Kind::AnswerCap,
                Class::New,
                "the net game hit the answer cap",
            )
            .at(Some(379), Some(189), Some(189))
            .seat(Some(1)),
        );
        report
            .entries
            .push(Entry::new(Kind::HouseFallback, Class::Expected, "plain"));
        report
    };
    let mut first = report("eval-a");
    first.entries.push(
        Entry::new(Kind::EngineRefusal, Class::New, "refused a | b")
            .at(Some(3), Some(42), Some(1))
            .seat(Some(0)),
    );
    first.entries.push(Entry::new(
        Kind::Panic,
        Class::New,
        "thread 'w' panicked at x.rs:1:1",
    ));
    // Two ladders' game 9: no seed, so one row each.
    let ladder = |name: &str| {
        let mut report = Report::new(Path::new(&format!("/l/{name}")), "ladder4");
        report.entries.push(
            Entry::new(Kind::SeatTimeout, Class::New, "ran out of time").at(Some(9), None, None),
        );
        report
    };
    let reports = [
        first,
        report("eval-b"),
        report("eval-c"),
        ladder("sh1"),
        ladder("sh2"),
    ];
    assert_eq!(harvest(&list, &reports, "2026-11-10").unwrap(), 5);
    let text = fs::read_to_string(&list).unwrap();
    assert!(text.contains(
        "| 2026-11-10 | eval-a +2 more | answer cap | seed 189, deal 379, table 189, seat 1 | the net game hit the answer cap (in 3 runs) | open |"
    ), "{text}");
    assert!(text.contains(
        "| 2026-11-10 | eval-a | engine refusal | seed 42, deal 3, table 1, seat 0 | refused a \\| b | open |"
    ));
    assert!(text.contains("| eval-a | panic | the run | thread 'w' panicked at x.rs:1:1 |"));
    assert!(text.contains("| sh1 | seat timeout | deal 9 | ran out of time |"));
    assert!(text.contains("| sh2 | seat timeout | deal 9 | ran out of time |"));
    assert!(!text.contains("plain"));
    // A later harvest of another run meeting the same deal adds nothing.
    assert_eq!(
        harvest(&list, &[report("eval-d")], "2026-11-11").unwrap(),
        0
    );
    assert_eq!(harvest(&list, &reports, "2026-11-11").unwrap(), 0);
}

#[test]
fn the_markdown_names_the_counts_the_new_entries_and_the_audit() {
    let mut report = from_eval4(
        &summary(10, 9_990),
        Path::new("/r/eval-x.json"),
        Some(&job()),
    );
    report.replay = Replay::not_run("no records");
    let text = markdown(&report);
    assert!(text.starts_with("# Anomalies: eval-x"));
    assert!(text.contains("| engine_refusal | 1 |"));
    assert!(text.contains("**engine refusal** seed 1000, deal 0, table 0, seat 0"));
    assert!(text.contains("no records."));
    assert!(text.contains("## Not recorded"));
}

#[test]
fn a_job_without_its_result_is_new_only_when_it_panicked() {
    let stopped = from_job(
        &json!({"status": "quarantined", "reason": "deadline", "returncode": null}),
        "[eval4] 100/512\n",
        Path::new("/x/jobs/a"),
    );
    assert_eq!(stopped.entries.len(), 1);
    assert_eq!(
        (stopped.entries[0].kind, stopped.entries[0].class),
        (Kind::RunFailed, Class::Expected)
    );
    let crashed = from_job(
        &json!({"status": "quarantined", "returncode": 101}),
        "thread 'w3' panicked at crates/baylee-engine/src/a.rs:9:1:\nboom\n",
        Path::new("/x/jobs/b"),
    );
    assert_eq!(crashed.counts["run_failed/new"], 1);
    assert_eq!(crashed.counts["panic/new"], 1);
    assert!(
        from_job(&json!({"status": "complete"}), "", Path::new("/x/jobs/c"))
            .entries
            .is_empty()
    );
}

#[test]
fn the_counter_watch_flags_a_run_whose_counters_go_astray() {
    let counted = |poor: u32, total: u32| {
        json!({"counterspells": total, "poor": poor,
               "poor_share": if total == 0 { serde_json::Value::Null } else { json!(f64::from(poor) / f64::from(total)) },
               "cheap_at_most": 1, "by_whose": {"opponent": total - poor, "own": poor},
               "by_kind": {"creature": total}})
    };
    assert!(super::counters::entries(&counted(0, 0)).is_empty());
    let calm = super::counters::entries(&counted(1, 10));
    assert_eq!(
        (calm[0].kind, calm[0].class, calm[0].count),
        (Kind::CounterTargets, Class::Expected, 1)
    );
    let astray = super::counters::entries(&counted(4, 10));
    assert_eq!(astray[0].class, Class::New);
    assert!(
        astray[0].what.starts_with("4 of 10 counterspells"),
        "{}",
        astray[0].what
    );
    let mut report = Report::new(Path::new("/r/eval-c.json"), "eval4");
    report.counters = counted(4, 10);
    assert!(markdown(&report).contains("10 counterspells by the net's seat"));
}
