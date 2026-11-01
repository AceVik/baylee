use super::*;
use crate::teacher::account_username;
use std::sync::Arc;
use tokio::net::TcpListener;

#[test]
fn only_an_api_profile_names_the_one_key_its_seat_receives() {
    let dir = crate::audit::tests::TestDir::new("key-env");
    let config = dir.path().join("profiles.json");
    std::fs::write(
        &config,
        json!({"profiles": {
            "cli": {"provider": "cli", "model": "codex"},
            "deepseek": {"provider": "openai", "model": "deepseek-flash",
                "key_env": "DEEPSEEK_API_KEY"},
            "plain": {"provider": "openai", "model": "local"},
            "claude": {"provider": "anthropic", "model": "claude-opus-5-5"},
        }})
        .to_string(),
    )
    .unwrap();
    let key = |profile| profile_key_env(&config, profile).unwrap();
    assert_eq!(key("cli"), None);
    assert_eq!(key("deepseek").as_deref(), Some("DEEPSEEK_API_KEY"));
    assert_eq!(key("plain").as_deref(), Some("BAYLEE_LLM_API_KEY"));
    assert_eq!(key("claude").as_deref(), Some("ANTHROPIC_API_KEY"));
    assert!(profile_key_env(&config, "missing").is_err());
}

/// A profiles file whose agent CLI profile `cc` names two fallbacks, the
/// same model behind Anthropic's API first, then `DeepSeek`'s.
fn chained_profiles(dir: &Path) -> PathBuf {
    let config = dir.join("profiles.json");
    write_atomic_json(
        &config,
        &json!({"profiles": {
            "cc": {"provider": "cli", "model": "codex:gpt-6.1-terra", "effort": "low",
                   "game_tokens": 100_000, "fallback": ["opus-api", "ds"]},
            "opus-api": {"provider": "anthropic", "model": "claude-opus-5-5",
                         "effort": "low", "game_usd": 1},
            "ds": {"provider": "openai", "model": "deepseek-flash",
                   "base_url": "https://api.deepseek.com/v1", "key_env": "DEEPSEEK_API_KEY",
                   "game_tokens": 100_000},
            "other": {"provider": "openai", "model": "gpt-6.1-sol", "key_env": "OTHER_KEY",
                      "game_tokens": 100_000},
        }}),
    )
    .unwrap();
    config
}

/// A seat's bridge is told its chain, each fallback a `--fallback-profile`
/// in the order it takes over, and handed the key variable of every API
/// profile of that chain, once each, and of nothing else.
#[test]
fn a_seat_is_started_with_its_chain_and_the_keys_of_that_chain_only() {
    let dir = crate::audit::tests::TestDir::new("chain-launch");
    let config = chained_profiles(dir.path());
    let fallbacks = vec!["opus-api".to_owned(), "ds".to_owned()];
    let launch = |fallbacks: &[String]| -> (Vec<String>, BTreeMap<String, String>) {
        let launch = SeatLaunch {
            seat_bin: Path::new("/x/baylee-seat"),
            game_id: "game",
            seat: 1,
            gateway: "http://127.0.0.1:28766",
            profile: "cc",
            fallbacks,
            profiles_config: &config,
            ledger: Path::new("/x/ledger.json"),
            deck_path: Path::new("/x/deck.txt"),
            think_secs: 90,
            transcripts_dir: Path::new("/x/transcripts"),
            work_dir: dir.path(),
            send_reports: false,
        };
        let env = |name: &str| {
            ["PATH", "ANTHROPIC_API_KEY", "DEEPSEEK_API_KEY", "OTHER_KEY"]
                .contains(&name)
                .then(|| format!("value-of-{name}"))
        };
        let command = launch.command(&env).unwrap();
        let args = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let envs = command
            .as_std()
            .get_envs()
            .filter_map(|(name, value)| {
                Some((
                    name.to_string_lossy().into_owned(),
                    value?.to_string_lossy().into_owned(),
                ))
            })
            .collect();
        (args, envs)
    };
    let (args, envs) = launch(&fallbacks);
    let named: Vec<&str> = args
        .windows(2)
        .filter(|pair| pair[0] == "--fallback-profile")
        .map(|pair| pair[1].as_str())
        .collect();
    assert_eq!(named, ["opus-api", "ds"], "in the order they take over");
    assert!(!args.iter().any(|arg| arg.starts_with("--fallback-config")));
    for key in ["ANTHROPIC_API_KEY", "DEEPSEEK_API_KEY"] {
        assert_eq!(envs.get(key), Some(&format!("value-of-{key}")), "{key}");
    }
    assert!(!envs.contains_key("OTHER_KEY"), "no key of another profile");
    assert_eq!(
        chain_key_envs(&config, "cc", &fallbacks).unwrap(),
        ["ANTHROPIC_API_KEY", "DEEPSEEK_API_KEY"]
    );
    let (args, envs) = launch(&[]);
    assert!(!args.iter().any(|arg| arg == "--fallback-profile"));
    assert!(
        !envs.keys().any(|name| name.ends_with("KEY")),
        "a CLI alone reads no key: {envs:?}"
    );
}

