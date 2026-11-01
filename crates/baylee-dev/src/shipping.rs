//! Release-relevant dependency checks, without building development features.
use crate::run::{self, Steps, cargo_with};
use std::time::Instant;

mod bigint;

/// The release-relevant feature check, without building development features.
pub fn run() -> i32 {
    let mut steps = Steps::default();
    check(&mut steps);
    steps.finish()
}

/// Checks shipped hooks and the exact scope of the reviewed advisory exception.
pub fn check(steps: &mut Steps) {
    verification_hooks_unshipped(steps);
    verification_hooks_not_unified(steps);
    bigint::check(steps);
}

/// Lines of a feature graph that name a verification hook.
fn hook_mentions(graph: &[u8]) -> usize {
    String::from_utf8_lossy(graph)
        .lines()
        .filter(|l| {
            l.contains("feature \"mutate\"")
                || l.contains("feature \"fuzz\"")
                || l.contains("baylee-engine feature \"simulation\"")
        })
        .count()
}

/// No crate's *default* features may turn a hook on, the training crates'
/// included. `cargo build-servers` and `cargo build-client` select
/// `--workspace`, so every member's defaults unify into the shipped binaries,
/// even a member that never ships: the graph of
/// [`verification_hooks_unshipped`] leaves the training crates out and would
/// not see one of their defaults enable `simulation`. The whole workspace's
/// graph, inverted on the crates that own a hook, must name none. The control:
/// the trainer's own build (`baylee-train/onnx`) turns `simulation` on, and the
/// same inverted graph must say so.
pub fn verification_hooks_not_unified(steps: &mut Steps) {
    let started = Instant::now();
    let inverted = |extra: &[&str]| {
        let mut args = vec!["tree", "--target", "all", "-e", "features,normal,build"];
        args.extend_from_slice(extra);
        run::captured(&mut cargo_with(args))
    };
    let unified = inverted(&["--workspace", "-i", "baylee-engine", "-i", "baylee-cards"]);
    let trainer = inverted(&[
        "-p",
        "baylee-train",
        "--features",
        "onnx",
        "-i",
        "baylee-engine",
    ]);
    let secs = started.elapsed().as_secs();
    match (unified, trainer) {
        (Some(unified), Some(trainer)) if unified.status.success() && trainer.status.success() => {
            let leaks = hook_mentions(&unified.stdout);
            let control = hook_mentions(&trainer.stdout);
            if leaks == 0 && control > 0 {
                println!(
                    "STEP verification-hooks-unified ok (0 in the --workspace graph, {control} in the trainer's, {secs}s)"
                );
                steps.record(true);
            } else {
                println!(
                    "STEP verification-hooks-unified FAILED - {leaks} hooks in the --workspace graph (must be 0), {control} in baylee-train/onnx's (must be >0)"
                );
                steps.record(false);
            }
        }
        (unified, trainer) => {
            println!("STEP verification-hooks-unified FAILED - cargo tree did not run");
            for out in [unified, trainer]
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

/// Neither verification hook may reach a build that ships. `mutate` is
/// enabled by `baylee-engine`'s dev-dependencies alone and `fuzz` by nothing,
/// so the feature graph without dev edges names neither. The same holds for
/// `baylee-engine`'s `simulation` (`Engine::fork_for`, a copy of a game with
/// its hidden cards dealt again): only the training crates, which are not in
/// the graph, may turn it on, so a hosting build can never fork a live game.
/// The engine's graph with dev edges must name `mutate`: the control that
/// proves the pattern matches what `cargo tree` prints, without which a
/// renamed feature would count zero leaks forever. `--target all`, so a dependency declared for one
/// platform cannot switch a hook on where this host never looks.
pub fn verification_hooks_unshipped(steps: &mut Steps) {
    let started = Instant::now();
    // The training crates never ship (and are not CI's, owner 30.10.2026).
    let shipped = run::captured(&mut cargo_with([
        "tree",
        "--workspace",
        "--exclude",
        "baylee-train",
        "--exclude",
        "baylee-train-ops",
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
            let leaks = hook_mentions(&shipped.stdout);
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
                    "STEP verification-hooks-shipped FAILED - {leaks} mentions of mutate/fuzz/simulation in the graph"
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
