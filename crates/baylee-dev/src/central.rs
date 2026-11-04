//! Central's integration checks against an owned PostgreSQL 18 container
//! (`docs/central-login.md` §"Development and rollout").
//!
//! - `db`: `baylee-db`'s schema suite, plus named gateway e2e targets.
//! - `integration`: a real gateway and Central against the same isolated
//!   database (`baylee-gateway`'s ignored `e2e_central`).
//! - `client`: the native client driven through dev-control against that
//!   integration fixture.
//!
//! No production setting reaches a child: `POSTGRES_PASSWORD`, `PGPASSWORD`
//! and `DATABASE_URL` are removed before each test process gets its own.
//! `--exercise-failure` fails right after the container is up, to prove the
//! cleanup path; it reports `synthetic fixture failure` only after cleanup.
use crate::{
    devctl::Devctl,
    fixture::{self, Owned, Postgres},
    run::cargo,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    fmt::Write as _,
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

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
            let mut test = test_command(
                &options,
                &["-p", package, "--test", target, "--", "--test-threads=1"],
            );
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
        &["-p", "baylee-gateway", "--test", "e2e_central", "--no-run"],
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
                "e2e_central",
                "--",
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

/// The fixture's one synthetic account's password (`PASSWORD` in the gateway's
/// `e2e_central`): the ready file carries no credential, so it is named twice.
const FIXTURE_PASSWORD: &str = "synthetic legacy password";

/// The file the client reads its settings from (`settings::store::SETTINGS`).
const CLIENT_SETTINGS: &str = "client-settings.json";

/// A gateway only this device knows. It stands in the file so that "nothing of
/// the device's list while Central's form is up, and only the account's list
/// after it" has something to hide: with it gone the counts below still pass.
const DEVICE_ONLY_GATEWAY: &str = "https://unused-device.invalid";

/// The fixture's row. A list with no uses is ordered by address, and the
/// fixture's is the only `http:` one, so it comes first and the pinned live
/// gateway never does.
const FIXTURE_ROW: &str = "Front(SelectGateway(0))";

/// `client`: the dev-control client against the integration fixture, through
/// the door `docs/central-login.md` describes. Choosing the fixture from the
/// device's list turns the form into Central's with that list set aside; the
/// sign-in admits; leaving the gateway shows only the account's list, and its
/// row admits with a new one-use ticket; Central's sign-out brings the device's
/// list back, and a second sign-in admits again. The client runs on monitor
/// `monitor`, never the primary display, and every owned process and port is
/// proven gone afterwards.
pub fn client(monitor: i64) -> Result<i32> {
    // Absolute: the fixture is a `cargo test`, which runs in its package's
    // directory and not here, and takes this path through the environment.
    let run_dir = std::env::current_dir()?
        .join("target")
        .join(format!("central-ui-{}", crate::secret::hex(5)?));
    fs::create_dir_all(&run_dir)?;
    let ready = run_dir.join("fixture.json");
    let exe = PathBuf::from("target/debug")
        .join(format!("baylee-client{}", std::env::consts::EXE_SUFFIX));
    ensure!(
        exe.is_file(),
        "build the dev-control client first (`cargo build -p baylee-client --features dev-control`)"
    );

    let mut fixture = Owned::spawn(
        Command::new(std::env::current_exe()?)
            .args(["central", "integration", "--cancel-file"])
            .arg(run_dir.join("cancel"))
            .env("BAYLEE_CENTRAL_UI_READY", &ready),
    )?;
    let port = free_port()?;
    let mut client: Option<Owned> = None;
    let result = (|| -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(160);
        while !ready.exists() {
            ensure!(
                !fixture.exited(),
                "the synthetic backend failed before the UI handoff"
            );
            ensure!(
                Instant::now() < deadline,
                "the synthetic backend handoff timed out"
            );
            std::thread::sleep(Duration::from_millis(250));
        }
        let config: Value = serde_json::from_str(&fs::read_to_string(&ready)?)?;
        let origin = |key: &str| -> Result<String> {
            let value = config[key].as_str().unwrap_or_default();
            ensure!(
                loopback_origin(value),
                "unexpected fixture origin for {key}"
            );
            Ok(value.to_owned())
        };
        ensure!(
            config["mode"] == "synthetic-only",
            "unexpected fixture mode"
        );
        let (central, gateway) = (origin("central")?, origin("gateway")?);

        let home = run_dir.join("config");
        fs::create_dir_all(home.join("baylee"))?;
        fs::write(
            home.join("baylee").join(CLIENT_SETTINGS),
            json!({
                "lang": "en",
                "feedback_url": "",
                "usage_count": false,
                "gateways": [gateway, DEVICE_ONLY_GATEWAY],
            })
            .to_string(),
        )?;
        let started = Owned::spawn(
            Command::new(&exe)
                .env("XDG_CONFIG_HOME", &home)
                .env("XDG_CACHE_HOME", run_dir.join("cache"))
                .env("XDG_DATA_HOME", run_dir.join("data"))
                .env("BAYLEE_DEV_CONTROL", port.to_string())
                .env("BAYLEE_DEV_PASSIVE", "1")
                .env("BAYLEE_DEV_MONITOR", monitor.to_string())
                .env("BAYLEE_DEV_WINDOW", "1280x800")
                .env("BAYLEE_DEV_SOUND", "")
                .env("BAYLEE_NO_TOURS", "1")
                .env("BAYLEE_CENTRAL_DEV_ENDPOINT", &central)
                .env("BAYLEE_GATEWAY", &gateway)
                .env(
                    "BAYLEE_UPDATE_API",
                    format!("{gateway}/synthetic-updates-disabled"),
                ),
        )?;
        println!(
            "Owned client PID {}; artifacts {}",
            started.id(),
            run_dir.display()
        );
        let client = client.insert(started);
        scenario(&Devctl::new(port), client, monitor, &run_dir)
    })();

    let errors = stop(client, fixture, &ready, &run_dir, port);
    result?;
    ensure!(errors.is_empty(), "{}", errors.join("; "));
    println!("Owned client exited; client port closed; backend fixture cleanup verified.");
    Ok(0)
}