/// A house seat (`house:<level>`) is started as the house at its level:
/// no profile, settings file, spend book or fallback, and no key at all,
/// whatever the environment holds; it still writes its transcripts. In
/// either chair it takes its own account's chair (`--host-chair`), never a
/// chair ticket, which seats only a language model.
#[test]
fn a_house_seat_is_started_at_its_level_with_no_profile_book_or_key() {
    let dir = crate::audit::tests::TestDir::new("house-launch");
    let config = chained_profiles(dir.path());
    let launch = |seat: usize, entry: &str| -> (Vec<String>, BTreeMap<String, String>) {
        let launch = SeatLaunch {
            seat_bin: Path::new("/x/baylee-seat"),
            game_id: "game",
            seat,
            gateway: "http://127.0.0.1:28766",
            profile: entry,
            fallbacks: &[],
            profiles_config: &config,
            ledger: Path::new("/x/ledger.json"),
            deck_path: Path::new("/x/deck.txt"),
            think_secs: 90,
            transcripts_dir: Path::new("/x/transcripts"),
            work_dir: dir.path(),
            send_reports: true,
        };
        let env = |name: &str| {
            [
                "PATH",
                "ANTHROPIC_API_KEY",
                "DEEPSEEK_API_KEY",
                "BAYLEE_LLM_API_KEY",
            ]
            .contains(&name)
            .then(|| format!("value-of-{name}"))
        };
        let command = launch.command(&env).unwrap();
        let args = command
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let envs = command
            .as_std()
            .get_envs()
            .filter_map(|(name, value)| {
                Some((
                    name.to_string_lossy().into_owned(),
                    value?.to_string_lossy().into_owned(),
                ))
            })
            .collect();
        (args, envs)
    };
    for (seat, entry, level, chair) in [
        (0, "house:sharp", "sharp", "--host-chair"),
        (1, "house:novice", "novice", "--host-chair"),
    ] {
        let (args, envs) = launch(seat, entry);
        let has = |flag: &str, value: &str| {
            args.windows(2)
                .any(|pair| pair[0] == flag && pair[1] == value)
        };
        assert!(has("--mind", "house"), "{args:?}");
        assert!(has("--level", level), "{args:?}");
        assert!(has("--transcripts", "/x/transcripts"), "{args:?}");
        assert!(has("--name", &format!("teacher-{seat}")), "{args:?}");
        assert!(args.iter().any(|arg| arg == chair), "{args:?}");
        assert!(!args.iter().any(|arg| arg == "--chair-ticket"), "{args:?}");
        assert_eq!(args.iter().filter(|arg| *arg == "--level").count(), 1);
        for refused in ["--profile", "--config", "--ledger", "--fallback-profile"] {
            assert!(
                !args.iter().any(|arg| arg == refused),
                "{refused}: {args:?}"
            );
        }
        assert!(
            !args
                .iter()
                .any(|arg| arg.contains("profiles.json") || arg == entry),
            "{args:?}"
        );
        assert_eq!(envs.get("PATH").map(String::as_str), Some("value-of-PATH"));
        assert!(
            !envs.keys().any(|name| name.ends_with("KEY")),
            "the house reads no key: {envs:?}"
        );
    }
}

