//! Central's integration checks against an owned PostgreSQL 18 container
//! (`docs/central-login.md` §"Development and rollout").
//!
//! - `db`: `baylee-db`'s schema suite, plus named gateway e2e targets.
//! - `integration`: a real gateway and Central against the same isolated
//!   database (`baylee-gateway`'s ignored `e2e_central` tests, a module of its `e2e` test program).
//! - `client`: the native client driven through dev-control against that
//!   integration fixture.
//!
//! No production setting reaches a child: `POSTGRES_PASSWORD`, `PGPASSWORD`
//! and `DATABASE_URL` are removed before each test process gets its own.
//! `--exercise-failure` fails right after the container is up, to prove the
//! cleanup path; it reports `synthetic fixture failure` only after cleanup.
use crate::{
    fixture::{self, Owned, Postgres},
    run::cargo,
};
use anyhow::{Context, Result, bail, ensure};
use baylee_client_e2e::Devctl;
use serde_json::{Value, json};
use std::{
    fmt::Write as _,
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

/// The file the client reads its settings from. A wrong name is no error: the
/// client starts on its defaults, and `feedback_url` and `usage_count` written
/// here to keep a test run out of production are never applied.
const CLIENT_SETTINGS: &str = "client-settings.json";

/// The fixture's password for its synthetic account.
const FIXTURE_PASSWORD: &str = "synthetic legacy password";

/// The schema tests `db` requires by name: the legacy accounts' move to
/// Central. A suite that passes without them proves nothing about it.
const SCHEMA_MUST_RUN: &[&str] = &[
    "the_legacy_export_takes_unmoved_password_accounts_and_no_secrets",
    "a_moved_account_is_linked_at_its_first_admission_and_only_once",
];

/// The gateway e2e targets `db` may add after the schema suite.
pub const GATEWAY_TESTS: &[&str] = &[
    "e2e_account",
    "e2e_admin",
    "e2e_admission",
    "e2e_chair_tickets",
    "e2e_departure",
    "e2e_lobby_feed",
    "e2e_moderation",
    "e2e_rematch",
    "e2e_rooms",
    "e2e_seat_rate",
    "e2e_social_depart",
    "e2e_social_friends",
    "e2e_social_groups",
    "e2e_social_invites",
    "e2e_social_keys",
    "e2e_social_messages",
    "e2e_social_presence",
    "e2e_social_reports",
    "e2e_social_table",
    "e2e_spectate",
    "e2e_ws_tickets",
];

/// How a suite is built: the lockfile as committed, or an explicit local
/// patch configuration for the sibling Central crates (which necessarily
/// resolves the lockfile anew; restore the pins before committing).
fn cargo_options(config: Option<&Path>) -> Result<Vec<String>> {
    match config {
        None => Ok(vec!["--locked".into()]),
        Some(path) => {
            ensure!(path.is_file(), "the cargo config must be an existing file");
            Ok(vec![
                "--config".into(),
                fs::canonicalize(path)?.display().to_string(),
            ])
        }
    }
}

/// A `cargo test` with no ambient database secret.
fn test_command(options: &[String], args: &[&str]) -> Command {
    let mut command = cargo();
    command.arg("test").args(options).args(args);
    for name in ["POSTGRES_PASSWORD", "PGPASSWORD", "DATABASE_URL"] {
        command.env_remove(name);
    }
    command
}

/// Runs `body` against a fresh container and always cleans up; the body's
/// error and a cleanup failure are both reported.
fn with_postgres(
    prefix: &str,
    memory: &str,
    data_bytes: u64,
    failure: bool,
    body: impl FnOnce(&Postgres) -> Result<()>,
) -> Result<i32> {
    let postgres = Postgres::start(prefix, memory, data_bytes)?;
    let result = if failure {
        Err(anyhow::anyhow!("synthetic fixture failure"))
    } else {
        body(&postgres)
    };
    let cleanup = postgres.finish();
    match (result, cleanup) {
        (Ok(()), Ok(())) => {
            println!(
                "PostgreSQL fixture passed and its owned container and listener were removed."
            );
            Ok(0)
        }
        (Err(e), Ok(())) => Err(e),
        (Ok(()), Err(c)) => Err(c),
        (Err(e), Err(c)) => Err(e.context(c)),
    }
}

/// `db`: the schema suite, then each named gateway target, one at a time.
pub fn db(gateway_tests: &[String], config: Option<&Path>, failure: bool) -> Result<i32> {
    for test in gateway_tests {
        ensure!(
            GATEWAY_TESTS.contains(&test.as_str()),
            "{test} is not an allowlisted gateway target"
        );
    }
    let options = cargo_options(config)?;
    // Repeated gateway schema creation also generates WAL: the optional suite
    // gets a larger, still bounded, filesystem.
    let (memory, data) = if gateway_tests.is_empty() {
        ("512m", 268_435_456)
    } else {
        ("2g", 1_073_741_824)
    };
    with_postgres("baylee-central-db", memory, data, failure, |pg| {
        println!(
            "Testing real PostgreSQL {} in an isolated synthetic Baylee database.",
            pg.version()?
        );
        let mut suites = vec![("baylee-db", "schema")];
        let mut seen = std::collections::BTreeSet::new();
        suites.extend(
            gateway_tests
                .iter()
                .filter(|t| seen.insert(t.as_str()))
                .map(|t| ("baylee-gateway", t.as_str())),
        );
        for (package, target) in suites {
            println!("Testing {package}/{target}");
            // The gateway's end-to-end tests are one program (`tests/e2e.rs`),
            // each file a module: a suite is its module's tests.
            let module = format!("{target}::");
            let args: Vec<&str> = if package == "baylee-gateway" {
                vec![
                    "-p",
                    package,
                    "--test",
                    "e2e",
                    "--",
                    &module,
                    "--test-threads=1",
                ]
            } else {
                vec!["-p", package, "--test", target, "--", "--test-threads=1"]
            };
            let mut test = test_command(&options, &args);
            test.env("DATABASE_URL", pg.admin_url());
            let done = Owned::spawn(&mut test)?.wait(Duration::from_secs(300), None)?;
            println!("{}", done.stdout);
            if !done.status.success() {
                println!("{}", done.stderr);
                bail!("PostgreSQL integration tests failed");
            }
            ensure!(
                all_passed(&done.stdout),
                "the expected PostgreSQL suite did not run completely"
            );
            if target == "schema" {
                let missing = not_passed(&done.stdout, SCHEMA_MUST_RUN);
                ensure!(
                    missing.is_empty(),
                    "the legacy move's schema tests did not pass: {}",
                    missing.join(", ")
                );
            }
            pg.checkpoint()?;
        }
        Ok(())
    })
}

/// The tests of `names` that the libtest output does not report as passed.
fn not_passed<'a>(output: &str, names: &[&'a str]) -> Vec<&'a str> {
    names
        .iter()
        .copied()
        .filter(|name| {
            let passed = format!("test {name} ... ok");
            !output.lines().any(|line| line.trim_end() == passed)
        })
        .collect()
}