/// Stops the client, then the fixture (asked to stop first and told to cancel
/// only if it does not, so it still removes its own container), keeps their
/// logs and proves the client's port closed. Returns what could not be shown.
fn stop(
    client: Option<Owned>,
    mut fixture: Owned,
    ready: &Path,
    run_dir: &Path,
    port: u16,
) -> Vec<String> {
    let mut errors = Vec::new();
    if let Some(client) = client {
        match client.kill_and_collect() {
            Ok(done) => {
                let _ = fs::write(run_dir.join("client.stdout.log"), &done.stdout);
                let _ = fs::write(run_dir.join("client.stderr.log"), &done.stderr);
            }
            Err(_) => errors.push("owned client cleanup failed".to_owned()),
        }
    }
    let _ = fs::write(ready.with_extension("stop"), "stop");
    if !fixture.wait_for(Duration::from_secs(15)).unwrap_or(false) {
        let _ = fs::write(run_dir.join("cancel"), "cancel");
    }
    match fixture.wait(Duration::from_secs(30), None) {
        Ok(done) => {
            let _ = fs::write(run_dir.join("fixture.stdout.log"), &done.stdout);
            let _ = fs::write(run_dir.join("fixture.stderr.log"), &done.stderr);
            if !done.status.success() {
                errors.push("the backend fixture failed; inspect its log".to_owned());
            }
        }
        Err(_) => errors.push("backend fixture cleanup is unverified; inspect its log".to_owned()),
    }
    if fixture::refused(port).is_err() {
        errors.push("the owned client listener remains".to_owned());
    }
    errors
}

