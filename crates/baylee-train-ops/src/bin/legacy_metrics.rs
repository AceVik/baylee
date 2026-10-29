//! Native reduction boundary for retained Torch baseline evaluation.
use anyhow::Result;
use std::io::Read;
fn main() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    println!(
        "{}",
        baylee_train_ops::legacy_metrics::run(&serde_json::from_str(&input)?)?
    );
    Ok(())
}
