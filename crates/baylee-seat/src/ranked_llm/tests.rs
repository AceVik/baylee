use super::*;
use baylee_client_core::llmseat::keys::{KeyEntry, MemoryKeys};
use baylee_protocol::v1::seat_mind;
use baylee_seat::bridge::{Limit, Standby};
use baylee_seat::ranked::chain::STANDARD;
use std::sync::Mutex;

/// A directory of this test's own, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("baylee-ranked-llm-{name}-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn mind(kind: seat_mind::Kind, provider: &str, model: &str, effort: &str) -> SeatMind {
    SeatMind {
        kind: kind as i32,
        provider: provider.into(),
        model: model.into(),
        effort: effort.into(),
        level: String::new(),
    }
}

/// What a mind claims, where the build knows its efforts.
fn claim(kind: seat_mind::Kind, tool: &str, model: &str, named: Option<&str>) -> Claimed {
    claimed(&mind(kind, tool, model, ""), named, true).unwrap()
}

#[test]
fn every_dialect_s_id_for_one_model_at_one_effort_is_the_same_claim() {
    use seat_mind::Kind::{LlmApi, LlmCli};
    // Sonnet 5.5 at low: Claude Code's pinned id, agy's tiered id, the
    // API's id.
    let claude = claim(LlmCli, "claude", "claude-sonnet-5-5", Some("low"));
    let agy = claim(LlmCli, "agy", "claude-sonnet-5-5-low", None);
    let api = claim(LlmApi, "anthropic", "claude-sonnet-5-5", Some("low"));
    assert_eq!(agy.model, "claude-sonnet-5-5", "agy's tier is its effort");
    assert_eq!(agy.effort, "low");
    for (a, b) in [
        (&claude, &agy),
        (&claude, &api),
        (&agy, &api),
        (&api, &claude),
    ] {
        assert_eq!(same_model(a, b), Ok(()), "{a:?} {b:?}");
    }
    // A named effort is what agy plays, whatever tier its id names.
    let named = claim(LlmCli, "agy", "claude-sonnet-5-5-low", Some("high"));
    assert_eq!(named.effort, "high");
    assert!(same_model(&claude, &named).is_err(), "high is not low");
    // Gemini on agy and behind an API; a GPT on Codex and behind an API.
    assert_eq!(
        same_model(
            &claim(LlmCli, "agy", "gemini-3.8-flash-high", None),
            &claim(LlmApi, "openai", "gemini-3.8-flash", Some("high"))
        ),
        Ok(())
    );
    assert_eq!(
        same_model(
            &claim(LlmCli, "codex", "gpt-6-astra", Some("high")),
            &claim(LlmApi, "openai", "gpt-6-astra", Some("high"))
        ),
        Ok(())
    );
    // Another model, or another family under a shared word, is not one.
    for other in [
        claim(LlmApi, "anthropic", "claude-opus-5-5", Some("low")),
        claim(LlmCli, "agy", "gemini-4-argon-low", None),
        claim(LlmCli, "codex", "sonnet", Some("low")),
    ] {
        assert!(same_model(&claude, &other).is_err(), "{other:?}");
    }
}

#[test]
fn a_model_names_its_effort_where_the_build_knows_them_and_plays_standard_where_not() {
    use seat_mind::Kind::{LlmApi, LlmCli};
    let deepseek = mind(LlmApi, "openai", "deepseek-flash", "");
    let standard = claimed(&deepseek, None, false).unwrap();
    assert_eq!(standard.effort, STANDARD);
    assert_eq!(standard.family.as_deref(), Some("deepseek"));
    let why = claimed(&deepseek, Some("high"), false).unwrap_err();
    assert!(
        why.contains("knows no efforts") && why.contains("leave `effort` out"),
        "{why}"
    );
    let sonnet = mind(LlmApi, "anthropic", "claude-sonnet-5-5", "medium");
    let why = claimed(&sonnet, None, true).unwrap_err();
    assert!(why.contains("names no effort"), "{why}");
    let why = claimed(&mind(LlmCli, "claude", "", ""), Some("high"), true).unwrap_err();
    assert!(why.contains("names no model"), "{why}");
    let house = SeatMind {
        kind: seat_mind::Kind::House as i32,
        ..sonnet
    };
    assert!(claimed(&house, Some("low"), true).is_err());
}

