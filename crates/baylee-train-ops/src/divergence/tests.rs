use super::*;

fn line(i: u64, n: u32, kind: &str, action: Value, margin: Option<f64>) -> TraceLine {
    TraceLine {
        i,
        n,
        seq: u64::from(n) * 3,
        turn: n / 4 + 1,
        seat: 0,
        kind: kind.to_owned(),
        head: Some(0),
        source: "Net".to_owned(),
        action,
        margin,
        margin_kind: margin.map(|_| "top2".to_owned()),
        options: Some(3),
    }
}

#[test]
fn the_margin_is_top_two_for_a_choice_and_the_closest_bit_for_a_mask() {
    let single = json!({"head": 0, "options": [{"logit": 1.5}, {"logit": -0.5}, {"logit": 1.25}], "pairs": []});
    assert_eq!(menu_margin(&single), Some((0.25, "top2", 0, 3)));
    let subset = json!({"head": 1, "options": [{"logit": 2.0}, {"logit": -0.125}], "pairs": []});
    assert_eq!(menu_margin(&subset), Some((0.125, "closest_bit", 1, 2)));
    let pairs = json!({"head": 2, "options": [], "pairs": [{"logit": -3.0}, {"logit": 0.5}]});
    assert_eq!(menu_margin(&pairs), Some((0.5, "closest_bit", 2, 2)));
    assert_eq!(
        menu_margin(&json!({"head": 3, "options": [], "pairs": []})),
        None
    );
    assert_eq!(
        menu_margin(&json!({"head": 0, "options": [{"logit": 1.0}], "pairs": []})),
        None
    );
    assert_eq!(menu_margin(&Value::Null), None);
}

#[test]
fn the_first_differing_answer_is_where_two_games_part() {
    let pass = json!("PassPriority");
    let cast = json!({"CastSpell": {"card": 7}});
    let reference = vec![
        line(4, 0, "Mulligan", json!("MulliganKeep"), Some(3.0)),
        line(4, 1, "Priority", pass.clone(), Some(0.2)),
        line(4, 2, "Priority", pass.clone(), Some(1.0)),
    ];
    let mut other = reference.clone();
    other[1].action = cast.clone();
    other[1].margin = Some(9.0);
    let parted = first_divergence(4, &reference, &other);
    assert_eq!(parted.n, Some(1));
    assert_eq!(parted.kind.as_deref(), Some("Priority"));
    assert_eq!(parted.margin, Some(0.2), "the reference run's margin");
    assert_eq!((parted.reference, parted.other), (Some(pass), Some(cast)));
    assert!(!parted.ended);
    let same = first_divergence(4, &reference, &reference);
    assert_eq!(same.n, None);
    let shorter = first_divergence(4, &reference, &reference[..2]);
    assert_eq!(shorter.n, Some(2));
    assert!(shorter.ended);
}

#[test]
fn buckets_cover_every_margin() {
    assert_eq!(bucket(Some(0.05)), "<0.1");
    assert_eq!(bucket(Some(0.1)), "0.1-0.25");
    assert_eq!(bucket(Some(0.99)), "0.5-1");
    assert_eq!(bucket(Some(7.0)), ">=2");
    assert_eq!(bucket(None), "none");
}

/// Two record directories of one deal set: the report pairs them by deal and
/// refuses runs of other deals.
#[test]
fn two_runs_of_the_same_deals_report_per_deal_and_by_kind() {
    let root = tempfile::tempdir().unwrap();
    let write = |name: &str, reset: Option<&str>, lines: &[TraceLine], seed: u64| {
        let dir = root.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut run = json!({"deal_offset": 0, "planned_deals": 2, "decks": [], "seats": 2,
            "deck_split": "test", "decks_manifest": null, "policy_mode": "raw",
            "tactics_rules": "", "choice_guard": "off", "temperature": 0.0});
        if let Some(reset) = reset {
            run["memory_reset"] = json!(reset);
        }
        std::fs::write(dir.join("run.json"), run.to_string()).unwrap();
        let games = format!(
            "{}\n{}\n",
            json!({"i": 0, "seed": seed, "policy_seed": 1, "focal_seat": 0}),
            json!({"i": 1, "seed": seed, "policy_seed": 1, "focal_seat": 1})
        );
        std::fs::write(dir.join("games.jsonl"), games).unwrap();
        let trace: String = lines
            .iter()
            .map(|row| serde_json::to_string(row).unwrap() + "\n")
            .collect();
        std::fs::write(dir.join(TRACE_FILE), trace).unwrap();
        dir
    };
    let normal = [
        line(0, 0, "Mulligan", json!("MulliganKeep"), Some(2.5)),
        line(0, 1, "Priority", json!("PassPriority"), Some(0.05)),
        line(1, 0, "Mulligan", json!("MulliganKeep"), Some(2.5)),
    ];
    let mut zero = normal.clone();
    zero[1].action = json!({"PlayLand": {"card": 3}});
    let a = write("normal", None, &normal, 9);
    let b = write("zero", Some("decision"), &zero, 9);
    let report = diverge(&a, &b).unwrap();
    assert_eq!(report["deals"], 2);
    assert_eq!(report["diverged"], 1);
    assert_eq!(report["identical"], 1);
    assert_eq!(report["other_memory_reset"], "decision");
    assert_eq!(report["reference_memory_reset"], Value::Null);
    assert_eq!(report["by_kind"]["Priority"]["<0.1"], 1);
    assert_eq!(report["first_divergence"][0]["n"], 1);
    assert!(table(&report).contains("| Priority | 1 |"));
    let elsewhere = write("elsewhere", Some("decision"), &zero, 10);
    assert!(
        diverge(&a, &elsewhere).is_err(),
        "other seeds are other deals"
    );
    let mut gap = normal.clone();
    gap[1].n = 2;
    let gapped = write("gapped", None, &gap, 9);
    assert!(read_trace(&gapped).is_err());
}
