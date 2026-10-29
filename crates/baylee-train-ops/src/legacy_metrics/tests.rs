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