/// Each slot account is called what plays its chair, as the bridge holds
/// a chair to (`Disclosure`) and as the gateway takes a name (3 to 16
/// letters, digits, `_` or `-`): the house's `HOUSE-…`, never `LLM-…`.
#[test]
fn a_slot_account_is_called_what_plays_its_chair() {
    use baylee_seat::Disclosure;
    for slot in [0, 7, 49, 999, 9999] {
        let (model, house) = (
            Sits::Model.display_name(slot),
            Sits::House.display_name(slot),
        );
        assert!(Disclosure::Llm.names(&model) && !Disclosure::House.names(&model));
        assert!(Disclosure::House.names(&house) && !Disclosure::Llm.names(&house));
        for name in [&model, &house] {
            assert!((3..=16).contains(&name.len()), "{name}");
            assert!(
                name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "{name}"
            );
        }
    }
    assert_eq!(Sits::House.display_name(3), "HOUSE-bench-3");
    assert_eq!(Sits::Model.display_name(3), "LLM-teacher-3");
}

/// A bridge's last stderr lines are kept, the last one why it stopped;
/// each is cut short, and one shaped like a key is never kept.
#[test]
fn a_bridge_s_last_stderr_lines_are_kept_without_a_key() {
    let mut tail = StderrTail::default();
    for n in 0..30 {
        tail.push(&format!("line {n}"));
    }
    tail.push(&"x".repeat(2000));
    tail.push("Authorization: Bearer abcdefghijklmnop0123456789");
    tail.push("Error: sit down on the host's chair ticket: 400");
    assert_eq!(tail.lines.len(), StderrTail::LINES);
    assert_eq!(tail.lines.front().map(String::as_str), Some("line 13"));
    assert_eq!(
        tail.lines.back().map(String::as_str),
        Some("Error: sit down on the host's chair ticket: 400")
    );
    assert!(
        tail.lines
            .iter()
            .all(|line| line.chars().count() <= StderrTail::CHARS)
    );
    assert!(
        !tail
            .lines
            .iter()
            .any(|line| line.contains("abcdefghijklmnop"))
    );
}

/// A house seat's pin is the house at its level with no chain behind it,
/// and its model `house:<level>`; the model it is paired with keeps its own.
#[test]
fn preopen_pins_hold_a_house_seat_to_the_house() {
    let dir = crate::audit::tests::TestDir::new("preopen-house");
    let config = chained_profiles(dir.path());
    let profiles = ["house:sharp".to_owned(), "cc".to_owned()];
    let models = ["house:sharp".to_owned(), "gpt-6.1-terra".to_owned()];
    let request = |chains: Value| {
        json!({"profiles_config_sha256": sha256_file(&config).unwrap(),
            "fallback_profiles": chains})
    };
    let chains = json!([[], ["opus-api", "ds"]]);
    let requested =
        validate_profile_pins(&config, &request(chains.clone()), &profiles, &models).unwrap();
    assert!(requested.chains[0].is_house() && !requested.chains[1].is_house());
    assert_eq!(
        serde_json::to_value(&requested.declarable()[0]).unwrap(),
        json!([{"kind": "house", "level": "sharp"}])
    );
    assert_eq!(requested.declarable()[1].len(), 3);
    // A fallback frozen for the house, or the house's model named otherwise.
    for (other, models) in [
        (json!([["ds"], ["opus-api", "ds"]]), models.clone()),
        (chains.clone(), ["sharp".to_owned(), models[1].clone()]),
        (chains, [String::new(), models[1].clone()]),
    ] {
        assert!(
            validate_profile_pins(&config, &request(other.clone()), &profiles, &models).is_err(),
            "{other} {models:?}"
        );
    }
}