fn scenario(devctl: &Devctl, client: &mut Owned, monitor: i64, run_dir: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(45);
    while devctl.get("health", Duration::from_secs(2)).is_err() {
        ensure!(
            !client.exited() && Instant::now() < deadline,
            "the client harness did not start"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    let monitors = devctl.get("monitors", Duration::from_secs(5))?;
    fs::write(run_dir.join("monitors.json"), monitors.to_string())?;
    let chosen: Vec<&Value> = monitors["monitors"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["index"] == monitor)
        .collect();
    ensure!(
        chosen.len() == 1 && !at_the_origin(chosen[0]),
        "the selected monitor is the primary display, or is not there"
    );
    let deadline = Instant::now() + Duration::from_secs(40);
    while state(devctl)?["loading"]["destination"] != "hidden" {
        ensure!(
            !client.exited() && Instant::now() < deadline,
            "the client's startup overlay did not close"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    // The device's own list, Central off: the file's two and the pinned row.
    let device = |a: &Value| {
        a["central_pending"] == false
            && a["central_ready"] == false
            && a["gateway_authenticated"] == false
            && a["saved_gateway_count"] == 3
    };
    // Central's form: the device's list is set aside, none of it shown.
    let form = |a: &Value| {
        a["central_pending"] == true && a["central_ready"] == false && a["saved_gateway_count"] == 0
    };
    let admitted = |a: &Value| {
        a["central_ready"] == true
            && a["gateway_authenticated"] == true
            && a["offline"] == false
            && a["saved_gateway_count"] == 1
    };
    // Signed out of the gateway and not of Central: the account's list.
    let listed = |a: &Value| {
        a["central_ready"] == true
            && a["gateway_authenticated"] == false
            && a["saved_gateway_count"] == 1
    };
    let start = wait_account(devctl, client, run_dir, device)?;
    click(devctl, FIXTURE_ROW)?;
    let hidden = wait_account(devctl, client, run_dir, form)?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("01-login.png") }),
    )?;
    sign_in(devctl)?;
    let bound = wait_account(devctl, client, run_dir, admitted)?;
    got_it(devctl)?;
    leave_gateway(devctl, run_dir)?;
    let restored = wait_account(devctl, client, run_dir, listed)?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("02-gateways.png") }),
    )?;
    click(devctl, FIXTURE_ROW)?;
    let ticket = wait_account(devctl, client, run_dir, admitted)?;
    leave_gateway(devctl, run_dir)?;
    click(devctl, "Front(CentralSignOut)")?;
    let out = wait_account(devctl, client, run_dir, device)?;
    click(devctl, FIXTURE_ROW)?;
    wait_account(devctl, client, run_dir, form)?;
    sign_in(devctl)?;
    let again = wait_account(devctl, client, run_dir, admitted)?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("03-admitted.png") }),
    )?;
    let summary = json!({
        "device_list": start, "central_form": hidden, "admitted": bound,
        "restored_list": restored, "admitted_from_the_list": ticket,
        "signed_out": out, "admitted_again": again,
    });
    fs::write(run_dir.join("result.json"), summary.to_string())?;
    println!("Native Central login, restored gateway list and ticket admission passed.");
    Ok(())
}

/// Whether this `/monitors` row is the primary display, which starts at the
/// desktop's origin: the one a test client keeps off, whichever screen the
/// machine's own rule names instead (`.claude/skills/dev-control/SKILL.md`).
fn at_the_origin(monitor: &Value) -> bool {
    monitor["x"] == 0 && monitor["y"] == 0
}

fn sign_in(devctl: &Devctl) -> Result<()> {
    click(devctl, "Shared(Focus(Username))")?;
    devctl.post("text", &json!({ "text": "synthetic" }))?;
    click(devctl, "Shared(Focus(Password))")?;
    devctl.post("text", &json!({ "text": FIXTURE_PASSWORD }))?;
    click(devctl, "Front(Submit)")
}

fn state(devctl: &Devctl) -> Result<Value> {
    devctl.get("state", Duration::from_secs(12))
}

/// Presses the one visible lobby control with this `press` name.
fn click(devctl: &Devctl, press: &str) -> Result<()> {
    let state = state(devctl)?;
    let controls: Vec<&Value> = state["lobby_controls"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["press"] == press)
        .collect();
    ensure!(controls.len() == 1, "expected one visible control: {press}");
    devctl.post("pointer", &json!({ "x": controls[0]["at_x"], "y": controls[0]["at_y"], "button": "left", "press": true }))?;
    Ok(())
}

