//! The development-feature matrix for current desktop and server support.
//! This matrix runs locally before landing; CI checks only shipped graphs.
//!
//! `clippy --workspace` and `test --workspace` compile each crate with its
//! default features, so the non-default ones were compiled by nobody:
//!
//! | crate | features |
//! | --- | --- |
//! | `baylee-client` | `dev-control`, `dev-reload`, `dev-dylink` |
//! | `baylee-gateway` | `dev-table` |
//! | `baylee-client-core` | `test-support` |
//! | `baylee-engine` | `fuzz` |
//! | `baylee-cards` | `mutate` |
//! | `baylee-train` | `llm-collector` |
//!
//! plus one default switched off, `baylee-train` without `play`, which is how
//! `xtask` links it (`onnx` is trainer-only and in no gate: it downloads ONNX
//! Runtime when it builds; so do `llm` and `live-seat`, which pull it in, and
//! they stay out for the same reason).
//!
//! It fails in both directions. A feature adds code the default build never
//! sees (`devctl.rs` broke on `Option<Refusal>` with the whole gate green,
//! #121), and a feature makes other code live that the default build reads
//! as dead, so `-D warnings` fires on an unused import only with it on.
//! `--all-features` is not the shortcut: `dev-dylink` changes how the binary
//! links and `dev-reload` how assets load, so all-on measures a configuration
//! nobody runs. Each feature gets the shape of check it earns.
use crate::run::{self, Steps, cargo_with};
use std::{process::Command, time::Instant};

/// The count that lets the client step refuse the failure this file exists
/// for: a test run passes just as happily when the feature stopped applying
/// and its tests stopped existing. It counts `devctl::` tests, not the suite,
/// so it is anchored to the feature by construction: measured 20.09.2026 as
/// 17 with the feature and 0 without, and zero is the exact signature of the
/// failure. Raise it when `devctl.rs` grows; never lower it to what came out.
const DEVCTL_TEST_FLOOR: usize = 15;

/// `dev-control` and `dev-reload` in one build: the configuration docs/dev/commands.md
/// documents for editing a shader in a running client, so a build somebody
/// makes. `dev-reload` forwards a bevy feature, so every distinct client
/// feature set is its own bevy compile; the union covers both `cfg` sites.
const CLIENT_FEATURES: &str = "dev-control,dev-reload";