/// Each seat's chain is frozen in the request: what the profiles file
/// says at room creation must be what the scheduler froze, and an older
/// request with no chain takes none.
#[test]
fn preopen_pins_hold_each_seat_to_its_frozen_chain() {
    let dir = crate::audit::tests::TestDir::new("preopen-chain");
    let config = chained_profiles(dir.path());
    let profiles = ["cc".to_owned(), "ds".to_owned()];
    let models = ["gpt-6.1-terra".to_owned(), "deepseek-flash".to_owned()];
    let request = |chains: Value| {
        let mut request = json!({"profiles_config_sha256": sha256_file(&config).unwrap()});
        if !chains.is_null() {
            request["fallback_profiles"] = chains;
        }
        request
    };
    let frozen = request(json!([["opus-api", "ds"], []]));
    let requested = validate_profile_pins(&config, &frozen, &profiles, &models).unwrap();
    assert_eq!(
        requested.fallback_profiles(),
        [vec!["opus-api".to_owned(), "ds".to_owned()], Vec::new()]
    );
    let minds = requested.fallback_minds();
    assert_eq!(
        serde_json::to_value(&minds).unwrap(),
        json!([[
            {"kind": "llm_api", "provider": "anthropic", "model": "claude-opus-5-5", "effort": "low"},
            {"kind": "llm_api", "provider": "openai", "model": "deepseek-flash"},
        ], []])
    );
    assert_eq!(requested.declarable()[0].len(), 3);
    assert_eq!(requested.declarable()[1].len(), 1);
    for other in [
        json!([["ds", "opus-api"], []]),
        json!([["opus-api"], []]),
        json!([[], []]),
        Value::Null,
    ] {
        assert!(
            validate_profile_pins(&config, &request(other.clone()), &profiles, &models).is_err(),
            "{other}"
        );
    }
}

#[test]
fn credential_destination_requires_literal_loopback_without_url_secrets() {
    for base in ["http://127.0.0.1:28768", "http://[::1]:28768"] {
        gateway_endpoint(base).unwrap();
    }
    for base in [
        "http://example.com:80",
        "https://127.0.0.1:80",
        "http://127.0.0.1:0",
        "http://127.0.0.1:80/path",
        "http://user:password@127.0.0.1:80",
    ] {
        assert!(gateway_endpoint(base).is_err());
    }
}

#[tokio::test]
async fn authenticated_gateway_requests_never_follow_redirects() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let target = format!("http://{}/unexpected", destination.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(socket);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
        }
        reader.get_mut().write_all(format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    });
    assert!(
        GatewayClient::new(&base, 0)
            .get_json("/test", Some("fixture-token"), 1)
            .await
            .is_err()
    );
    server.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), destination.accept())
            .await
            .is_err()
    );
}

#[test]
fn waiting_room_absence_requires_complete_empty_first_page() {
    let valid = json!({"offset":0,"limit":100,"total":0,"games":[]});
    assert!(complete_empty_lobby_page(&valid));
    for key in ["offset", "limit", "total", "games"] {
        let mut incomplete = valid.clone();
        incomplete.as_object_mut().unwrap().remove(key);
        assert!(!complete_empty_lobby_page(&incomplete));
    }
    for (key, value) in [
        ("offset", json!(100)),
        ("limit", json!(25)),
        ("total", json!(1)),
        ("total", json!("0")),
        ("games", json!([{"id":"owned-game"}])),
    ] {
        let mut page = valid.clone();
        page[key] = value;
        assert!(!complete_empty_lobby_page(&page));
    }
}

