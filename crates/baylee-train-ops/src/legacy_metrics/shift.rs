use super::{floats, ratio};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::fmt::Write;
fn zeros(n: usize) -> Vec<f64> {
    vec![0.0; n]
}
fn average(sum: f64, n: u64) -> Result<Value> {
    if n == 0 {
        Ok(Value::Null)
    } else {
        Ok(json!(sum / f64::from(u32::try_from(n)?)))
    }
}
pub(super) fn report(input: &Value) -> Result<Value> {
    let names: Vec<String> = serde_json::from_value(input["names"].clone())?;
    let k = names.len();
    ensure!(k > 0, "at least one policy required");
    let mut n = 0u64;
    let (mut probability, mut entropy, mut hits) = (zeros(k), zeros(k), zeros(k));
    let (mut agree, mut kl) = (vec![zeros(k); k], vec![zeros(k); k]);
    let (mut advantage_sum, mut advantage_n) = ([0.0; 2], [0usize; 2]);
    for batch in input["batches"]
        .as_array()
        .context("shift tensor batches")?
    {
        n = n
            .checked_add(batch["n"].as_u64().context("shift decision count")?)
            .context("shift sample overflow")?;
        for (key, target) in [
            ("top_p", &mut probability),
            ("entropy", &mut entropy),
            ("hit", &mut hits),
        ] {
            let source = floats(&batch[key])?;
            ensure!(source.len() == k, "policy summary width differs");
            for (a, b) in target.iter_mut().zip(source) {
                *a += b;
            }
        }
        for (key, target) in [("agree", &mut agree), ("kl", &mut kl)] {
            let rows = batch[key].as_array().context("policy pair matrix")?;
            ensure!(rows.len() == k, "pair matrix height");
            for (out, row) in target.iter_mut().zip(rows) {
                let row = floats(row)?;
                ensure!(row.len() == k, "pair matrix width");
                for (a, b) in out.iter_mut().zip(row) {
                    *a += b;
                }
            }
        }
        if let Some(values) = batch["advantage"].as_array() {
            let finite = batch["advantage_finite"]
                .as_array()
                .context("advantage finite flags")?;
            let on_top = batch["advantage_on_top"]
                .as_array()
                .context("advantage top flags")?;
            ensure!(
                values.len() == finite.len() && values.len() == on_top.len(),
                "advantage tensor arrays differ"
            );
            for ((value, valid), top) in values.iter().zip(finite).zip(on_top) {
                if valid.as_bool().context("finite flag")? {
                    let index = usize::from(top.as_bool().context("top flag")?);
                    advantage_sum[index] += value
                        .as_f64()
                        .filter(|v| v.is_finite())
                        .context("finite advantage")?;
                    advantage_n[index] += 1;
                }
            }
        }
    }
    let mut rows = Vec::new();
    let mut text = format!("{n} held-out learner decisions with a choice\n");
    for (i, name) in names.iter().enumerate() {
        let row = json!({"model":name,"sampled_top_hit":average(hits[i],n)?,"top_probability":average(probability[i],n)?,"entropy":average(entropy[i],n)?});
        writeln!(
            text,
            "{name}: sampled-top {}; mean top probability {}; entropy {}",
            row["sampled_top_hit"], row["top_probability"], row["entropy"]
        )?;
        rows.push(row);
    }
    let mut pairs = Vec::new();
    for i in 0..k {
        for j in i + 1..k {
            let row = json!({"from":names[i],"to":names[j],"top_agrees":average(agree[i][j],n)?,"kl":average(kl[i][j],n)?});
            writeln!(
                text,
                "{} -> {}: top agrees {}; KL {}",
                names[i], names[j], row["top_agrees"], row["kl"]
            )?;
            pairs.push(row);
        }
    }
    let advantage = json!({"on_top":{"n":advantage_n[1],"mean":advantage_sum[1]/ratio(advantage_n[1].max(1))},"other":{"n":advantage_n[0],"mean":advantage_sum[0]/ratio(advantage_n[0].max(1))}});
    if let Some(player) = input["player"].as_u64() {
        let name = names
            .get(usize::try_from(player)?)
            .context("player model index")?;
        for (key, label) in [("on_top", "its top option"), ("other", "another option")] {
            writeln!(
                text,
                "sampled {label} of {name}: {} decisions, mean advantage {}",
                advantage[key]["n"], advantage[key]["mean"]
            )?;
        }
    }
    Ok(
        json!({"n":n,"models":rows,"pairs":pairs,"advantage":advantage,"text":text,"aggregation":"native f64"}),
    )
}
