//! Owned native Central client checks.
#[allow(clippy::wildcard_imports)]
use super::*;

/// `client`: the dev-control client against the integration fixture. It
/// signs in, sees exactly the restored gateway list, signs out (access gone),
/// signs in again and is admitted to the gateway with a one-use ticket. The
/// client runs on monitor `monitor` (the additional left display) and every
/// owned process and port is proven gone afterwards.
pub fn client(monitor: i64) -> Result<i32> {
    let run_dir = PathBuf::from("target").join(format!("central-ui-{}", crate::secret::hex(5)?));
    fs::create_dir_all(&run_dir)?;
    let run_dir = fs::canonicalize(&run_dir)?;
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
            r#"{"lang":"en","feedback_url":"","usage_count":false,"gateways":["https://unused-device.invalid"]}"#,
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
        chosen.len() == 1 && chosen[0]["index"].as_i64().is_some_and(|index| index > 0),
        "the selected monitor is not an additional non-primary display"
    );
    let deadline = Instant::now() + Duration::from_secs(40);
    while state(devctl)?["loading"]["destination"] != "hidden" {
        ensure!(
            !client.exited() && Instant::now() < deadline,
            "the client's startup overlay did not close"
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    let signed_out = |a: &Value| a["central_pending"] == true && a["central_ready"] == false;
    let restored = |a: &Value| {
        a["central_ready"] == true && a["central_pending"] == false && a["saved_gateway_count"] == 1
    };
    let initial = wait_account(devctl, client, run_dir, |a| {
        signed_out(a) && a["saved_gateway_count"] == 0
    })?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("01-login.png") }),
    )?;
    hover(devctl, "Front(OpenCommunityDialog(WhatsApp))")?;
    hover(devctl, "Memorial(Open)")?;
    std::thread::sleep(Duration::from_millis(450));
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("01b-logo-hover.png") }),
    )?;
    click(devctl, "Memorial(Open)")?;
    wait_control(devctl, client, "Memorial(Close)")?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("01c-memorial.png") }),
    )?;
    click(devctl, "Memorial(Close)")?;
    wait_control(devctl, client, "Front(Submit)")?;
    sign_in(devctl)?;
    wait_account(devctl, client, run_dir, restored)?;
    got_it(devctl)?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("02-gateways.png") }),
    )?;
    sign_out(devctl, run_dir)?;
    let out = wait_account(devctl, client, run_dir, |a| {
        signed_out(a) && a["gateway_authenticated"] == false && a["saved_gateway_count"] == 0
    })?;
    sign_in(devctl)?;
    let again = wait_account(devctl, client, run_dir, restored)?;
    click(devctl, "Front(SelectGateway(0))")?;
    let admitted = wait_account(devctl, client, run_dir, |a| {
        a["central_ready"] == true && a["gateway_authenticated"] == true && a["offline"] == false
    })?;
    devctl.post(
        "screenshot",
        &json!({ "path": run_dir.join("03-admitted.png") }),
    )?;
    let summary =
        json!({ "initial": initial, "restored": again, "signed_out": out, "admitted": admitted });
    fs::write(run_dir.join("result.json"), summary.to_string())?;
    println!("Native Central login, restored gateway list and ticket admission passed.");
    Ok(())
}

fn sign_in(devctl: &Devctl) -> Result<()> {
    click(devctl, "Shared(Focus(Username))")?;
    devctl.post("text", &json!({ "text": "synthetic" }))?;
    click(devctl, "Shared(Focus(Password))")?;
    devctl.post("text", &json!({ "text": FIXTURE_PASSWORD }))?;
    click(devctl, "Front(Submit)")
}

/// Answers the chat's first notice (`social::sheet`), which comes up over the
/// account's list a moment after the first sign-in and takes every press until
/// then. Pressed until it is gone, as the first press may come too early.
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
                devctl.post(
                    "pointer",
                    &json!({ "x": control["x"], "y": control["y"], "button": "left", "press": true }),
                )?;
                answered = true;
            }
            None if answered && state["chat"]["sheet"].is_null() => return Ok(()),
            None => {}
        }
        std::thread::sleep(Duration::from_millis(750));
    }
    bail!("the chat's notice did not come up, or did not close")
}

/// Signs out of Central through the account menu, which on the account's list
/// is the way out. A press there may not land the moment the menu is drawn,
/// so the menu is opened and its sign-out pressed until Central's form is back.
fn sign_out(devctl: &Devctl, run_dir: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        let state = state(devctl)?;
        if state["account"]["central_pending"] == true {
            return Ok(());
        }
        let shown = state["lobby_controls"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| c["press"] == "Header(SignOut)");
        click(
            devctl,
            if shown {
                "Header(SignOut)"
            } else {
                "Header(Account)"
            },
        )?;
        std::thread::sleep(Duration::from_millis(750));
    }
    fs::write(
        run_dir.join("failure-state.json"),
        state(devctl)?.to_string(),
    )?;
    bail!("signing out of Central did not bring its form back")
}

fn state(devctl: &Devctl) -> Result<Value> {
    devctl.get("state", Duration::from_secs(12))
}

/// Moves the pointer over the one visible lobby control with this `press` name.
fn hover(devctl: &Devctl, press: &str) -> Result<()> {
    let state = state(devctl)?;
    let controls: Vec<&Value> = state["lobby_controls"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["press"] == press)
        .collect();
    ensure!(controls.len() == 1, "expected one visible control: {press}");
    devctl.post(
        "pointer",
        &json!({ "x": controls[0]["at_x"], "y": controls[0]["at_y"], "button": "left", "press": false }),
    )?;
    Ok(())
}

/// Presses the visible lobby control with this `press` name (if a backdrop
/// scrim shares the same press, chooses the smaller button).
fn click(devctl: &Devctl, press: &str) -> Result<()> {
    let state = state(devctl)?;
    let mut controls: Vec<&Value> = state["lobby_controls"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["press"] == press)
        .collect();
    ensure!(!controls.is_empty(), "expected visible control: {press}");
    controls.sort_by(|a, b| {
        let aw = a["w"].as_f64().unwrap_or(f64::MAX);
        let bw = b["w"].as_f64().unwrap_or(f64::MAX);
        aw.total_cmp(&bw)
    });
    devctl.post("pointer", &json!({ "x": controls[0]["at_x"], "y": controls[0]["at_y"], "button": "left", "press": true }))?;
    Ok(())
}

/// Waits until a visible lobby control with this `press` name is present.
fn wait_control(devctl: &Devctl, client: &mut Owned, press: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !client.exited() {
        let state = state(devctl)?;
        let count = state["lobby_controls"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c["press"] == press)
            .count();
        if count >= 1 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    bail!("the expected control did not appear: {press}")
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

pub(super) fn loopback_origin(value: &str) -> bool {
    value
        .strip_prefix("http://127.0.0.1:")
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("no free loopback port")?;
    Ok(listener.local_addr()?.port())
}

