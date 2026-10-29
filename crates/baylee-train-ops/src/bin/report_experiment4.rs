//! Inspect offline experiments and refresh derived uncertainty reports.
use anyhow::{Result, ensure};
use baylee_train_ops::evaluation::{atomic_json, progress::inspect};
use clap::Parser;
use serde_json::json;
use std::{
    fmt::Write,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Parser)]
struct Args {
    #[arg(long, required = true)]
    experiment: Vec<PathBuf>,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 0.0)]
    watch_hours: f64,
}
fn main() -> Result<()> {
    let a = Args::parse();
    ensure!(
        a.watch_hours.is_finite() && a.watch_hours >= 0.0,
        "watch hours must be finite and nonnegative"
    );
    std::fs::create_dir_all(&a.out)?;
    let start = Instant::now();
    loop {
        let results = a
            .experiment
            .iter()
            .map(|p| inspect(&p.canonicalize()?))
            .collect::<Result<Vec<_>>>()?;
        let finished = results.iter().all(|v| v["finished"] == true);
        let expired = start.elapsed().as_secs_f64() >= a.watch_hours * 3600.0;
        let status = if finished {
            "complete"
        } else if a.watch_hours <= 0.0 {
            "one_shot"
        } else if expired {
            "deadline"
        } else {
            "watching"
        };
        atomic_json(
            &a.out.join("progress.json"),
            &json!({"updated_unix":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64(),"status":status,"experiments":results}),
        )?;
        let mut text = format!(
            "# Playernet: lokaler Laufstatus\n\nBericht: {status}.\n\nAutomatisch aus den Experimentdateien erstellt. Keine Aussage über menschliche Spielstärke.\n"
        );
        for r in &results {
            write!(text, "\n## {}\n\n", r["root"].as_str().unwrap_or_default())?;
            if let Some(line) = r["latest_step"].as_str() {
                text.push_str(line);
                text.push_str("\n\n");
            }
            let validation = &r["validation"];
            if let (Some(active), Some(brier), Some(uniform), Some(ece)) = (
                validation["val_active_top1_acc"].as_f64(),
                validation["val_mse"].as_f64(),
                validation["val_uniform_seat_mse"].as_f64(),
                validation["val_ece"].as_f64(),
            ) {
                writeln!(
                    text,
                    "Aktive Auswahl: {:.1}%; Brier: {brier:.4}; einfache Basis: {uniform:.4}; ECE: {ece:.4}.\n",
                    active * 100.0
                )?;
            }
            if let Some(jobs) = r["jobs"].as_array() {
                for j in jobs {
                    writeln!(
                        text,
                        "- {}: {}{}",
                        j["stage"].as_str().unwrap_or_default(),
                        j["status"].as_str().unwrap_or_default(),
                        j["reason"]
                            .as_str()
                            .map_or(String::new(), |s| format!(" ({s})"))
                    )?;
                }
            }
            if let Some(evals) = r["evaluations"].as_object() {
                for (name, ev) in evals {
                    writeln!(
                        text,
                        "\n{name}: {} unabhängige Tische; gepaarte Differenz {}; konservatives 95%-Intervall {}; {} unvollständige Paare.",
                        ev["independent_tables"],
                        ev["paired_utility_difference"],
                        ev["bounded_independent_table_95_percent_interval"],
                        ev["capped_pairs"]
                    )?;
                }
            }
        }
        let tmp = a.out.join("progress.md.partial");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, a.out.join("progress.md"))?;
        if a.watch_hours <= 0.0 || expired || finished {
            break;
        }
        std::thread::sleep(Duration::from_secs(30));
    }
    Ok(())
}
