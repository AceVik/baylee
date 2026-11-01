use super::*;
use serde_json::json;
fn golden() -> Value {
    serde_json::from_str(include_str!("fixtures/python_golden.json")).unwrap()
}
fn comparison(c: &Value, r: &Value) -> Result<Value> {
    let g = golden();
    compare(c, r, &g["provenance"], &g["provenance"], 100, false)
}
#[test]
fn python_golden_metrics_and_reporting_match() {
    let g = golden();
    let mut actual = comparison(&g["candidate"], &g["reference"]).unwrap();
    let mut expected = g["comparison"].clone();
    // Bootstrap stream is explicitly versioned; compare every deterministic field.
    actual["paired_uncertainty"]
        .as_object_mut()
        .unwrap()
        .remove("bootstrap_rng");
    actual["paired_uncertainty"]
        .as_object_mut()
        .unwrap()
        .remove("table_bootstrap_95_percent_interval");
    expected["paired_uncertainty"]
        .as_object_mut()
        .unwrap()
        .remove("table_bootstrap_95_percent_interval");
    assert_eq!(actual, expected);
}
#[test]
fn worker_order_does_not_change_statistics() {
    let g = golden();
    let c = g["candidate"].clone();
    let mut r = g["reference"].clone();
    let expected = comparison(&c, &r).unwrap();
    r["deal_details"].as_array_mut().unwrap().reverse();
    assert_eq!(comparison(&c, &r).unwrap(), expected);
}
#[test]
fn cap_is_unknown_and_analytic_bounds_match_python() {
    let g = golden();
    let row = json!({"table":0,"seed":1,"decks":["a","b"],"net_utility":0.5,"base_utility":1.,"net_capped":true,"base_capped":false});
    let mut report = summarize(&json!({"deal_details":[row]}), 100).unwrap();
    report.as_object_mut().unwrap().remove("bootstrap_rng");
    assert_eq!(report, g["capped_summary"]);
}
#[test]
fn focal_rotations_do_not_multiply_independence() {
    let rows:Vec<_>=(0..8).map(|i|json!({"table":i,"seed":i,"decks":["a","b"],"net_utility":f64::from(i%2),"base_utility":0.5,"net_capped":false,"base_capped":false})).collect();
    let a = summarize(&json!({"deal_details":rows}), 100).unwrap();
    let b = summarize(
        &json!({"deal_details":(0..8).flat_map(|_| rows.clone()).collect::<Vec<_>>()}),
        100,
    )
    .unwrap();
    for k in [
        "independent_tables",
        "table_bootstrap_95_percent_interval",
        "bounded_independent_table_95_percent_interval",
    ] {
        assert_eq!(a[k], b[k]);
    }
}
#[test]
fn protocol_and_case_mismatches_rejected() {
    let g = golden();
    let c = &g["candidate"];
    for key in [
        "against",
        "thinking_steps",
        "policy_mode",
        "policy_contract",
    ] {
        let mut r = c.clone();
        r[key] = json!("different");
        assert!(comparison(c, &r).is_err(), "{key}");
    }
    for key in ["seed", "decks", "focal_seat", "base_utility"] {
        let mut r = c.clone();
        r["deal_details"][0][key] = json!(999);
        assert!(comparison(c, &r).is_err(), "{key}");
    }
    let mut r = c.clone();
    r["deal_details"].as_array_mut().unwrap().pop();
    assert!(comparison(c, &r).is_err());
    let mut r = c.clone();
    let row = r["deal_details"][0].clone();
    r["deal_details"].as_array_mut().unwrap().push(row);
    assert!(comparison(c, &r).is_err());
}
#[test]
fn depth_exception_keeps_other_protocol_checks() {
    let g = golden();
    let c = &g["candidate"];
    let mut r = c.clone();
    r["thinking_steps"] = json!(4);
    assert!(comparison(c, &r).is_err());
    let p = &g["provenance"];
    let result = compare(c, &r, p, p, 100, true).unwrap();
    assert_eq!(result["thinking_steps"]["reference"], 4);
    r["against"] = json!("other");
    assert!(compare(c, &r, p, p, 100, true).is_err());
}
#[test]
fn invalid_statistics_fail_closed() {
    assert!(summarize(&json!({"deal_details":[]}), 100).is_err());
    assert!(summarize(&golden()["candidate"], 0).is_err());
    let mut c = golden()["candidate"].clone();
    c["deal_details"][0]["net_utility"] = json!(2.0);
    assert!(summarize(&c, 100).is_err());
}
#[test]
fn raw_boolean_flag_preserves_next_option_and_frozen_job_binding() {
    let root = std::env::temp_dir().join(format!("baylee-eval-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&root).unwrap();
    let outcome = (|| -> Result<()> {
        let path = root.join("evaluation.json");
        let job = root.join("job.json");
        let mut c = golden()["candidate"].clone();
        c["decks_manifest"] = json!("decks.json");
        atomic_json(&path, &c)?;
        let mut run = json!({"status":"complete","executable_sha256":"engine","inputs":{"decks.json":"decks"},"command":["eval4","--raw-policy","--out",path,"--model","test","--decks-manifest","decks.json","--max-decisions","42"]});
        atomic_json(&job, &run)?;
        let (_, p, h) = read_verified(&path, &job)?;
        assert_eq!(p["max_decisions"], 42);
        assert_eq!(h.len(), 64);
        // The catalog flags take no value either.
        run["command"] = json!([
            "eval4",
            "--catalog-superset",
            "--catalog-drift",
            "--out",
            path,
            "--model",
            "test",
            "--decks-manifest",
            "decks.json",
            "--max-decisions",
            "41",
            "--search",
            "4,2,4"
        ]);
        atomic_json(&job, &run)?;
        assert_eq!(read_verified(&path, &job)?.1["max_decisions"], 41);
        run["status"] = json!("running");
        atomic_json(&job, &run)?;
        assert!(read_verified(&path, &job).is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(root).unwrap();
    outcome.unwrap();
}

#[test]
fn heterogeneous_table_bootstrap_agrees_with_archived_numpy_distribution() {
    let g = golden();
    let actual = summarize(&g["heterogeneous_tables"], 20_000).unwrap();
    let expected = &g["heterogeneous_summary"];
    for key in [
        "paired_difference_with_caps_imputed_half",
        "censoring_bounds_on_observed_tables",
        "bounded_independent_table_95_percent_interval",
    ] {
        let a = &actual[key];
        let b = &expected[key];
        if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
            assert!((a - b).abs() < 1e-12);
        } else {
            for (a, b) in a.as_array().unwrap().iter().zip(b.as_array().unwrap()) {
                assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 1e-12);
            }
        }
    }
    for (a, b) in actual["table_bootstrap_95_percent_interval"]
        .as_array()
        .unwrap()
        .iter()
        .zip(
            expected["table_bootstrap_95_percent_interval"]
                .as_array()
                .unwrap(),
        )
    {
        assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 0.015);
    }
}