#[tokio::test]
async fn already_left_room_checks_listing_after_leave_refusal() {
    use tokio::io::AsyncReadExt as _;
    for (leave_status, listing_status, expected) in
        [(404, 200, true), (409, 200, true), (404, 403, false)]
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for (status, expected_request) in [
                (leave_status, "POST /lobby/games/owned-game/leave HTTP/1.1"),
                (
                    listing_status,
                    "GET /lobby/games?q=owned-game&offset=0&limit=100&waiting_only=false HTTP/1.1",
                ),
            ] {
                let (tcp, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(tcp);
                let mut first = String::new();
                reader.read_line(&mut first).await.unwrap();
                assert_eq!(first.trim_end(), expected_request);
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        content_length = value.trim().parse::<usize>().unwrap();
                    }
                }
                reader
                    .read_exact(&mut vec![0; content_length])
                    .await
                    .unwrap();
                let body = r#"{"offset":0,"limit":100,"total":0,"games":[]}"#;
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                reader
                    .get_mut()
                    .write_all(response.as_bytes())
                    .await
                    .unwrap();
            }
        });
        let client = GatewayClient::new(&format!("http://{addr}"), 0);
        assert_eq!(
            close_unstarted_room(&client, "test-session", "owned-game").await,
            expected
        );
        server.await.unwrap();
    }
}

#[tokio::test]
#[ignore = "requires explicit private gateway; creates one waiting room, no model or engine"]
async fn live_setup_only_configures_twenty_life_and_deletes_owned_room_and_account()
-> anyhow::Result<()> {
    let gateway = std::env::var("BAYLEE_COLLECTOR_SETUP_TEST_GATEWAY")
        .context("Set BAYLEE_COLLECTOR_SETUP_TEST_GATEWAY to an authorized private gateway")?;
    let authority = gateway
        .strip_prefix("http://")
        .context("Setup test requires loopback HTTP")?;
    crate::private_bind(authority)?;
    // The gateway's admin listener and database, which the teacher is
    // written into once the gateway proved them its own.
    let admin = crate::admin_cleanup::AdminCleanup::from_env(
        &std::env::var("BAYLEE_COLLECTOR_SETUP_TEST_ADMIN")
            .context("Set BAYLEE_COLLECTOR_SETUP_TEST_ADMIN to that gateway's admin listener")?,
    )?;
    let database = std::env::var("BAYLEE_COLLECTOR_SETUP_TEST_DATABASE")
        .context("Set BAYLEE_COLLECTOR_SETUP_TEST_DATABASE to that gateway's database")?;
    let own =
        crate::teacher::OwnDatabase::prove(&admin, baylee_db::open(&database, 2).await?).await?;
    let mut account = SlotAccount::init(&own, &gateway, 0, crate::teacher::Sits::Model).await?;
    let mut owned_room = None;
    let result: anyhow::Result<()> = async {
        let grant = account
            .client
            .post_json(
                "/lobby/games",
                &json!({
                    "deck_id":"", "mode":"open", "seats":2,
                    "name":"Collector setup-only verification", "allow_spectators":false,
                }),
                Some(&account.token),
                10,
            )
            .await?;
        let game = grant
            .get("game_id")
            .and_then(Value::as_str)
            .context("Missing owned setup-test room id")?
            .to_owned();
        owned_room = Some(game.clone());
        account.room_closed = false;
        let setup = collector_room_setup(["Constructed", "Constructed"])?;
        let listing = account
            .client
            .post_json(
                &format!("/lobby/games/{game}/configure"),
                &serde_json::to_value(room_update(0, &setup))?,
                Some(&account.token),
                10,
            )
            .await?;
        verify_configured_setup(&listing, &game, &setup)?;
        Ok(())
    }
    .await;
    let cleanup: anyhow::Result<()> = async {
        if let Some(game) = owned_room {
            account
                .client
                .post_json(
                    &format!("/lobby/games/{game}/leave"),
                    &json!({}),
                    Some(&account.token),
                    10,
                )
                .await?;
            let listing = account
                .client
                .get_json(
                    &format!("/lobby/games?q={game}&limit=100&waiting_only=false"),
                    Some(&account.token),
                    10,
                )
                .await?;
            ensure!(
                listing
                    .get("games")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty),
                "Owned setup-test room closure unconfirmed"
            );
            account.room_closed = true;
        }
        account.shutdown().await
    }
    .await;
    if cleanup.is_err() {
        account.presence.close().await;
    }
    cleanup.context("Setup-only test cleanup failed")?;
    result
}

