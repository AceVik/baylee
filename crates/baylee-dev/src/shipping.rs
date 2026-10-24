//! Keeps verification hooks out of every shipped feature graph.
use crate::run::{self, Steps, cargo_with};
use std::time::Instant;

/// Neither verification hook may reach a build that ships. `mutate` is
/// enabled by `baylee-engine`'s dev-dependencies alone and `fuzz` by nothing,
/// so the feature graph without dev edges names neither. The engine's graph
/// with dev edges must name `mutate`: the control that proves the pattern
/// matches what `cargo tree` prints, without which a renamed feature would
/// count zero leaks forever. `--target all`, so a dependency declared for one
/// platform cannot switch a hook on where this host never looks.
pub fn verification_hooks_unshipped(steps: &mut Steps) {
    let started = Instant::now();
    let shipped = run::captured(&mut cargo_with([
        "tree",
        "--workspace",
        "--target",
        "all",
        "-e",
        "features,normal,build",
    ]));
    let engine = run::captured(&mut cargo_with([
        "tree",
        "-p",
        "baylee-engine",
        "--target",
        "all",
        "-e",
        "features",
    ]));
    let secs = started.elapsed().as_secs();
    match (shipped, engine) {
        (Some(shipped), Some(engine)) if shipped.status.success() && engine.status.success() => {
            let leaks = String::from_utf8_lossy(&shipped.stdout)
                .lines()
                .filter(|l| l.contains("feature \"mutate\"") || l.contains("feature \"fuzz\""))
                .count();
            let control = String::from_utf8_lossy(&engine.stdout)
                .lines()
                .filter(|l| l.contains("baylee-cards feature \"mutate\""))
                .count();
            if leaks == 0 && control > 0 {
                println!(
                    "STEP verification-hooks-shipped ok (0 in the shipped graph, {control} in the engine's dev graph, {secs}s)"
                );
                steps.record(true);
            } else {
                println!(
                    "STEP verification-hooks-shipped FAILED - {leaks} mentions of mutate/fuzz in the graph"
                );
                println!(
                    "  without dev edges (must be 0), {control} of mutate in baylee-engine's dev graph (must be >0)"
                );
                steps.record(false);
            }
        }
        (shipped, engine) => {
            println!("STEP verification-hooks-shipped FAILED - cargo tree did not run");
            for out in [shipped, engine]
                .into_iter()
                .flatten()
                .filter(|o| !o.status.success())
            {
                println!("{}", run::tail(&run::text(&out), 40));
            }
            steps.record(false);
        }
    }
}