/// A settings file: Sonnet 5.5 at low on Claude Code, on agy (its tier in
/// its id) and behind the API; a chain that crosses to Opus, one that
/// crosses efforts; `DeepSeek`, whose efforts this build does not know, on
/// two keys; and profiles that name too little or too much.
fn settings(dir: &std::path::Path) -> PathBuf {
    let program = std::env::current_exe().unwrap();
    let path = dir.join("llm-seat.json");
    let deepseek = |effort: Option<&str>| {
        let mut profile = serde_json::json!({
            "provider": "openai", "model": "deepseek-flash",
            "base_url": "https://api.deepseek.com", "key_env": "TEST_DS_KEY"
        });
        if let Some(effort) = effort {
            profile["effort"] = effort.into();
        }
        profile
    };
    let mut flash = deepseek(None);
    flash["fallback"] = serde_json::json!(["deepseek-spare"]);
    let file = serde_json::json!({
        "profiles": {
            "sonnet-low": {
                "provider": "cli", "model": "claude:claude-sonnet-5-5", "effort": "low",
                "command": program, "fallback": ["sonnet-agy", "sonnet-api"]
            },
            "sonnet-agy": {
                "provider": "cli", "model": "agy:claude-sonnet-5-5-low", "command": program
            },
            "sonnet-api": {
                "provider": "anthropic", "model": "claude-sonnet-5-5", "effort": "low",
                "key_env": "TEST_RANKED_KEY"
            },
            "crossing": {
                "provider": "cli", "model": "claude:claude-sonnet-5-5", "effort": "low",
                "command": program, "fallback": ["opus-api"]
            },
            "opus-api": {
                "provider": "anthropic", "model": "claude-opus-5-5", "effort": "low",
                "key_env": "TEST_RANKED_KEY"
            },
            "tiers": {
                "provider": "cli", "model": "claude:claude-sonnet-5-5", "effort": "low",
                "command": program, "fallback": ["agy-high"]
            },
            "agy-high": {
                "provider": "cli", "model": "agy:claude-sonnet-5-5-high", "command": program
            },
            "deepseek-flash": flash,
            "deepseek-spare": deepseek(None),
            "deepseek-high": deepseek(Some("high")),
            "no-model": {
                "provider": "cli", "model": "claude", "effort": "high", "command": program
            },
            "no-effort": {
                "provider": "anthropic", "model": "claude-sonnet-5-5",
                "key_env": "TEST_RANKED_KEY"
            },
            "Upper": {
                "provider": "cli", "model": "claude:claude-sonnet-5-5", "effort": "low",
                "command": program
            },
            "alias": {
                "provider": "cli", "model": "claude:sonnet", "effort": "low",
                "command": program
            }
        }
    });
    std::fs::write(&path, file.to_string()).unwrap();
    path
}

fn keys() -> Arc<dyn KeyStore> {
    let keys = MemoryKeys::default();
    for (name, host) in [
        ("TEST_RANKED_KEY", "https://api.anthropic.com"),
        ("TEST_DS_KEY", "https://api.deepseek.com"),
    ] {
        keys.set(
            &KeyEntry::new(name, host).unwrap(),
            "TEST-ranked-0123456789abcdef",
        )
        .unwrap();
    }
    Arc::new(keys)
}

fn setup(dir: &Scratch, profile: &str) -> Setup {
    Setup {
        profile: profile.into(),
        config: Some(settings(&dir.0)),
        ledger: Some(dir.0.join("llm-spend.json")),
        transcripts: Some(dir.0.join("transcripts")),
        house_level: "expert".into(),
    }
}