/// Answers the chat's first notice (`social::sheet`), which comes up over the
/// lobby a moment after the first admission and takes every press until then.
/// Pressed until it is gone, as a lobby coming in may not answer the first
/// ([`open_until_shown`]).
fn got_it(devctl: &Devctl) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(25);
    let mut answered = false;
    while Instant::now() < deadline {
        let state = state(devctl)?;
        let notice = state["chat"]["controls"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["press"] == "GotIt");
        match notice {
            Some(control) => {
                devctl.post("pointer", &json!({ "x": control["x"], "y": control["y"], "button": "left", "press": true }))?;
                answered = true;
            }
            None if answered && state["chat"]["sheet"].is_null() => return Ok(()),
            None => {}
        }
        std::thread::sleep(Duration::from_millis(750));
    }
    bail!("the chat's notice did not come up, or did not close")
}

/// From the lobby back to the gateway list: the session at the gateway ends and
/// Central's goes on, so the list drawn is the account's.
fn leave_gateway(devctl: &Devctl, run_dir: &Path) -> Result<()> {
    open_until_shown(devctl, run_dir, "Header(Account)", "Header(SignOut)")?;
    click(devctl, "Header(SignOut)")?;
    open_until_shown(
        devctl,
        run_dir,
        "Front(LeaveGateway)",
        "Front(CentralSignOut)",
    )
}

/// Presses `opener` until `shown` is drawn. A lobby coming in answers no press
/// (`lobby::clicks`), and `/state` does not say when it is in: the press is
/// repeated, and only while what it opens is not there, so it never closes it.
fn open_until_shown(devctl: &Devctl, run_dir: &Path, opener: &str, shown: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        click(devctl, opener)?;
        std::thread::sleep(Duration::from_millis(750));
        if has_control(&state(devctl)?, shown) {
            return Ok(());
        }
    }
    fs::write(
        run_dir.join("failure-state.json"),
        state(devctl)?.to_string(),
    )?;
    bail!("{opener} did not bring up {shown}")
}

fn has_control(state: &Value, press: &str) -> bool {
    state["lobby_controls"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|c| c["press"] == press)
}

/// The account state once it holds on two reads in a row with the front
/// panel at rest: admission is read from application state, not from a
/// transitional frame.
fn wait_account(
    devctl: &Devctl,
    client: &mut Owned,
    run_dir: &Path,
    holds: impl Fn(&Value) -> bool,
) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(35);
    let mut settled = 0;
    let mut last = Value::Null;
    while Instant::now() < deadline && !client.exited() {
        last = state(devctl)?;
        let account = &last["account"];
        settled = if account["front_moving"] != true && holds(account) {
            settled + 1
        } else {
            0
        };
        if settled >= 2 {
            return Ok(account.clone());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    fs::write(run_dir.join("failure-state.json"), last.to_string())?;
    bail!("the expected account state was not reached")
}

fn loopback_origin(value: &str) -> bool {
    value
        .strip_prefix("http://127.0.0.1:")
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("no free loopback port")?;
    Ok(listener.local_addr()?.port())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_scenario_names_what_the_client_and_the_fixture_name() {
        // Neither is read by anything in the gate: a wrong name here only
        // shows when somebody runs the scenario, as `settings.json` did not.
        let settings = include_str!("../../baylee-client/src/settings.rs");
        assert!(settings.contains(&format!("const SETTINGS: &str = \"{CLIENT_SETTINGS}\";")));
        let fixture = include_str!("../../baylee-gateway/tests/e2e_central.rs");
        assert!(fixture.contains(&format!("const PASSWORD: &str = \"{FIXTURE_PASSWORD}\";")));
    }

    #[test]
    fn the_primary_display_is_the_one_at_the_origin() {
        assert!(at_the_origin(&json!({ "x": 0, "y": 0 })));
        // The MacBook's built-in display and the workstation's left one.
        assert!(!at_the_origin(&json!({ "x": 6016, "y": 1112 })));
        assert!(!at_the_origin(&json!({ "x": -2560, "y": 864 })));
        // A screen above or below the primary shares its `x`.
        assert!(!at_the_origin(&json!({ "x": 0, "y": -1080 })));
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
