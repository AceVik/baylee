use super::{DEFAULT_MONITOR, StackState, loopback_http_origin, services::parse_http_200_body};
use std::path::PathBuf;

#[test]
fn default_interactive_monitor_is_left_2k_index_two() {
    assert_eq!(DEFAULT_MONITOR, 2);
}

#[test]
fn loopback_http_origin_accepts_only_numeric_ipv4_loopback_with_nonzero_port() {
    assert!(loopback_http_origin("http://127.0.0.1:28790"));
    assert!(loopback_http_origin("http://127.0.0.1:28794"));
    for bad in [
        "https://baylee.app",
        "https://127.0.0.1:28790",
        "http://localhost:28790",
        "http://127.0.0.1:0",
        "http://127.0.0.1:28790/",
        "http://127.0.0.1:28790/v1",
        "http://192.168.0.53:28790",
    ] {
        assert!(!loopback_http_origin(bad), "{bad}");
    }
}

#[test]
fn stack_state_json_contains_no_secret_material() {
    let state = StackState {
        supervisor_pid: 42,
        postgres_port: 25432,
        gateway_port: 28766,
        admin_port: 28767,
        feedback_port: 28780,
        gateway_origin: "http://127.0.0.1:28766".into(),
        central_endpoint: Some("http://127.0.0.1:28790".into()),
        central_web_origin: Some("http://127.0.0.1:28794".into()),
        owned_ports: super::OWN_CENTRAL_PORTS.to_vec(),
        account_file: Some(PathBuf::from("target/local-stack/central-account.json")),
        central_revision: Some("../baylee-central at 0123abc (main)".to_owned()),
        monitor: 2,
        logs_dir: PathBuf::from("target/local-stack/logs"),
        client_pid: Some(99),
    };
    let json = serde_json::to_string(&state).unwrap();
    for forbidden in ["password", "token", "secret", "postgres://"] {
        assert!(
            !json.contains(forbidden),
            "unexpected {forbidden} in {json}"
        );
    }
    let decoded: StackState = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, state);
}

#[test]
fn parse_http_200_body_extracts_ok_payload_and_rejects_errors() {
    let ok = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true}";
    assert_eq!(parse_http_200_body(ok).as_deref(), Some("{\"ok\":true}"));
    let err = b"HTTP/1.1 503 Service Unavailable\r\n\r\n{\"ok\":false}";
    assert_eq!(parse_http_200_body(err), None);
}

#[test]
fn an_unset_or_empty_locale_is_not_a_locale() {
    use super::postgres::names_a_locale;
    let some = |text: &str| Some(std::ffi::OsString::from(text));
    assert!(!names_a_locale(None, None));
    assert!(!names_a_locale(some(""), some("")));
    assert!(names_a_locale(some("C"), None));
    assert!(names_a_locale(None, some("de_DE.UTF-8")));
    assert!(names_a_locale(some(""), some("en_US.UTF-8")));
}

#[test]
fn a_second_supervisor_cannot_replace_data_or_take_ownership() {
    let root =
        std::env::temp_dir().join(format!("baylee-stack-{}", crate::secret::hex(12).unwrap()));
    std::fs::create_dir(&root).unwrap();
    let result = (|| -> anyhow::Result<()> {
        let guard = super::lock_stack(&root)?;
        assert!(super::lock_stack(&root).is_err());
        let data = root.join("pgdata");
        super::postgres::claim_data_dir(&data)?;
        std::fs::write(data.join("sentinel"), b"existing data")?;
        assert!(super::postgres::claim_data_dir(&data).is_err());
        assert_eq!(std::fs::read(data.join("sentinel"))?, b"existing data");
        drop(guard);
        let _guard = super::lock_stack(&root)?;
        Ok(())
    })();
    std::fs::remove_dir_all(&root).unwrap();
    result.unwrap();
}

/// A `stack.json` written before `--central` existed still reads: `status`,
/// `down` and `client` take it from whichever supervisor is running.
#[test]
fn an_older_state_file_reads_without_the_central_fields() {
    let old = r#"{"supervisor_pid":1,"postgres_port":2,"gateway_port":3,"admin_port":4,
        "feedback_port":5,"gateway_origin":"http://127.0.0.1:3","central_endpoint":null,
        "monitor":2,"logs_dir":"logs","client_pid":null}"#;
    let state: StackState = serde_json::from_str(old).unwrap();
    assert!(state.owned_ports.is_empty());
    assert_eq!(state.central_web_origin, None);
    assert_eq!(state.account_file, None);
}