/// At least one test passed, none failed and none was ignored.
fn all_passed(output: &str) -> bool {
    output.lines().any(|line| {
        line.strip_prefix("test result: ok. ").is_some_and(|rest| {
            let n = rest
                .split_whitespace()
                .next()
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0);
            n > 0 && rest.contains("passed; 0 failed; 0 ignored;")
        })
    })
}

/// `integration`: builds first, so the bounded handoff to a UI run is spent
/// on the test and not the compile; then bootstraps Central's two roles and
/// runs the one ignored end-to-end test. `cancel` ends it early when it exists.
pub fn integration(config: Option<&Path>, cancel: Option<&Path>, failure: bool) -> Result<i32> {
    let options = cargo_options(config)?;
    let build = Owned::spawn(&mut test_command(
        &options,
        &["-p", "baylee-gateway", "--test", "e2e", "--no-run"],
    ))?
    .wait(Duration::from_secs(300), cancel)?;
    print!("{}{}", build.stdout, build.stderr);
    ensure!(
        build.status.success(),
        "building the integration fixture failed"
    );
    ensure!(
        !cancel.is_some_and(Path::exists),
        "fixture cancelled by its owner"
    );
    let (migrator, runtime, legacy) = (
        crate::secret::hex(32)?,
        crate::secret::hex(32)?,
        crate::secret::hex(32)?,
    );
    with_postgres("baylee-central-e2e", "512m", 268_435_456, failure, |pg| {
        // The secrets are generated hex, never caller-provided SQL.
        pg.sql(&bootstrap(&migrator, &runtime, &legacy))?;
        println!(
            "Testing real PostgreSQL {} with isolated migration and runtime roles.",
            pg.version()?
        );
        let mut test = test_command(
            &options,
            &[
                "-p",
                "baylee-gateway",
                "--test",
                "e2e",
                "--",
                "e2e_central::",
                "--ignored",
                "--test-threads=1",
            ],
        );
        test.env("CENTRAL_TEST_FIXTURE", "isolated-v1")
            .env("CENTRAL_TEST_PORT", pg.port().to_string())
            .env("CENTRAL_TEST_MIGRATOR_PASSWORD", &migrator)
            .env("CENTRAL_TEST_RUNTIME_PASSWORD", &runtime)
            .env("CENTRAL_TEST_LEGACY_PASSWORD", &legacy)
            .env("DATABASE_URL", pg.admin_url());
        let done = Owned::spawn(&mut test)?.wait(Duration::from_secs(300), cancel)?;
        println!("{}", done.stdout);
        if !done.status.success() {
            println!("{}", done.stderr);
            bail!("PostgreSQL integration tests failed");
        }
        ensure!(
            done.stdout
                .matches("test result: ok. 1 passed; 0 failed")
                .count()
                == 1,
            "the expected gateway and Central integration test did not run"
        );
        Ok(())
    })
}

