//! Native statistics for retained value-v2 and policy-v3 tensor baselines.
mod benchmark;
mod benchmark_resources;
#[cfg(test)]
mod benchmark_tests;
mod policy;
mod shift;
#[cfg(test)]
mod tests;
mod training;
mod validation;
mod value;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
fn floats(value: &Value) -> Result<Vec<f64>> {
    value
        .as_array()
        .context("metric vector")?
        .iter()
        .map(|v| {
            v.as_f64()
                .filter(|n| n.is_finite())
                .context("nonfinite legacy metric input")
        })
        .collect()
}
#[allow(
    clippy::cast_precision_loss,
    reason = "Report means use approximate floats; sample counts remain exact integers."
)]
fn ratio(n: usize) -> f64 {
    n as f64
}
#[allow(
    clippy::float_cmp,
    reason = "The legacy draw label is exactly encoded as 0.5, not an approximate threshold."
)]
fn metrics(p: &[f64], y: &[f64]) -> Result<Value> {
    ensure!(
        p.len() == y.len() && !p.is_empty(),
        "nonempty equal-length value arrays required"
    );
    ensure!(
        p.iter()
            .chain(y)
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
        "value predictions/outcomes outside [0,1]"
    );
    let samples = p.len();
    let mut bins = [(0usize, 0.0, 0.0); 10];
    let (mut brier, mut logloss, mut hits, mut decided) = (0.0, 0.0, 0usize, 0usize);
    for (&p, &y) in p.iter().zip(y) {
        let p = p.clamp(1e-6, 1.0 - 1e-6);
        brier += (p - y).powi(2);
        logloss -= y * p.ln() + (1.0 - y) * (1.0 - p).ln();
        if y != 0.5 {
            decided += 1;
            hits += usize::from((p > 0.5) == (y > 0.5));
        }
        let index = bin(p);
        let bin = &mut bins[index];
        bin.0 += 1;
        bin.1 += p;
        bin.2 += y;
    }
    let mut reliability = Vec::new();
    let mut ece = 0.0;
    for (index, (n, p, y)) in bins.into_iter().enumerate() {
        if n > 0 {
            let mp = p / ratio(n);
            let my = y / ratio(n);
            ece += ratio(n) / ratio(samples) * (mp - my).abs();
            reliability.push(json!({"bin":format!("{:.1}-{:.1}",ratio(index)/10.0,ratio(index+1)/10.0),"n":n,"predicted":mp,"won":my}));
        }
    }
    Ok(
        json!({"n":p.len(),"brier":brier/ratio(p.len()),"logloss":logloss/ratio(p.len()),"accuracy":if decided>0{json!(ratio(hits)/ratio(decided))}else{Value::Null},"accuracy_text":if decided>0{format!("{:.3}",ratio(hits)/ratio(decided))}else{"n/a".into()},"ece":ece,"reliability":reliability}),
    )
}
/// Aggregate tensor-produced baseline arrays without loading or changing a model.
/// # Errors
/// Refuses invalid vectors, unknown request kinds or inconsistent sample identities.
pub fn run(input: &Value) -> Result<Value> {
    match input["kind"].as_str().context("legacy metric kind")? {
        "benchmark-resources" => benchmark_resources::run(input),
        "benchmark-statistics" => benchmark::statistics(input),
        "benchmark-plan" => benchmark::plan(input),
        "reference-calibration" => validation::calibration(input),
        "reference-validation" => validation::summary(input),
        "metrics" => metrics(&floats(&input["prediction"])?, &floats(&input["truth"])?),
        "policy-report" => policy::publish(input),
        "v3-value-slices" => policy::value_slices(input),
        "group-report" => training::grouped(input),
        "training-report" | "training-markdown" => training::publish(input),
        "value-plan" => value::plan(input),
        "value-report" => value::report(input),
        "shift-report" => shift::report(input),
        other => anyhow::bail!("unknown legacy metric kind {other}"),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Validated/clipped probabilities times ten are bounded in [0,10); truncation matches legacy NumPy bins."
)]
fn bin(p: f64) -> usize {
    ((p * 10.0) as usize).min(9)
}