fn environment(auth: &super::Auth<'_>) -> std::collections::BTreeMap<&'static str, String> {
    auth.environment().into_iter().collect()
}

#[test]
fn the_gateway_is_told_its_sign_in_mode_and_nothing_it_does_not_have() {
    let guests = environment(&super::Auth::Guests);
    assert_eq!(guests.len(), 1);
    assert_eq!(guests["BAYLEE_AUTH_MODE"], "guests");

    let bridge = PathBuf::from("/state/bridge.json");
    let attached = environment(&super::Auth::Attached {
        endpoint: "http://127.0.0.1:28790",
        bridge: &bridge,
    });
    assert_eq!(attached["BAYLEE_AUTH_MODE"], "mixed");
    assert_eq!(
        attached["BAYLEE_CENTRAL_ENDPOINT"],
        "http://127.0.0.1:28790"
    );
    assert_eq!(attached["BAYLEE_CENTRAL_CONFIG_FILE"], "/state/bridge.json");
    assert!(
        attached.keys().all(|name| !name.contains("TOWER")),
        "someone else's Central brings no Tower authority of ours"
    );

    let host = "h".repeat(64);
    let own = environment(&super::Auth::Own {
        bridge: &bridge,
        tower_host: &host,
    });
    assert_eq!(own["BAYLEE_AUTH_MODE"], "mixed");
    assert_eq!(own["BAYLEE_REGISTRATION"], "open");
    assert_eq!(own["BAYLEE_CENTRAL_ENDPOINT"], "http://127.0.0.1:28790");
    assert_eq!(own["BAYLEE_TOWER_ORIGIN"], "http://127.0.0.1:28795");
    assert_eq!(own["BAYLEE_TOWER_HOST_TOKEN"], host);
    // Without it the gateway insists on an HTTPS Tower origin.
    assert_eq!(own["BAYLEE_TOWER_DEVELOPMENT"], "1");
}

/// "Register" in the client opens `BAYLEE_CENTRAL_WEB_ORIGIN`; the client's
/// fallback from the endpoint alone is four ports up, and both must name the
/// website this stack serves.
#[test]
fn the_website_is_where_the_client_looks_for_it() {
    use super::central::{NATIVE_PORT, WEBSITE_PORT, origin};
    assert_eq!(WEBSITE_PORT, NATIVE_PORT + 4);
    assert!(loopback_http_origin(&origin(WEBSITE_PORT)));
    let ports = super::OWN_CENTRAL_PORTS;
    let mut distinct = ports.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), ports.len());
    assert!(!ports.contains(&super::DEFAULT_GATEWAY_PORT));
}

#[test]
fn a_checkouts_migrations_must_be_the_pinned_ones() {
    use super::central::first_difference;
    let names = |list: &[&str]| {
        list.iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>()
    };
    let pinned = names(&["m1_accounts", "m2_tower"]);
    assert_eq!(first_difference(&pinned, &pinned), None);
    assert_eq!(
        first_difference(&pinned, &names(&["m1_accounts", "m2_tower", "m3_new"])).as_deref(),
        Some("the checkout adds m3_new")
    );
    assert_eq!(
        first_difference(&pinned, &names(&["m1_accounts"])).as_deref(),
        Some("the checkout lacks m2_tower")
    );
}

/// A cargo started in the Central checkout must find that checkout's
/// toolchain and target directory, and keep the user's own settings.
#[test]
fn the_checkouts_cargo_sheds_this_workspaces_variables_only() {
    use super::central::name_is_this_workspaces;
    use std::ffi::OsStr;
    for shed in [
        "CARGO",
        "RUSTUP_TOOLCHAIN",
        "CARGO_TARGET_DIR",
        "CARGO_MANIFEST_DIR",
        "CARGO_PKG_NAME",
        "CARGO_BIN_NAME",
    ] {
        assert!(name_is_this_workspaces(OsStr::new(shed)), "{shed}");
    }
    for kept in ["CARGO_HOME", "RUSTUP_HOME", "PATH", "RUSTC_WRAPPER", "HOME"] {
        assert!(!name_is_this_workspaces(OsStr::new(kept)), "{kept}");
    }
}