/// Central's roles as its own fixture makes them: a migrator owning the two
/// test databases, a runtime role, the legacy store's role, and a NOINHERIT
/// member of the migrator the store must refuse.
fn bootstrap(migrator: &str, runtime: &str, legacy: &str) -> String {
    let mut sql = format!(
        "CREATE ROLE central_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{migrator}';\n\
         CREATE ROLE central_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{runtime}';\n\
         CREATE ROLE central_legacy LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{legacy}';\n\
         CREATE ROLE central_inherited LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{runtime}';\n\
         GRANT central_migrator TO central_inherited;\n"
    );
    for db in ["central_test", "central_native_test"] {
        let _ = writeln!(sql, "CREATE DATABASE {db} OWNER central_migrator;");
    }
    for db in ["central_native_test", "central_test"] {
        let _ = writeln!(
            sql,
            "\\connect {db}\nREVOKE CREATE, TEMP ON DATABASE {db} FROM PUBLIC;\nREVOKE CREATE ON SCHEMA public FROM PUBLIC;"
        );
    }
    sql
}

mod client;
pub use client::client;
#[cfg(test)]
use client::loopback_origin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scenario_names_what_the_client_and_the_fixture_name() {
        // Nothing in the gate runs the scenario: a wrong name here only shows
        // when somebody does, as `settings.json` did not for two weeks.
        let settings = include_str!("../../baylee-client/src/settings.rs");
        assert!(settings.contains(&format!("const SETTINGS: &str = \"{CLIENT_SETTINGS}\";")));
        let fixture = include_str!("../../baylee-gateway/tests/e2e_central.rs");
        assert!(fixture.contains(&format!("const PASSWORD: &str = \"{FIXTURE_PASSWORD}\";")));
    }

    #[test]
    fn a_suite_counts_only_when_every_test_ran_and_passed() {
        assert!(all_passed(
            "test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured"
        ));
        assert!(!all_passed(
            "test result: ok. 0 passed; 0 failed; 0 ignored;"
        ));
        assert!(!all_passed(
            "test result: ok. 3 passed; 0 failed; 1 ignored;"
        ));
        assert!(!all_passed(
            "test result: FAILED. 3 passed; 1 failed; 0 ignored;"
        ));
    }

    #[test]
    fn the_required_schema_tests_exist_and_must_each_pass() {
        // A rename in the schema suite fails here, in every gate, and not
        // only in the CI job that runs PostgreSQL.
        let suite = include_str!("../../baylee-db/tests/schema.rs");
        for name in SCHEMA_MUST_RUN {
            assert!(suite.contains(&format!("async fn {name}(")), "{name}");
        }
        let all = SCHEMA_MUST_RUN
            .iter()
            .map(|name| format!("test {name} ... ok"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(not_passed(&all, SCHEMA_MUST_RUN).is_empty());
        let one_failed = all.replacen(" ... ok", " ... FAILED", 1);
        assert_eq!(
            not_passed(&one_failed, SCHEMA_MUST_RUN),
            [SCHEMA_MUST_RUN[0]]
        );
        assert_eq!(not_passed("", SCHEMA_MUST_RUN), SCHEMA_MUST_RUN);
    }

    #[test]
    fn only_numeric_loopback_origins_are_fixture_origins() {
        assert!(loopback_origin("http://127.0.0.1:5432"));
        for bad in [
            "http://localhost:1",
            "https://127.0.0.1:1",
            "http://127.0.0.1:",
            "http://127.0.0.1:1/x",
            "http://10.0.0.1:1",
        ] {
            assert!(!loopback_origin(bad), "{bad}");
        }
    }

    #[test]
    fn the_bootstrap_makes_the_roles_and_revokes_public_rights() {
        let sql = bootstrap("aa", "bb", "cc");
        assert!(sql.contains("central_migrator") && sql.contains("PASSWORD 'aa'"));
        assert!(sql.contains("ROLE central_legacy") && sql.contains("PASSWORD 'cc'"));
        assert_eq!(
            sql.matches("REVOKE CREATE ON SCHEMA public FROM PUBLIC;")
                .count(),
            2
        );
        assert!(sql.contains("GRANT central_migrator TO central_inherited;"));
    }
}