#[test]
fn configure_request_roundtrips_and_refuses_gateway_default_setup() {
    let setup = collector_room_setup(["Constructed", "Constructed"]).unwrap();
    let request = serde_json::to_value(room_update(4, &setup)).unwrap();
    let body: baylee_core::preset::RoomUpdate = serde_json::from_value(request.clone()).unwrap();
    assert_eq!(body.chairs, 2);
    assert_eq!(
        (body.setup.starting_life, body.setup.free_mulligans),
        (20, 0)
    );
    let confirmed = json!({"games":[{"id":"game", "setup":request["setup"]}]});
    assert!(verify_configured_setup(&confirmed, "game", &setup).is_ok());
    let ignored =
        json!({"games":[{"id":"game", "setup":baylee_core::preset::RoomSetup::default()}]});
    assert!(verify_configured_setup(&ignored, "game", &setup).is_err());
    assert!(verify_configured_setup(&json!({"games":[]}), "game", &setup).is_err());
}

/// The models' reports of this game are counted from the shared
/// reports file, and each seat's bug report read from its transcript.
#[test]
fn the_models_reports_are_counted_per_game() {
    let dir = crate::audit::tests::TestDir::new("llm-reports");
    let transcripts = dir.path();
    let (none, unsent) = llm_reports(transcripts, "game");
    assert_eq!((none, unsent), (0, vec![Value::Null, Value::Null]));
    let lines = [
        json!({"game_id": "game", "seat": 0, "what": "a"}),
        json!({"game_id": "other", "seat": 1, "what": "b"}),
        json!({"game_id": "game", "seat": 1, "what": "c"}),
    ];
    let text: Vec<String> = lines.iter().map(ToString::to_string).collect();
    std::fs::write(
        transcripts.join(baylee_seat::reports::FILE),
        text.join("\n") + "\n",
    )
    .unwrap();
    let filing = json!({"count": 1, "sent": true, "report_id": "r", "why": null});
    std::fs::write(
        transcripts.join("game-seat0.jsonl"),
        format!(
            "{}\n{}\n",
            json!({"event": "over"}),
            json!({"llm_reports": filing})
        ),
    )
    .unwrap();
    let (counted, sent) = llm_reports(transcripts, "game");
    assert_eq!(counted, 2);
    assert_eq!(sent, vec![filing, Value::Null]);
}

#[test]
fn exited_seats_without_gameover_fail_before_absent_room_can_continue() {
    assert!(!owned_seats_finished(true, false, false).unwrap());
    assert!(owned_seats_finished(true, true, true).unwrap());
    assert!(owned_seats_finished(true, true, false).is_err());
    assert!(owned_seats_finished(false, true, true).is_err());
}

#[test]
fn official_end_proves_closure_without_gameover_but_not_admission() {
    let dir = crate::audit::tests::TestDir::new("end-without-over");
    let out = dir.path();
    let record = out.join("official.jsonl");
    let expected = crate::audit::tests::codex_identity();
    crate::audit::tests::write_identity_record(&record, &expected);
    assert!(!both_transcripts_over(out, "game"));
    confirm_terminal_closure(out, &record, "game", true).unwrap();
    let room: Value =
        serde_json::from_slice(&std::fs::read(out.join("room-state.json")).unwrap()).unwrap();
    assert_eq!(room["closure_verified"], true);
    let proof = audit_record(&record, &[vec![expected.clone()], vec![expected]]).unwrap();
    assert!(audit_transcript(&out.join("missing-seat.jsonl"), &proof.inputs[0], false).is_err());
    assert!(!out.join("technical-result.json").exists());
    assert!(!out.join("receipt.json").exists());
}