#[test]
fn a_model_account_flies_its_chain_and_refuses_one_that_crosses_models() {
    let dir = Scratch::new("chain");
    let pilot = LlmPilot::new(&setup(&dir, "sonnet-low"), keys()).unwrap();
    assert_eq!(pilot.kind(), AiKind::Model);
    assert_eq!(pilot.profile(), "model:sonnet-low");
    let names: Vec<&str> = pilot.paths.iter().map(|p| p.profile.as_str()).collect();
    assert_eq!(names, ["sonnet-low", "sonnet-agy", "sonnet-api"]);
    assert_eq!(pilot.declared().model, "claude-sonnet-5-5");
    assert_eq!(pilot.declared().effort, "low");
    // No per-game budget named: none held, by the owner's decision.
    for path in &pilot.paths {
        assert_eq!(path.join.spend_tokens, Some(UNLIMITED_TOKENS));
        assert_eq!(path.join.spend_calls, Some(UNLIMITED_CALLS));
    }
    for (profile, why) in [
        ("crossing", "another model"),
        ("tiers", "effort «high» is not «low»"),
        ("Upper", "model:<profile>"),
        // A moving alias: the pinned id to name instead.
        ("alias", "`claude:claude-sonnet-5-5`"),
    ] {
        let refused = format!(
            "{:#}",
            LlmPilot::new(&setup(&dir, profile), keys()).unwrap_err()
        );
        assert!(refused.contains(why), "{profile}: {refused}");
    }
    // A path whose key is not kept yet cannot be reached: the account still
    // starts, that path held back as a failing check is, and retried.
    let pilot = LlmPilot::new(&setup(&dir, "sonnet-low"), Arc::new(MemoryKeys::default())).unwrap();
    let live: Vec<&str> = pilot
        .live(Instant::now())
        .iter()
        .map(|p| p.profile.as_str())
        .collect();
    assert_eq!(live, ["sonnet-low", "sonnet-agy"]);
    assert_eq!(pilot.ready(), Ok(()));
    assert!(pilot.held.lock().unwrap().contains_key("sonnet-api"));
    // With no path reachable at all, it starts and offers nothing.
    let pilot = LlmPilot::new(&setup(&dir, "sonnet-api"), Arc::new(MemoryKeys::default())).unwrap();
    assert!(pilot.ready().is_err());
}

/// `DeepSeek`'s efforts this build does not know: it plays at its Standard,
/// on every path of its chain, and a profile that names one is refused.
#[test]
fn a_model_whose_efforts_the_build_does_not_know_plays_at_its_standard() {
    let dir = Scratch::new("standard");
    let pilot = LlmPilot::new(&setup(&dir, "deepseek-flash"), keys()).unwrap();
    assert_eq!(pilot.paths.len(), 2);
    assert!(pilot.paths.iter().all(|p| p.claim.effort == STANDARD));
    assert_eq!(pilot.profile(), "model:deepseek-flash");
    let why = format!(
        "{:#}",
        LlmPilot::new(&setup(&dir, "deepseek-high"), keys()).unwrap_err()
    );
    assert!(why.contains("knows no efforts"), "{why}");
}

#[test]
fn a_held_back_path_sits_out_new_games_and_all_held_makes_no_offer() {
    let dir = Scratch::new("held");
    let pilot = LlmPilot::new(&setup(&dir, "sonnet-low"), keys()).unwrap();
    let now = Instant::now();
    assert_eq!(pilot.ready(), Ok(()));
    assert_eq!(pilot.live(now).len(), 3);
    // The first subscription is rate limited: the next game starts on the
    // second, the API behind it.
    pilot.hold("sonnet-low", Duration::from_mins(5), "rate limited");
    let live: Vec<&str> = pilot
        .live(Instant::now())
        .iter()
        .map(|p| p.profile.as_str())
        .collect();
    assert_eq!(live, ["sonnet-agy", "sonnet-api"]);
    pilot.hold("sonnet-agy", Duration::from_mins(5), "rate limited");
    pilot.hold("sonnet-api", Duration::from_mins(5), "failed");
    let why = pilot.ready().unwrap_err();
    assert!(why.contains("every access path"), "{why}");
    assert!(why.contains("sonnet-api for 5 min"), "{why}");
    // A hold that lifted is gone.
    assert_eq!(pilot.live(now + Duration::from_mins(6)).len(), 3);
}