/// The checks that only have to pass, in the order they run. Each one's
/// shape is the check its feature earns:
///
/// Android and iOS are coming soon and have no feature-gate checks yet.
///
/// - `dev-dylink` gates no source at all (`["bevy/dynamic_linking"]`), so the
///   only thing it can break is the feature graph, which `cargo tree` answers
///   in half a second; forwarding to a `dynamic_linking_NOPE` exits 101. A
///   resolver error here once came from a year-stale local index cache
///   (`crates/baylee-client/Cargo.toml` tells it); suspect that first.
/// - `test-support`, `fuzz` and `mutate` are `cfg(any(test, feature = …))`,
///   so `--all-targets` compiles them through `test` anyway; `--lib` checks
///   the shape every consumer builds: the feature on, `cfg(test)` off.
/// - `baylee-train` without `play` is how every `cargo run -p xtask` links it,
///   which the workspace build never does.
/// - `llm-collector` builds the language-model replay collector, a binary
///   no default feature reaches; it needs no ONNX Runtime, so it is checked
///   whole, tests included, in [`LINUX_CHECKS`].
const CHECKS: &[(&str, &[&str])] = &[
    (
        "dev-dylink-resolves",
        &[
            "tree",
            "-p",
            "baylee-client",
            "--features",
            "dev-dylink",
            "-e",
            "features",
        ],
    ),
    (
        "test-support",
        &[
            "clippy",
            "-p",
            "baylee-client-core",
            "--features",
            "test-support",
            "--lib",
            "--",
            "-D",
            "warnings",
        ],
    ),
    (
        "train-without-play",
        &[
            "clippy",
            "-p",
            "baylee-train",
            "--no-default-features",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    ),
    (
        "fuzz",
        &[
            "clippy",
            "-p",
            "baylee-engine",
            "--features",
            "fuzz",
            "--lib",
            "--",
            "-D",
            "warnings",
        ],
    ),
    (
        "mutate",
        &[
            "clippy",
            "-p",
            "baylee-cards",
            "--features",
            "mutate",
            "--lib",
            "--",
            "-D",
            "warnings",
        ],
    ),
];

/// The LLM collector runs its seats as process groups it reads through
/// `/proc` (`baylee_train_ops::process`), so it builds on Linux only.
const LINUX_CHECKS: &[(&str, &[&str])] = &[(
    "train-llm-collector",
    &[
        "clippy",
        "-p",
        "baylee-train",
        "--features",
        "llm-collector",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ],
)];

/// The gateway's seated dev table: `main.rs` carries both
/// `cfg(feature = "dev-table")` and its negation, so the default gate compiles
/// the refusal and never the thing refused. Needs PostgreSQL.
const DEV_TABLE: &[(&str, &[&str])] = &[
    (
        "dev-table-clippy",
        &[
            "clippy",
            "-p",
            "baylee-gateway",
            "--features",
            "dev-table",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    ),
    (
        "dev-table-test",
        &[
            "test",
            "-p",
            "baylee-gateway",
            "--features",
            "dev-table",
            "--all-targets",
        ],
    ),
];

pub fn run() -> i32 {
    let mut steps = Steps::default();
    quiet_step(
        &mut steps,
        "client-features-clippy",
        &mut cargo_with([
            "clippy",
            "-p",
            "baylee-client",
            "--features",
            CLIENT_FEATURES,
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
    );
    client_tests(&mut steps);
    if std::env::var_os("DATABASE_URL").is_some_and(|v| !v.is_empty()) {
        for (label, args) in DEV_TABLE {
            quiet_step(&mut steps, label, &mut cargo_with(*args));
        }
    } else {
        println!("STEP dev-table SKIPPED - no DATABASE_URL; the gateway suite needs PostgreSQL");
        println!(
            "  (a gateway without one refuses to start: this did not run, it did not degrade)"
        );
    }
    for (label, args) in CHECKS {
        quiet_step(&mut steps, label, &mut cargo_with(*args));
    }
    if cfg!(target_os = "linux") {
        for (label, args) in LINUX_CHECKS {
            quiet_step(&mut steps, label, &mut cargo_with(*args));
        }
    } else {
        for (label, _) in LINUX_CHECKS {
            println!("STEP {label} SKIPPED - Linux only (it reads process groups through /proc)");
        }
    }
    crate::shipping::check(&mut steps);
    steps.finish()
}

/// A step whose output is kept back and shown, last 40 lines, only on failure.
fn quiet_step(steps: &mut Steps, label: &str, command: &mut Command) {
    let started = Instant::now();
    let output = run::captured(command);
    let secs = started.elapsed().as_secs();
    match output {
        Some(out) if out.status.success() => {
            println!("STEP {label} ok ({secs}s)");
            steps.record(true);
        }
        other => {
            println!("STEP {label} FAILED ({secs}s)");
            if let Some(out) = other {
                println!("{}", run::tail(&run::text(&out), 40));
            }
            steps.record(false);
        }
    }
}

/// The one step that counts rather than merely passing (see the floor).
fn client_tests(steps: &mut Steps) {
    let started = Instant::now();
    let output = run::captured(&mut cargo_with([
        "test",
        "-p",
        "baylee-client",
        "--features",
        CLIENT_FEATURES,
        "--all-targets",
    ]));
    let secs = started.elapsed().as_secs();
    let out = match output {
        Some(out) if out.status.success() => out,
        other => {
            println!("STEP client-features-test FAILED ({secs}s)");
            if let Some(out) = other {
                println!("{}", run::tail(&run::text(&out), 40));
            }
            steps.record(false);
            return;
        }
    };
    let log = run::text(&out);
    let (passed, devctl) = count_tests(&log);
    if devctl >= DEVCTL_TEST_FLOOR {
        println!(
            "STEP client-features-test ok ({passed} passed, {devctl} of them devctl::, floor {DEVCTL_TEST_FLOOR}, {secs}s)"
        );
        steps.record(true);
    } else {
        println!(
            "STEP client-features-test FAILED - {passed} tests passed and only {devctl} of them were"
        );
        println!(
            "  devctl:: tests, under the floor of {DEVCTL_TEST_FLOOR}. Zero means the feature did not apply and"
        );
        println!(
            "  the whole harness compiled out: a green suite measuring the default build under"
        );
        println!(
            "  another name. Short of the floor but not zero means tests were removed: re-measure"
        );
        println!("  and move the floor in a commit that says the number.");
        steps.record(false);
    }
}

/// The passed total over every `test result:` line and the `devctl::` tests
/// that passed: an ignored one is named in the log as readily as one that
/// ran, and the floor witnesses tests that ran (#171).
fn count_tests(log: &str) -> (usize, usize) {
    let passed = log
        .lines()
        .filter_map(|l| l.strip_prefix("test result: ok. "))
        .filter_map(|rest| rest.split_whitespace().next()?.parse::<usize>().ok())
        .sum();
    let devctl = log
        .lines()
        .filter(|l| l.starts_with("test devctl::") && l.trim_end().ends_with("... ok"))
        .count();
    (passed, devctl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_read_cargos_test_output() {
        let log = "test devctl::a ... ok\ntest devctl::b ... ok\ntest other ... ok\n\
                   test result: ok. 3 passed; 0 failed\ntest result: ok. 10 passed; 0 failed\n";
        assert_eq!(count_tests(log), (13, 2));
        // A default build's output: the harness compiled out, zero devctl.
        assert_eq!(
            count_tests("test result: ok. 400 passed; 0 failed\n"),
            (400, 0)
        );
    }

    /// An ignored test is named in the log too; the floor counts only the
    /// ones that ran and passed (#171).
    #[test]
    fn an_ignored_devctl_test_does_not_count() {
        let log = "test devctl::a ... ignored\ntest devctl::b ... ok\n\
                   test devctl::c ... ignored, needs a window\n\
                   test result: ok. 1 passed; 0 failed; 2 ignored\n";
        assert_eq!(count_tests(log), (1, 1));
    }
}