#[test]
fn absent_terminal_end_cannot_mark_unknown_room_closed() {
    let dir = crate::audit::tests::TestDir::new("missing-end");
    let out = dir.path();
    let record = out.join("incomplete.jsonl");
    write_atomic_json(
        &out.join("room-state.json"),
        &json!({"status":"unknown", "closure_verified":false}),
    )
    .unwrap();
    std::fs::write(&record, "{\"kind\":\"header\"}\n").unwrap();
    assert!(confirm_terminal_closure(out, &record, "game", true).is_err());
    let room: Value =
        serde_json::from_slice(&std::fs::read(out.join("room-state.json")).unwrap()).unwrap();
    assert_eq!(room["status"], "unknown");
    assert_eq!(room["closure_verified"], false);
}

#[test]
fn completed_invalid_identity_proves_closure_but_never_creates_admission() {
    let dir = crate::audit::tests::TestDir::new("closed-not-admitted");
    let out = dir.path();
    let record = out.join("record.jsonl");
    let expected = crate::audit::tests::codex_identity();
    let mut spoof = expected.clone();
    spoof.provider = "agy".into();
    crate::audit::tests::write_identity_record(&record, &spoof);
    std::fs::create_dir(out.join("transcripts")).unwrap();
    for seat in 0..2 {
        std::fs::write(
            out.join(format!("transcripts/game-seat{seat}.jsonl")),
            "{\"event\":\"over\"}\n",
        )
        .unwrap();
    }
    confirm_terminal_closure(out, &record, "game", true).unwrap();
    let room: Value =
        serde_json::from_slice(&std::fs::read(out.join("room-state.json")).unwrap()).unwrap();
    assert_eq!(room["status"], "closed");
    assert_eq!(room["closure_verified"], true);
    assert!(audit_record(&record, &[vec![expected.clone()], vec![expected]]).is_err());
    assert!(!out.join("technical-result.json").exists());
    assert!(!out.join("receipt.json").exists());
    // A correctly declared completed game still fails admission if its
    // accepted input has no matching ordered transcript send.
    let expected = crate::audit::tests::codex_identity();
    crate::audit::tests::write_identity_record(&record, &expected);
    let proof = audit_record(&record, &[vec![expected.clone()], vec![expected]]).unwrap();
    assert!(
        ordered_attribution(
            &out.join("transcripts/game-seat0.jsonl"),
            &proof.inputs[0],
            false
        )
        .is_err()
    );
    assert!(!out.join("technical-result.json").exists());
    std::fs::write(&record, "{\"kind\":\"header\"}\n").unwrap();
    assert!(confirm_terminal_closure(out, &record, "game", true).is_err());
}

#[test]
fn preopen_pins_reject_changes_and_model_mismatch() {
    let dir = crate::audit::tests::TestDir::new("preopen-pins");
    let config = dir.path().join("profiles.json");
    let pairings = dir.path().join("pairings.json");
    write_atomic_json(
        &config,
        &json!({"profiles": {
            "terra": {"provider":"cli", "model":"codex:gpt-6.1-terra", "effort":"low", "game_tokens":100_000}
        }}),
    )
    .unwrap();
    write_atomic_json(&pairings, &json!([["terra", "terra"]])).unwrap();
    let profiles = ["terra".to_owned(), "terra".to_owned()];
    let models = ["gpt-6.1-terra".to_owned(), "gpt-6.1-terra".to_owned()];
    let request = json!({
        "profiles_config_sha256":sha256_file(&config).unwrap(),
        "pairings_file":pairings, "pairings_sha256":sha256_file(&pairings).unwrap(),
    });
    let requested = validate_profile_pins(&config, &request, &profiles, &models).unwrap();
    let expected_minds = requested.minds();
    assert_eq!(expected_minds[0], crate::audit::tests::codex_identity());
    assert_eq!(expected_minds[1], crate::audit::tests::codex_identity());
    assert_eq!(
        requested.fallback_profiles(),
        [Vec::<String>::new(), Vec::new()]
    );
    let wrong = ["wrong-model".to_owned(), models[1].clone()];
    assert!(validate_profile_pins(&config, &request, &profiles, &wrong).is_err());
    std::fs::write(&pairings, "[]").unwrap();
    assert!(validate_profile_pins(&config, &request, &profiles, &models).is_err());
    write_atomic_json(&pairings, &json!([["terra", "terra"]])).unwrap();
    std::fs::write(&config, "{}").unwrap();
    assert!(validate_profile_pins(&config, &request, &profiles, &models).is_err());
}