#[test]
fn hex_is_two_lowercase_digits_a_byte() {
    assert_eq!(super::central::hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
}

/// The site server's settings carry no secret and no production host.
#[test]
fn the_site_server_is_pointed_at_the_local_central_only() {
    let settings = super::central::site_settings(
        std::path::Path::new("/state/central-site"),
        std::path::Path::new("/checkout/fallback"),
    );
    assert_eq!(settings["origin"], "http://127.0.0.1:28794");
    assert_eq!(settings["central"], "127.0.0.1:28791");
    assert_eq!(settings["listen"], "127.0.0.1:28792");
    assert_eq!(settings["github"], false);
    assert!(!settings.to_string().contains("baylee.app"));
}

/// The authority's configuration is one development worker for the one
/// cohort, with the award credentials Central was provisioned for.
#[test]
fn the_tower_authority_serves_the_published_cohort() {
    let config = super::tower::config_for_test("01a1edec-29bf-718e-8e40-952eb5a689b1");
    assert_eq!(config["development"], true);
    let service = &config["service"];
    assert_eq!(service["listen"], "127.0.0.1:28795");
    assert_eq!(service["origin"], "http://127.0.0.1:28795");
    assert_eq!(service["central"], "http://127.0.0.1:28790");
    let workers = service["workers"].as_array().unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0]["cohort"], "01a1edec-29bf-718e-8e40-952eb5a689b1");
    assert_eq!(workers[0]["artifact"], "/stack/bin/baylee-engine-tower");
    assert_eq!(workers[0]["verifier"], "v".repeat(64));
    assert_eq!(service["awards"]["issuer"], "i".repeat(64));
    assert_eq!(service["awards"]["courier"], "c".repeat(64));
    // The host and publisher credentials are not the authority's to hold.
    let text = config.to_string();
    assert!(!text.contains(&"h".repeat(64)) && !text.contains(&"p".repeat(64)));
}

#[test]
fn the_task_stock_publishes_today_first_and_then_the_days_ahead() {
    use super::stock::Pass;
    assert_eq!(
        Pass::Today.arguments(7),
        ["--activities", "calendar", "--days", "1"]
    );
    assert_eq!(
        Pass::Ahead.arguments(7),
        ["--activities", "all", "--days", "7"]
    );
    assert_eq!(Pass::Today.next(7), Some(Pass::Ahead));
    // One day asked for is today's alone: no second pass, no expedition.
    assert_eq!(Pass::Today.next(1), None);
    assert_eq!(Pass::Ahead.next(7), None);
    assert_ne!(Pass::Today.name(), Pass::Ahead.name());
}

#[test]
fn a_pass_reports_the_last_thing_it_said() {
    use super::stock::{Pass, last_line};
    let logs =
        std::env::temp_dir().join(format!("baylee-stock-{}", crate::secret::hex(8).unwrap()));
    std::fs::create_dir(&logs).unwrap();
    let result = std::panic::catch_unwind(|| {
        assert_eq!(last_line(&logs, Pass::Today), None);
        std::fs::write(
            logs.join("challenge-stock-today.err.log"),
            "a warning on stderr\n",
        )
        .unwrap();
        assert_eq!(
            last_line(&logs, Pass::Today).as_deref(),
            Some("a warning on stderr")
        );
        // An empty stdout log does not hide what stderr said.
        std::fs::write(logs.join("challenge-stock-today.out.log"), "\n").unwrap();
        assert_eq!(
            last_line(&logs, Pass::Today).as_deref(),
            Some("a warning on stderr")
        );
        std::fs::write(
            logs.join("challenge-stock-today.out.log"),
            "Daily 2026-11-18\nChallenge stock: 0 already present, 2 published\n\n",
        )
        .unwrap();
        assert_eq!(
            last_line(&logs, Pass::Today).as_deref(),
            Some("Challenge stock: 0 already present, 2 published")
        );
    });
    std::fs::remove_dir_all(&logs).unwrap();
    result.unwrap();
}

#[test]
fn a_private_file_is_readable_by_its_owner_only_and_replaces_an_older_one() {
    let dir =
        std::env::temp_dir().join(format!("baylee-private-{}", crate::secret::hex(8).unwrap()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("secret.json");
    let result = (|| -> anyhow::Result<()> {
        std::fs::write(&path, b"older, world-readable")?;
        super::services::write_private(&path, b"{}")?;
        assert_eq!(std::fs::read(&path)?, b"{}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(&path)?.permissions().mode() & 0o777,
                0o600
            );
        }
        Ok(())
    })();
    std::fs::remove_dir_all(&dir).unwrap();
    result.unwrap();
}

/// The website's readiness is its status line: the front page is larger than
/// the body probe reads, and a cut inside a character is no valid text.
#[test]
fn a_status_is_read_from_the_first_bytes_of_a_response() {
    use super::services::parse_http_status;
    assert_eq!(
        parse_http_status(b"HTTP/1.1 200 OK\r\ncontent-type: text/html"),
        Some(200)
    );
    assert_eq!(parse_http_status(b"HTTP/1.1 404 Not Found\r\n"), Some(404));
    // Sixty-four bytes of a response end anywhere, also inside a character.
    assert_eq!(
        parse_http_status(b"HTTP/1.1 302 Found\r\nlocation: /\xc3"),
        Some(302)
    );
    assert_eq!(
        parse_http_status(b"HTTP/1.1 20"),
        Some(20),
        "a cut status is not 200"
    );
    assert_eq!(parse_http_status(b"SSH-2.0-OpenSSH\r\n"), None);
    assert_eq!(parse_http_status(b""), None);
}
