use super::*;
#[test]
fn proper_score_and_draw_accuracy_are_explicit() -> Result<()> {
    let result = metrics(&[0.25, 0.75], &[0.0, 1.0])?;
    ensure!(result["brier"] == 0.0625 && result["accuracy"] == 1.0 && result["ece"] == 0.25);
    ensure!(metrics(&[0.5], &[0.5])?["accuracy"].is_null());
    ensure!(metrics(&[], &[]).is_err());
    Ok(())
}
#[test]
fn value_fit_and_eval_games_are_disjoint_with_unknown_key_fallback() -> Result<()> {
    let result = run(
        &json!({"kind":"value-plan","no_deck":true,"games":[],"rows":{"game":[0,1,2,3,3],"seat":[0,0,0,1,0],"turn":[1,1,1,1,1],"result":[2,0,0,2,0],"step":[0,0,0,0,1]}}),
    )?;
    ensure!(
        result["indices"] == json!([1, 3])
            && result["baseline"] == json!([0.5, 0.5])
            && result["truth"] == json!([0.0, 1.0])
    );
    Ok(())
}
#[test]
fn shift_merges_batches_and_ignores_nonfinite_advantage_slots() -> Result<()> {
    let result = run(
        &json!({"kind":"shift-report","names":["a","b"],"player":0,"batches":[{"n":2,"top_p":[1.8,1.4],"entropy":[0.4,0.8],"hit":[2,1],"agree":[[2,1],[1,2]],"kl":[[0,0.2],[0.3,0]],"advantage":[1.0,0.0],"advantage_finite":[true,false],"advantage_on_top":[true,false]},{"n":1,"top_p":[0.6,0.5],"entropy":[0.2,0.4],"hit":[0,1],"agree":[[1,0],[0,1]],"kl":[[0,0.1],[0.2,0]]}]}),
    )?;
    ensure!(
        result["n"] == 3
            && result["advantage"]["on_top"]["n"] == 1
            && result["advantage"]["on_top"]["mean"] == 1.0
    );
    ensure!(
        (result["models"][0]["top_probability"]
            .as_f64()
            .context("probability")?
            - 0.8)
            .abs()
            < 1e-12
    );
    Ok(())
}
#[test]
fn policy_reports_preserve_profile_thresholds_masking_and_uniform_baselines() -> Result<()> {
    let mut request = json!({"kind":"policy-report","v3":false,"hits":vec![true;101],"usable":vec![true;101],"question":vec![0;101],"profile":vec![0;101],"options":vec![2;101],"kinds":["choice"],"profiles":["house","rare"],"report":{"model":"fixture","params":1,"best_step":1000,"curve":[],"unmatched_answers":null}});
    request["profile"][100] = json!(1);
    let policy = run(&request)?;
    ensure!(policy["report"]["by_profile"]["rare"]["n"] == 1);
    ensure!(policy["report"]["agreement"] == 1.0 && policy["report"]["uniform_chance"] == 0.5);
    ensure!(
        policy["markdown"]
            .as_str()
            .context("markdown")?
            .contains("step 1,000.")
    );
    request["v3"] = json!(true);
    request["usable"][100] = json!(false);
    request["options"][0] = json!(0);
    request["report"]["value"] = json!({});
    let v3 = run(&request)?;
    ensure!(v3["report"]["held_out_steps"] == 100 && v3["report"]["by_profile"]["rare"].is_null());
    ensure!((v3["report"]["uniform_chance"].as_f64().context("chance")? - 0.505).abs() < 1e-12);
    ensure!(v3["report"]["params"] == 1 && v3["report"]["unmatched_answers"].is_null());
    Ok(())
}
#[test]
fn v3_value_slices_preserve_team_draw_and_first_step_rules() -> Result<()> {
    let request = json!({"kind":"v3-value-slices","prediction":[0.25,0.75,0.5,0.99],"step":[0,0,0,1],"result":[0,0,0,0],"team":[1,2,1,1],"winners":[1,2,0,1],"seats":[2,3,4,2],"turn":[1,1,1,1]});
    let result = run(&request)?;
    ensure!(result["all"]["n"] == 3 && result.as_object().context("slices")?.len() == 1);
    ensure!((result["all"]["brier"].as_f64().context("brier")? - 1.0 / 24.0).abs() < 1e-12);
    ensure!(result["all"]["accuracy"] == 1.0);
    Ok(())
}