#[test]
fn account_retry_requires_proven_cleanup() {
    assert!(!account_init_retry_safe(&anyhow!(
        "ambiguous login failure"
    )));
    assert!(!account_init_retry_safe(&anyhow!(
        "account deletion failed"
    )));
    let cleaned = anyhow::Error::new(CleanAccountInitFailure(anyhow!("presence failed")))
        .context("slot initialization");
    assert!(account_init_retry_safe(&cleaned));
}

#[test]
fn ephemeral_username_satisfies_gateway_contract() {
    // Slot IDs never enter the credential name, including large slot IDs.
    for _ in 0..128 {
        let name = account_username(uuid::Uuid::now_v7());
        assert_eq!(name.len(), baylee_protocol::names::USERNAME_MAX);
        assert!(baylee_protocol::names::username(&name).is_ok());
    }
}

#[tokio::test]
async fn tether_stays_open_until_explicit_cleanup() {
    let child = Command::new("/bin/sh")
        .args(["-c", "read first; read second"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap();
    let pgid = i32::try_from(child.id().unwrap()).unwrap();
    let mut guard = SeatProcessGroup {
        child,
        stdin: None,
        pgid,
        diag_task: None,
    };
    guard.stdin = guard.child.stdin.take();
    guard.deliver_secret("placeholder").await.unwrap();
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        guard.try_wait().unwrap().is_none(),
        "tether EOF must not end a seated bridge"
    );
    guard.terminate().await.unwrap();
    assert!(!guard.group_alive());
}

#[tokio::test]
async fn owned_child_is_killed_and_reaped_before_cleanup_succeeds() {
    let child = Command::new("/bin/sleep")
        .arg("60")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap();
    let pgid = i32::try_from(child.id().unwrap()).unwrap();
    let mut guard = SeatProcessGroup {
        child,
        stdin: None,
        pgid,
        diag_task: None,
    };
    guard.terminate().await.unwrap();
    assert!(!guard.group_alive());
    assert!(guard.try_wait().unwrap().is_some());
}

#[tokio::test]
async fn launch_pacer_spaces_concurrent_callers() {
    let pacer = Arc::new(LaunchPacer::new(Duration::from_millis(40)));
    let t0 = Instant::now();
    let mut handles = Vec::new();
    for _ in 0..3 {
        let p = Arc::clone(&pacer);
        handles.push(tokio::spawn(async move {
            p.wait_turn().await;
            Instant::now()
        }));
    }
    let mut stamps = Vec::new();
    for h in handles {
        stamps.push(h.await.unwrap());
    }
    stamps.sort();
    assert!(stamps[2].duration_since(t0) >= Duration::from_millis(75));
}

#[tokio::test]
async fn presence_answers_websocket_pings_and_closes_cleanly() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
        ws.send(Message::Ping(vec![9, 8, 7].into())).await.unwrap();
        let reply = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(reply, Message::Pong(vec![9, 8, 7].into()));
    });
    let mut presence = Presence::connect_with_ticket(&format!("http://{addr}"), "test-ticket")
        .await
        .unwrap();
    assert!(presence.check().is_ok());
    server.await.unwrap();
    presence.close().await;
}
