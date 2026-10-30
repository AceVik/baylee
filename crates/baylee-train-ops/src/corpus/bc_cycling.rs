//! Complete-pass budget planning around the tensor sampler callback.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
const STRIDE: u64 = 1_000_003;
#[derive(Serialize, Deserialize)]
struct Pass {
    cycle: u64,
    sampler_epoch: u64,
    total_batches: usize,
    consumed_batches: usize,
    complete: bool,
    padding_lane_batches: usize,
}
#[derive(Serialize, Deserialize)]
struct State {
    epoch: u64,
    half: usize,
    budget: usize,
    result: Vec<Vec<i64>>,
    passes: Vec<Pass>,
}
impl State {
    fn next_epoch(&self) -> Result<u64> {
        self.epoch
            .checked_add(
                u64::try_from(self.passes.len())?
                    .checked_mul(STRIDE)
                    .context("BC cycle epoch overflow")?,
            )
            .context("BC cycle epoch overflow")
    }
    fn advance(&mut self, rows: Vec<Vec<i64>>) -> Result<()> {
        ensure!(!rows.is_empty(), "BC cycling source has no batches");
        ensure!(
            !self.passes.is_empty() || self.budget >= rows.len(),
            "BC epoch batches cannot truncate a source first complete pass"
        );
        ensure!(
            self.result.len() < self.budget,
            "BC cycle is already complete"
        );
        let take = rows.len().min(self.budget - self.result.len());
        let epoch = self.next_epoch()?;
        let mut padding = 0;
        let total = rows.len();
        for mut row in rows.into_iter().take(take) {
            ensure!(
                row.len() <= self.half,
                "BC sampler batch exceeds its declared half lanes"
            );
            row.resize(self.half, -1);
            padding += row.iter().filter(|i| **i < 0).count();
            self.result.push(row);
        }
        self.passes.push(Pass {
            cycle: u64::try_from(self.passes.len())?,
            sampler_epoch: epoch,
            total_batches: total,
            consumed_batches: take,
            complete: take == total,
            padding_lane_batches: padding,
        });
        Ok(())
    }
    fn receipt(&self) -> Result<Value> {
        let first = self
            .passes
            .first()
            .context("BC cycle has no completed pass")?;
        let last = self
            .passes
            .last()
            .context("BC cycle has no completed pass")?;
        Ok(
            json!({"first_pass_batches":first.total_batches,"complete_passes":self.passes.iter().filter(|p|p.complete).count(),"repeated_passes_started":self.passes.len()-1,"partial_repeat_batches":if last.complete{0}else{last.consumed_batches},"padding_lane_batches":self.passes.iter().map(|p|p.padding_lane_batches).sum::<usize>(),"passes":self.passes}),
        )
    }
}
/// Plan one whole source pass; actual tensor-sampler iteration stays in its worker.
///
/// # Errors
/// Rejects empty/truncated first passes, impossible lane sizes and epoch overflow.
pub fn run(value: &Value) -> Result<Value> {
    let mut state: State = if value["state"].is_null() {
        State {
            epoch: value["epoch"].as_u64().context("invalid BC epoch")?,
            half: usize::try_from(value["half"].as_u64().context("invalid BC lane count")?)?,
            budget: usize::try_from(value["budget"].as_u64().context("invalid BC budget")?)?,
            result: Vec::new(),
            passes: Vec::new(),
        }
    } else {
        serde_json::from_value(value["state"].clone())?
    };
    ensure!(
        state.half > 0 && state.budget > 0,
        "BC lanes and epoch budget must be positive"
    );
    if let Some(rows) = value.get("rows") {
        state.advance(serde_json::from_value(rows.clone())?)?;
    }
    let done = state.result.len() == state.budget;
    let receipt = if done { state.receipt()? } else { Value::Null };
    Ok(json!({"next_epoch":state.next_epoch()?,"done":done,"receipt":receipt,"state":state}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_source_passes_preserve_padding_and_explicit_repeat_prefix() -> Result<()> {
        let first = run(&json!({"epoch":7,"half":2,"budget":3,"rows":[[1],[2,3]]}))?;
        assert_eq!(first["next_epoch"], 1_000_010);
        assert_eq!(first["done"], false);
        let final_pass = run(&json!({"state":first["state"],"rows":[[4],[5,6]]}))?;
        assert_eq!(final_pass["done"], true);
        assert_eq!(
            final_pass["state"]["result"],
            json!([[1, -1], [2, 3], [4, -1]])
        );
        assert_eq!(final_pass["receipt"]["partial_repeat_batches"], 1);
        assert_eq!(final_pass["receipt"]["padding_lane_batches"], 2);
        assert!(run(&json!({"epoch":0,"half":2,"budget":1,"rows":[[1],[2]]})).is_err());
        Ok(())
    }
}
