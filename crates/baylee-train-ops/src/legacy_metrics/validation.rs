//! Bounded calibration state for reference recurrent validation.
use super::{benchmark::numpy_sum, floats};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
pub(super) fn calibration(input: &Value) -> Result<Value> {
    let mut state = if input["state"].is_null() {
        vec![0.0; 22]
    } else {
        floats(&input["state"])?
    };
    ensure!(state.len() == 22, "invalid calibration state");
    let predictions = floats(&input["prediction"])?;
    let targets = floats(&input["truth"])?;
    ensure!(
        predictions.len() == targets.len(),
        "calibration vector lengths differ"
    );
    for (&p, &y) in predictions.iter().zip(&targets) {
        ensure!(
            (0.0..=1.0).contains(&p) && (0.0..=1.0).contains(&y),
            "calibration probabilities outside [0,1]"
        );
        // Reference output is FP32: retain FP32 multiply before its integer bin cast.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Validated FP32 probabilities times ten are in [0,10], clamped to bin9 like the reference NumPy path."
        )]
        let bin = (((p as f32) * 10.0) as usize).min(9);
        state[bin] += p;
        state[10 + bin] += y;
    }
    let mut totals = if input["totals"].is_null() {
        vec![0.0; 9]
    } else {
        floats(&input["totals"])?
    };
    ensure!(totals.len() == 9, "invalid validation accumulation state");
    let mut offset = 0;
    for row in input["steps"].as_array().context("validation steps")? {
        let values = floats(row)?;
        ensure!(values.len() == 9, "invalid validation step statistics");
        ensure!(
            values[3] >= 0.0
                && values[3].fract() == 0.0
                && values[3] <= super::ratio(predictions.len() - offset),
            "invalid calibration step count"
        );
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Validated integer frame count is bounded by the remaining prediction vector."
        )]
        let end = offset + values[3] as usize;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "Reference predictions/targets originate as FP32 tensors; preserve the original NumPy FP32 per-step reduction."
        )]
        let p: Vec<f32> = predictions[offset..end].iter().map(|&v| v as f32).collect();
        #[allow(
            clippy::cast_possible_truncation,
            reason = "Reference targets originate as FP32 tensors."
        )]
        let y: Vec<f32> = targets[offset..end].iter().map(|&v| v as f32).collect();
        state[20] += f64::from(numpy_sum(&p));
        state[21] += f64::from(numpy_sum(&y));
        offset = end;
        for (i, value) in values.iter().enumerate() {
            totals[i] += if i == 1 || i == 2 {
                value * values[3]
            } else {
                *value
            };
        }
    }
    ensure!(
        offset == predictions.len(),
        "validation step counts do not cover calibration rows"
    );
    Ok(json!({"calibration":state,"totals":totals}))
}
pub(super) fn summary(input: &Value) -> Result<Value> {
    let state = floats(&input["calibration"])?;
    let totals = floats(&input["totals"])?;
    let choice = floats(&input["choice"])?;
    ensure!(
        state.len() == 22 && totals.len() == 9 && choice.len() == 3,
        "invalid reference validation totals"
    );
    let [
        pol,
        val,
        aux,
        frames,
        correct,
        singles,
        baseline,
        active,
        active_correct,
    ] = totals[..]
    else {
        unreachable!()
    };
    let denominator = frames.max(1.0);
    let batches = input["batches"].as_f64().context("validation batches")?;
    let unroll = input["unroll"].as_f64().context("validation unroll")?;
    Ok(
        json!({"val_choice_nll":choice[0]/choice[2].max(1.0),"val_choice_top1_acc":choice[1]/choice[2].max(1.0),"val_choices":input["counts"]["choices"],"val_pol_loss":pol/(batches*unroll).max(1.0),"val_mse":val/denominator,"val_aux_mse":aux/denominator,"val_top1_acc":correct/singles.max(1.0),"val_active_top1_acc":active_correct/active.max(1.0),"val_active_choices":input["counts"]["active"],"val_uniform_seat_mse":baseline/denominator,"val_ece":(0..10).map(|i|(state[i]-state[10+i]).abs()).sum::<f64>()/denominator,"val_prediction_mean":state[20]/denominator,"val_outcome_mean":state[21]/denominator,"val_frames":input["counts"]["frames"]}),
    )
}
