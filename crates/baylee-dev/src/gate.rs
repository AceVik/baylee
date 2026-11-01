//! The local gates. `full` is CI's gate before a push, `rules` the half that
//! never builds the client, `touched` a working filter over changed files,
//! `wasm` the browser crates, and [`features`](crate::features) the
//! non-default features `--workspace` never builds.
use crate::run::{self, Steps, cargo_with, command};
use anyhow::Result;
use std::{collections::BTreeSet, fs, path::Path};

/// The Scryfall payload cache `validate` reads. Nobody's to commit
/// (`docs/legal.md` §3), so a fresh checkout has none; its absence is said
/// out loud rather than skipped in silence.
const SCRYFALL_CACHE: &str = "data/scryfall-cache";

/// fmt, clippy, every test through nextest, `validate` and the API
/// documents (`api`), as CI runs them.
/// Every phase runs even when an earlier one failed. Needs Postgres for the
/// catalog and gateway tests (`DATABASE_URL`).
pub fn full() -> Result<i32> {
    run::require_env(
        "DATABASE_URL",
        "the catalog and gateway tests need Postgres",
    )?;
    warn_on_deps_directory();
    let mut steps = Steps::default();
    steps.run("fmt", &mut cargo_with(["fmt", "--all", "--check"]));
    steps.run(
        "clippy",
        &mut cargo_with([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
    );
    steps.run(
        "test",
        &mut cargo_with([
            "nextest",
            "run",
            "--workspace",
            "--all-targets",
            "--no-fail-fast",
        ]),
    );
    if Path::new(SCRYFALL_CACHE).is_dir() {
        // A header naming a printing id that does not exist is fatal in
        // `scryfall-cache` and only uncountable in `validate`.
        steps.run("scryfall-cache", &mut xtask("scryfall-cache"));
        steps.run("validate", &mut xtask("validate"));
    } else {
        println!("STEP validate SKIPPED - no {SCRYFALL_CACHE}");
    }
    steps.run("api", &mut crate::api::command("check"));
    steps.run("api-tests", &mut crate::api::tests());
    Ok(steps.finish())
}

/// The rules half: everything but the client and wasm, stopping at the first
/// red. `baylee-client` dominates a `--workspace` run, and an engine or card
/// change that rebuilt it paid for a renderer it had not touched. A filter,
/// not a replacement: it says nothing about the client, and CI runs it all.
///
/// `validate` is part of it because the one check that reads a card against
/// its printing was the one nothing local ran (Invasion of Ikoria carried a
/// battle subtype no `SubtypeKind` had for six commits). It needs the
/// Scryfall cache; `validate` itself prints the cache's age.
pub fn rules() -> i32 {
    let steps: [(&str, &[&str]); 3] = [
        ("fmt", &["fmt", "--all"]),
        ("lint", &["lint-rules"]),
        ("test", &["test-rules"]),
    ];
    for (name, args) in steps {
        if !run::succeeds(&mut cargo_with(args)) {
            println!("STEP {name} failed");
            println!("done rc=1");
            return 1;
        }
        println!("STEP {name} ok");
    }
    if Path::new(SCRYFALL_CACHE).is_dir() {
        for task in ["scryfall-cache", "validate"] {
            if !run::succeeds(&mut xtask(task)) {
                println!("STEP validate failed ({task})");
                println!("done rc=1");
                return 1;
            }
        }
        println!("STEP validate ok");
    } else {
        println!(
            "STEP validate SKIPPED - no {SCRYFALL_CACHE}; run `cargo run -p xtask -- scryfall-cache`"
        );
    }
    println!("done rc=0");
    0
}

/// An opt-in check for the experimental browser build (coming soon), outside
/// CI and landing. All eight crates are named: cargo unifies features across
/// the members a command selects, so
/// `-p baylee-client` would resolve the other seven with whatever the client
/// asks for. Naming them checks each on its own default features.
pub fn wasm() -> i32 {
    let mut command = cargo_with(["check", "--locked"]);
    for krate in [
        "baylee-cardtext",
        "baylee-core",
        "baylee-deckio",
        "baylee-protocol",
        "baylee-seal",
        "baylee-view",
        "baylee-client-core",
        "baylee-client",
    ] {
        command.args(["-p", krate]);
    }
    command.args(["--target", "wasm32-unknown-unknown"]);
    run::code(run::status(&mut command))
}

/// fmt and clippy over what changed, and nothing else. Not a gate: it says
/// nothing about a crate that merely depends on what changed, and a new enum
/// variant breaks those and not this.
///
/// The halves differ in grain because the tools do: rustfmt takes a file,
/// clippy lints by compiling, so its smallest unit is the crate.
pub fn touched(since: Option<&str>) -> Result<i32> {
    let files = changed_rust_files(since)?;
    if files.is_empty() {
        println!("gate touched: no .rs file has changed — nothing to format or lint");
        return Ok(0);
    }
    println!("gate touched: {} changed .rs file(s)", files.len());

    // rustfmt directly, with the edition said: without it rustfmt assumes
    // 2015. It follows the `mod` tree below a file, so naming a `lib.rs`
    // formats its subtree too: more than asked for, never less.
    let mut rustfmt = command("rustfmt", ["--edition", &workspace_edition()]);
    rustfmt.args(&files);
    let fmt_rc = run::code(run::status(&mut rustfmt));
    println!("STEP fmt rc={fmt_rc}");

    let crates: BTreeSet<String> = files
        .iter()
        .filter_map(|f| owning_crate(Path::new(f)))
        .collect();
    if crates.is_empty() {
        println!("gate touched: no crate owns those files — nothing to lint");
        return Ok(fmt_rc);
    }
    let mut rc = fmt_rc;
    for krate in &crates {
        println!("--- clippy -p {krate}");
        let ok = run::succeeds(&mut cargo_with([
            "clippy",
            "-p",
            krate,
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]));
        if !ok {
            rc = 1;
        }
    }
    println!("STEP clippy rc={rc}");
    let names: Vec<&str> = crates.iter().map(String::as_str).collect();
    println!("done rc={rc}  (crates: {})", names.join(" "));
    Ok(rc)
}

fn xtask(task: &str) -> std::process::Command {
    cargo_with(["run", "-q", "-p", "xtask", "--", task])
}

/// Every rustc lists `target/debug/deps` when it starts. On macOS that
/// directory once held 1.3 M stale objects and made a gate take 13 minutes;
/// the rustflags in `.cargo/config.toml` stop that, and this says so if it
/// comes back. A directory's size is the size of its entry table there.
fn warn_on_deps_directory() {
    if !cfg!(target_os = "macos") {
        return;
    }
    if let Ok(meta) = fs::metadata("target/debug/deps")
        && meta.len() > 4_000_000
    {
        println!(
            "WARN target/debug/deps is {} bytes of directory entries - every rustc lists it.",
            meta.len()
        );
        println!(
            "     Sweep it: rename target/debug away and delete the old one in the background."
        );
    }
}

/// Changed `.rs` files: since a revision, or the working tree's tracked
/// changes, staged changes and untracked files (a brand-new card file is
/// exactly what a "what did I touch" question must not miss).
fn changed_rust_files(since: Option<&str>) -> Result<Vec<String>> {
    let queries: Vec<Vec<&str>> = match since {
        Some(rev) => vec![vec![
            "diff",
            "--name-only",
            "--diff-filter=d",
            rev,
            "--",
            "*.rs",
        ]],
        None => vec![
            vec!["diff", "--name-only", "--diff-filter=d", "--", "*.rs"],
            vec![
                "diff",
                "--name-only",
                "--diff-filter=d",
                "--cached",
                "--",
                "*.rs",
            ],
            vec!["ls-files", "--others", "--exclude-standard", "--", "*.rs"],
        ],
    };
    let mut files = BTreeSet::new();
    for args in queries {
        let out = command("git", &args).output()?;
        anyhow::ensure!(out.status.success(), "git {} failed", args.join(" "));
        files.extend(String::from_utf8(out.stdout)?.lines().map(str::to_owned));
    }
    Ok(files.into_iter().collect())
}

/// A file's crate: the nearest ancestor holding a `Cargo.toml`, named as
/// `-p` wants it. Walking up beats a path table, which goes stale with the
/// first new crate.
fn owning_crate(file: &Path) -> Option<String> {
    let mut dir = file.parent();
    while let Some(d) = dir.filter(|d| !d.as_os_str().is_empty()) {
        if let Ok(manifest) = fs::read_to_string(d.join("Cargo.toml")) {
            return manifest_value(&manifest, "name");
        }
        dir = d.parent();
    }
    None
}

/// The workspace's edition, `2024` when the manifest does not say.
fn workspace_edition() -> String {
    fs::read_to_string("Cargo.toml")
        .ok()
        .and_then(|m| manifest_value(&m, "edition"))
        .unwrap_or_else(|| "2024".into())
}

/// The first `key = "value"` line of a manifest.
fn manifest_value(manifest: &str, key: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_value_is_the_first_matching_key() {
        let manifest = "[package]\nname = \"baylee-dev\"\nedition.workspace = true\n";
        assert_eq!(
            manifest_value(manifest, "name").as_deref(),
            Some("baylee-dev")
        );
        assert_eq!(manifest_value(manifest, "edition"), None);
        assert_eq!(
            manifest_value("edition = \"2024\"\n", "edition").as_deref(),
            Some("2024")
        );
    }

    #[test]
    fn a_file_belongs_to_its_nearest_crate() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            owning_crate(&here.join("src/gate.rs")).as_deref(),
            Some("baylee-dev")
        );
        // A path with no manifest above it inside the walk owns nothing.
        assert_eq!(owning_crate(Path::new("nowhere.rs")), None);
    }
}
