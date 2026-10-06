use super::*;

fn opus_profile() -> Profile {
    Profile {
        effort: Some("high".into()),
        ..Profile::new(Provider::Anthropic, "claude-opus-5-5")
    }
}

fn launch() -> Launch<'static> {
    Launch {
        room: "TEST-room",
        gateway: "http://127.0.0.1:28766",
        chair: 2,
        config: "/tmp/TEST/llm-seat.json",
        deck: "Allytifact",
        level: "steady",
    }
}

#[test]
fn a_chair_keeps_an_effort_only_where_its_model_takes_it() {
    let profile = opus_profile();
    let mut chair = ChairModel::of("opus", &profile);
    assert_eq!(chair.effort.as_deref(), Some("high"));
    let sonnet46 = resolved(&profile, None, "claude-sonnet-4-6", &[]);
    assert!(chair.choose_effort(&resolved(&profile, None, &chair.model, &[]), Some("xhigh")));
    // Sonnet 4.6 takes no xhigh: the effort falls back to the model's own.
    chair.choose_model(&sonnet46);
    assert_eq!(chair.model, "claude-sonnet-4-6");
    assert_eq!(chair.effort, None);
    assert!(!chair.choose_effort(&sonnet46, Some("xhigh")), "refused");
    assert_eq!(chair.effort, None, "and nothing changed");
    assert!(chair.choose_effort(&sonnet46, Some("max")));
    // A model this build does not know takes no named effort.
    let typed = resolved(&profile, None, "claude-next-9", &[]);
    chair.choose_model(&typed);
    assert_eq!(chair.effort, None);
    assert!(!chair.choose_effort(&typed, Some("low")));
    assert!(chair.choose_effort(&typed, None));
    // Saved, the profile is the chair's choice.
    let mut saved = profile.clone();
    chair.save_into(&mut saved);
    assert_eq!(saved.model, "claude-next-9");
    assert_eq!(saved.effort, None);
    assert_eq!(saved.provider, profile.provider);
}

#[test]
fn a_chair_is_offered_its_profiles_models_and_what_it_plays() {
    let lm = Preset::LmStudio.profile();
    let listed = vec!["qwen3-8b".to_string()];
    let ids: Vec<String> = models_for(&lm, None, &listed)
        .into_iter()
        .map(|m| m.id)
        .collect();
    // The listed model, and the profile's own placeholder after it.
    assert_eq!(ids, ["qwen3-8b", "local-model"]);
    let claude = Preset::ClaudeCode.profile();
    let offered = models_for(&claude, None, &[]);
    assert!(offered.iter().any(|m| m.id == "claude:sonnet"));
    assert!(offered.iter().all(|m| m.id.starts_with("claude")));
    // The environment's address counts for a profile that names none.
    let bare = Profile::new(Provider::OpenAi, "x-1");
    let here = models_for(&bare, Some("http://localhost:8080/v1"), &listed);
    assert!(here.iter().any(|m| m.id == "qwen3-8b"));
    assert!(
        models_for(&bare, None, &listed)
            .iter()
            .all(|m| m.id != "qwen3-8b"),
        "api.openai.com is never listed"
    );
}

#[test]
fn the_bridge_is_told_the_chair_the_file_and_only_what_the_profile_does_not_say() {
    let profile = opus_profile();
    let chair = ChairModel::of("opus", &profile);
    let args = bridge_args(&launch(), &chair, &profile);
    assert_eq!(
        args,
        [
            "join",
            "TEST-room",
            "--gateway",
            "http://127.0.0.1:28766",
            "--chair",
            "2",
            "--config",
            "/tmp/TEST/llm-seat.json",
            "--profile",
            "opus",
            "--acceptance",
            "Allytifact",
            "--level",
            "steady",
            "--tethered",
        ]
    );
    let changed = ChairModel {
        model: "claude-sonnet-5-5".into(),
        effort: Some("low".into()),
        ..chair.clone()
    };
    let args = bridge_args(&launch(), &changed, &profile);
    let tail: Vec<&str> = args.iter().map(String::as_str).skip(15).collect();
    assert_eq!(
        tail,
        ["--mind", "anthropic:claude-sonnet-5-5", "--effort", "low"]
    );
    // A CLI's model is the tool's, after `cli:`.
    let cli = Preset::ClaudeCode.profile();
    let mut on_cli = ChairModel::of("claude-code", &cli);
    on_cli.model = "claude:sonnet".into();
    let args = bridge_args(&launch(), &on_cli, &cli);
    assert!(
        args.windows(2)
            .any(|w| w == ["--mind", "cli:claude:sonnet"])
    );
    // The default effort over a profile that names one says so: the
    // profile's would stand otherwise.
    let own = ChairModel {
        effort: None,
        ..chair
    };
    let args = bridge_args(&launch(), &own, &profile);
    assert!(!args.contains(&"--effort".to_string()));
    assert_eq!(args.last().map(String::as_str), Some("--default-effort"));
    let bare = Profile::new(Provider::Anthropic, "claude-opus-5-5");
    let args = bridge_args(&launch(), &ChairModel::of("bare", &bare), &bare);
    assert!(
        !args.contains(&"--default-effort".to_string()),
        "nothing to drop"
    );
}