#[test]
fn a_path_that_could_not_answer_is_held_back_as_the_next_takes_over() {
    let held = Held::default();
    let taken = Arc::new(Mutex::new(0));
    let counted = Arc::clone(&taken);
    let next = Standby {
        label: "sonnet-api".into(),
        arm: Box::new(move |_: &Limit| {
            *counted.lock().unwrap() += 1;
            Err("TEST: no mind built here".into())
        }),
    };
    let wrapped = holding(next, "sonnet-low".into(), Arc::clone(&held));
    let limit = Limit {
        turn: 7,
        why: "429".into(),
        lifts_in: Some(Duration::from_mins(20)),
        failed: false,
        by: None,
    };
    assert!((wrapped.arm)(&limit).is_err());
    assert_eq!(*taken.lock().unwrap(), 1, "the fallback was still asked");
    let until = held.lock().unwrap()["sonnet-low"];
    let left = until.saturating_duration_since(Instant::now());
    assert!(left > Duration::from_mins(19) && left <= Duration::from_mins(20));
    // A fallback that played and failed is held back by its own name.
    let house = holding(
        house_at_the_end(
            "expert".into(),
            baylee_seat::house::level("expert").unwrap(),
        ),
        "sonnet-low".into(),
        Arc::clone(&held),
    );
    let failed = Limit {
        turn: 9,
        why: "502".into(),
        lifts_in: None,
        failed: true,
        by: Some("sonnet-agy".into()),
    };
    let swap = (house.arm)(&failed).expect("the house always takes over");
    let declared = swap.declared.expect("the house declares itself");
    assert_eq!(declared.kind, seat_mind::Kind::House as i32);
    assert_eq!(declared.level, "expert");
    assert!(held.lock().unwrap().contains_key("sonnet-agy"));
}

#[test]
fn a_path_that_keeps_failing_before_its_games_is_held_back_ever_longer() {
    let dir = Scratch::new("failing");
    let pilot = LlmPilot::new(&setup(&dir, "sonnet-low"), keys()).unwrap();
    let left = |pilot: &LlmPilot| {
        pilot.held.lock().unwrap()["sonnet-api"].saturating_duration_since(Instant::now())
    };
    pilot.failed("sonnet-api", "TEST: the model list did not answer");
    let first = left(&pilot);
    assert!(
        first > Duration::from_mins(9) && first <= FAILED_HOLD,
        "{first:?}"
    );
    pilot.failed("sonnet-api", "TEST: still not");
    let second = left(&pilot);
    assert!(second > Duration::from_mins(19), "doubled: {second:?}");
    for _ in 0..10 {
        pilot.failed("sonnet-api", "TEST: never");
    }
    assert!(left(&pilot) <= MOST_HOLD);
}

/// Most models have one access path: the profile alone, no fallback, the
/// declared house behind it. It starts and is ready.
#[test]
fn a_model_with_one_access_path_starts_with_the_house_behind_it() {
    let dir = Scratch::new("solo");
    let pilot = LlmPilot::new(&setup(&dir, "sonnet-api"), keys()).unwrap();
    assert_eq!(pilot.paths.len(), 1);
    assert_eq!(pilot.profile(), "model:sonnet-api");
    assert_eq!(pilot.declared().model, "claude-sonnet-5-5");
    assert_eq!(pilot.declared().effort, "low");
    assert_eq!(pilot.ready(), Ok(()));
    pilot.failed("sonnet-api", "TEST: the only path is down");
    assert!(pilot.ready().is_err(), "no path left: no offer");
}

/// The account's name and rating claim one model at one effort: a path
/// that names no model (a CLI's own default) or no effort is refused, alone
/// as in a chain, saying what to add.
#[test]
fn a_lone_path_that_names_no_model_or_no_effort_is_refused() {
    let dir = Scratch::new("unnamed");
    for (profile, missing) in [
        ("no-model", "names no model"),
        ("no-effort", "names no effort"),
    ] {
        let why = LlmPilot::new(&setup(&dir, profile), keys()).unwrap_err();
        let why = format!("{why:#}");
        assert!(why.contains(missing), "{why}");
        assert!(why.contains("in the settings file"), "{why}");
    }
}
