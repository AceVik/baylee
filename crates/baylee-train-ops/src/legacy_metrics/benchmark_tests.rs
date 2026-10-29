use super::*;
#[test]
fn quantile_methods_remain_distinct_and_empty_timings_refuse() -> Result<()> {
    let inference =
        run(&json!({"kind":"benchmark-statistics","mode":"inference","samples":[1,2,3,4]}))?;
    let step =
        run(&json!({"kind":"benchmark-statistics","mode":"step","samples":[1,2,3,4],"batch":5}))?;
    ensure!(inference["p50_ms"] == 2.5 && step["median_ms"] == 2500.0 && step["p95_ms"] == 4000.0);
    ensure!((inference["p95_ms"].as_f64().context("quantile")? - 3.85).abs() < 1e-12);
    ensure!(step["decisions_per_s"] == 2);
    ensure!(run(&json!({"kind":"benchmark-statistics","mode":"t1","samples":[]})).is_err());
    Ok(())
}
#[test]
fn calibration_adds_only_admitted_rows_and_retains_fixed_state() -> Result<()> {
    let state = run(
        &json!({"kind":"reference-calibration","prediction":[0.25,0.75],"truth":[0,1],"steps":[[2,0.5,0.25,2,1,2,0.5,1,1]]}),
    )?;
    ensure!(
        state["calibration"]
            .as_array()
            .context("calibration")?
            .len()
            == 22
    );
    let report = run(
        &json!({"kind":"reference-validation","calibration":state["calibration"],"totals":state["totals"],"choice":[1,1,2],"batches":1,"unroll":2,"counts":{"frames":2,"active":1,"choices":2}}),
    )?;
    ensure!(
        report["val_ece"] == 0.25
            && report["val_prediction_mean"] == 0.5
            && report["val_mse"] == 0.5
    );
    ensure!(report["val_frames"].as_u64() == Some(2));
    let next = run(
        &json!({"kind":"reference-calibration","state":state["calibration"],"totals":state["totals"],"prediction":[],"truth":[],"steps":[]}),
    )?;
    ensure!(next == state);
    Ok(())
}