#[test]
fn an_order_reads_back_and_refuses_what_is_not_one() {
    for order in [
        Order::House {
            level: "sharp".into(),
        },
        Order::Model {
            profile: "claude-code".into(),
            model: "claude:opus".into(),
            effort: Some("max".into()),
        },
        Order::Model {
            profile: "lm".into(),
            model: "qwen3-8b".into(),
            effort: None,
        },
    ] {
        let line = order.line();
        assert!(line.ends_with('\n') && line.matches('\n').count() == 1);
        assert_eq!(Order::parse(&line), Ok(order));
    }
    assert_eq!(
        Order::parse(r#"{"mind":"model","profile":"p","model":"claude:opus"}"#)
            .unwrap()
            .mind(Provider::Cli)
            .as_deref(),
        Some("cli:claude:opus")
    );
    let key = format!(
        r#"{{"mind":"house","level":"sk-ant-api03-{}"}}"#,
        "A".repeat(30)
    );
    let refused = Order::parse(&key).unwrap_err();
    assert!(!refused.contains("AAAA"), "{refused}");
    for bad in [
        r#"{"mind":"house"}"#,
        r#"{"mind":"house","level":"Two Words"}"#,
        r#"{"mind":"model","profile":"-p","model":"m"}"#,
        r#"{"mind":"model","profile":"p","model":"two words"}"#,
        r#"{"mind":"model","profile":"p","model":"m","effort":"HIGH"}"#,
        r#"{"mind":"model","profile":"p","model":"m","key":"x"}"#,
        r#"{"mind":"net"}"#,
        "not json",
    ] {
        assert!(Order::parse(bad).is_err(), "{bad}");
    }
    assert!(Order::parse(&" ".repeat(ORDER_BYTES + 1)).is_err());
}

/// Before the game a change starts the chair's bridge again; during it,
/// only a debug build tells the running bridge, and a release build
/// refuses. Both branches are asked here, whichever build runs the test.
#[test]
fn a_chair_changes_during_the_game_only_in_a_debug_build() {
    let profile = opus_profile();
    let mut seating = Seating::default();
    seating.enter("TEST-room");
    let opus = ChairModel::of("opus", &profile);
    assert_eq!(seating.plan(1, opus.clone(), None), Change::Reseat);
    assert_eq!(seating.chair(1).unwrap().version, 0);
    assert_eq!(seating.chair(1).unwrap().deck, DECKS[0]);
    let sonnet = ChairModel {
        model: "claude-sonnet-5-5".into(),
        ..opus.clone()
    };
    assert_eq!(
        seating.change(1, Some(sonnet.clone()), "steady", Phase::Waiting, false),
        Ok(Change::Reseat)
    );
    assert_eq!(seating.chair(1).unwrap().version, 1);
    // During the game: a release build refuses and changes nothing.
    assert_eq!(
        seating.change(1, Some(opus.clone()), "steady", Phase::Playing, false),
        Err(Refusal::NotLive)
    );
    assert_eq!(seating.chair(1).unwrap().model, sonnet);
    // A debug build tells the bridge, and the plan shows it, at the same
    // version: nothing is started again.
    assert_eq!(
        seating.change(1, Some(opus.clone()), "steady", Phase::Playing, true),
        Ok(Change::Order(Order::model(&opus)))
    );
    assert_eq!(seating.chair(1).unwrap().model, opus);
    assert_eq!(seating.chair(1).unwrap().version, 1);
    assert_eq!(
        seating.change(1, None, "sharp", Phase::Playing, true),
        Ok(Change::Order(Order::House {
            level: "sharp".into()
        }))
    );
    // A chair this client seats no model in is not its to change.
    assert_eq!(
        seating.change(3, Some(opus), "steady", Phase::Playing, true),
        Err(Refusal::NotOurs)
    );
    assert_eq!(LIVE_CHANGES, cfg!(debug_assertions));
}

#[test]
fn bridges_start_once_their_chair_is_open_and_stop_when_their_plan_moves_on() {
    let profile = opus_profile();
    let mut seating = Seating::default();
    let waiting = Phase::Waiting;
    assert!(seating.steps(waiting, &[1], &[]).is_empty(), "no room");
    seating.enter("TEST-room");
    seating.plan(1, ChairModel::of("opus", &profile), Some("Weltenbaum"));
    // Not open yet (the gateway still lists the house there): wait.
    assert!(seating.steps(waiting, &[], &[]).is_empty());
    assert_eq!(seating.steps(waiting, &[1], &[]), [Step::Launch(1)]);
    let started = Running {
        chair: 1,
        version: 0,
        exited: false,
    };
    assert!(seating.steps(waiting, &[], &[started]).is_empty());
    // A bridge that exited (refused) is not started again for that plan.
    let refused = Running {
        exited: true,
        ..started
    };
    assert!(seating.steps(waiting, &[1], &[refused]).is_empty());
    // A new plan stops the old bridge first, and starts the new one only
    // once the old one has left the chair.
    seating.plan(1, ChairModel::of("opus", &profile), None);
    assert_eq!(seating.chair(1).unwrap().deck, "Weltenbaum", "kept");
    assert_eq!(seating.steps(waiting, &[], &[started]), [Step::Stop(1)]);
    assert_eq!(seating.steps(waiting, &[1], &[]), [Step::Launch(1)]);
    // During the game nothing is started or stopped, even for a moved plan.
    assert!(seating.steps(Phase::Playing, &[1], &[started]).is_empty());
    // Unplanned, or the room left: every bridge stops.
    seating.unplan(1);
    assert_eq!(seating.steps(waiting, &[1], &[started]), [Step::Stop(1)]);
    seating.plan(1, ChairModel::of("opus", &profile), None);
    seating.leave();
    assert_eq!(
        seating.steps(Phase::Playing, &[], &[started]),
        [Step::Stop(1)]
    );
    // Another room forgets the last one's chairs.
    seating.enter("TEST-room");
    seating.plan(2, ChairModel::of("opus", &profile), None);
    seating.enter("TEST-other");
    assert!(seating.chair(2).is_none());
}

#[test]
fn a_preset_makes_a_profile_the_file_takes_under_a_free_name() {
    let mut settings = SeatSettings::default();
    for preset in Preset::ALL {
        let name = preset.add_to(&mut settings);
        assert_eq!(name, preset.name());
        assert!(profile_name_is_a_name(&name));
    }
    assert_eq!(Preset::Anthropic.add_to(&mut settings), "anthropic-2");
    assert_eq!(settings.check(), Ok(()), "{:?}", settings.faults());
    // And the bridge would sit down with each: a model with no price states
    // a token budget, and no key is in any of them.
    for (name, profile) in &settings.profiles {
        assert!(
            profile.price().is_some()
                || profile.game_tokens.is_some()
                || profile.provider == Provider::Cli,
            "{name}"
        );
    }
    assert!(!settings.to_json().contains("sk-"));
}

#[test]
fn an_adapter_is_a_protocol_and_an_address_the_player_may_change() {
    let deepseek = Preset::DeepSeek.profile();
    let over_anthropic = Preset::DeepSeekAnthropic.profile();
    assert_eq!(deepseek.provider, Provider::OpenAi);
    assert_eq!(over_anthropic.provider, Provider::Anthropic);
    assert_eq!(
        protocol_label(over_anthropic.provider),
        "Anthropic Messages"
    );
    // One vendor, one key, whichever protocol reaches it.
    assert_eq!(deepseek.key_env(), over_anthropic.key_env());
    assert_ne!(
        Preset::OpenAi.profile().key_env(),
        Preset::DeepSeek.profile().key_env()
    );
    let mut edited = deepseek.clone();
    assert_eq!(
        set_address(&mut edited, Some(" https://llm.example.org/v1/ ")),
        Ok(())
    );
    assert_eq!(
        edited.base_url.as_deref(),
        Some("https://llm.example.org/v1")
    );
    for refused in [
        "http://llm.example.org/v1",
        "https://llm.example.org/sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        assert!(
            set_address(&mut edited, Some(refused)).is_err(),
            "{refused}"
        );
    }
    assert_eq!(
        edited.base_url.as_deref(),
        Some("https://llm.example.org/v1")
    );
    assert_eq!(
        set_address(&mut edited, Some("http://localhost:8080/v1")),
        Ok(())
    );
    assert_eq!(set_address(&mut edited, None), Ok(()));
    assert_eq!(edited.base_url, None);
    let mut cli = Preset::ClaudeCode.profile();
    assert!(set_address(&mut cli, Some("https://api.anthropic.com")).is_err());
}
