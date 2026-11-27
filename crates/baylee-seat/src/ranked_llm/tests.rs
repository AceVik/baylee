use super::*;
use baylee_client_core::llmseat::keys::{KeyEntry, MemoryKeys};
use baylee_protocol::v1::seat_mind;
use baylee_seat::bridge::{Limit, Standby};
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

#[test]
fn a_chain_reaches_one_model_at_one_effort() {
    use seat_mind::Kind::{LlmApi, LlmCli};
    let cli = mind(LlmCli, "claude", "haiku", "high");
    let api = mind(LlmApi, "anthropic", "claude-haiku-4-5", "high");
    assert_eq!(same_model(&cli, &cli), Ok(()));
    assert_eq!(
        same_model(&cli, &api),
        Ok(()),
        "the CLI's name is a word of the id"
    );
    assert_eq!(same_model(&api, &cli), Ok(()));
    assert_eq!(
        same_model(&cli, &mind(LlmCli, "claude", "claude-haiku-4-5", "high")),
        Err("«claude-haiku-4-5» is not «haiku»".into()),
        "two CLI paths name the model alike"
    );
    // Another model, another effort, or a model left to the CLI's default,
    // is refused.
    for other in [
        mind(LlmApi, "anthropic", "claude-sonnet-4-5", "high"),
        mind(LlmApi, "anthropic", "claude-haiku-4-5", "low"),
        mind(LlmCli, "codex", "haiku", "high"),
        mind(LlmCli, "claude", "", "high"),
        mind(LlmApi, "anthropic", "claude-haiku-4-5", ""),
        SeatMind {
            kind: seat_mind::Kind::House as i32,
            ..cli.clone()
        },
    ] {
        assert!(same_model(&cli, &other).is_err(), "{other:?}");
    }
    assert!(
        same_model(
            &mind(LlmApi, "openai", "deepseek-v4-flash", "high"),
            &mind(LlmApi, "anthropic", "deepseek-v4-flash", "high")
        )
        .is_ok(),
        "one model behind two wire protocols"
    );
}

/// A settings file: Haiku on a CLI, on a second CLI program, and behind
/// the API, all at high effort; and a chain that crosses to Sonnet.
fn settings(dir: &std::path::Path) -> PathBuf {
    let program = std::env::current_exe().unwrap();
    let path = dir.join("llm-seat.json");
    let file = serde_json::json!({
        "profiles": {
            "haiku-high": {
                "provider": "cli", "model": "claude:haiku", "effort": "high",
                "command": program, "fallback": ["haiku-second", "haiku-api"]
            },
            "haiku-second": {
                "provider": "cli", "model": "claude:haiku", "effort": "high",
                "command": program
            },
            "haiku-api": {
                "provider": "anthropic", "model": "claude-haiku-4-5", "effort": "high",
                "key_env": "TEST_RANKED_KEY"
            },
            "crossing": {
                "provider": "cli", "model": "claude:haiku", "effort": "high",
                "command": program, "fallback": ["sonnet-api"]
            },
            "sonnet-api": {
                "provider": "anthropic", "model": "claude-sonnet-4-5", "effort": "high",
                "key_env": "TEST_RANKED_KEY"
            },
            "Upper": {
                "provider": "cli", "model": "claude:haiku", "effort": "high",
                "command": program
            }
        }
    });
    std::fs::write(&path, file.to_string()).unwrap();
    path
}

fn keys() -> Arc<dyn KeyStore> {
    let keys = MemoryKeys::default();
    keys.set(
        &KeyEntry::new("TEST_RANKED_KEY", "https://api.anthropic.com").unwrap(),
        "TEST-ranked-0123456789abcdef",
    )
    .unwrap();
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
    let pilot = LlmPilot::new(&setup(&dir, "haiku-high"), keys()).unwrap();
    assert_eq!(pilot.kind(), AiKind::Model);
    assert_eq!(pilot.profile(), "model:haiku-high");
    let names: Vec<&str> = pilot.paths.iter().map(|p| p.profile.as_str()).collect();
    assert_eq!(names, ["haiku-high", "haiku-second", "haiku-api"]);
    assert_eq!(pilot.declared().model, "haiku");
    assert_eq!(pilot.declared().effort, "high");
    // No per-game budget named: none held, by the owner's decision.
    for path in &pilot.paths {
        assert_eq!(path.join.spend_tokens, Some(UNLIMITED_TOKENS));
        assert_eq!(path.join.spend_calls, Some(UNLIMITED_CALLS));
    }

    let why = LlmPilot::new(&setup(&dir, "crossing"), keys()).unwrap_err();
    assert!(format!("{why:#}").contains("another model"), "{why:#}");
    let why = LlmPilot::new(&setup(&dir, "Upper"), keys()).unwrap_err();
    assert!(format!("{why:#}").contains("model:<profile>"), "{why:#}");
    // A path whose key is nowhere cannot be reached: refused before any
    // offer.
    let why =
        LlmPilot::new(&setup(&dir, "haiku-high"), Arc::new(MemoryKeys::default())).unwrap_err();
    assert!(format!("{why:#}").contains("haiku-api"), "{why:#}");
}

#[test]
fn a_held_back_path_sits_out_new_games_and_all_held_makes_no_offer() {
    let dir = Scratch::new("held");
    let pilot = LlmPilot::new(&setup(&dir, "haiku-high"), keys()).unwrap();
    let now = Instant::now();
    assert_eq!(pilot.ready(), Ok(()));
    assert_eq!(pilot.live(now).len(), 3);
    // The first subscription is rate limited: the next game starts on the
    // second, the API behind it.
    pilot.hold("haiku-high", Duration::from_mins(5), "rate limited");
    let live: Vec<&str> = pilot
        .live(Instant::now())
        .iter()
        .map(|p| p.profile.as_str())
        .collect();
    assert_eq!(live, ["haiku-second", "haiku-api"]);
    pilot.hold("haiku-second", Duration::from_mins(5), "rate limited");
    pilot.hold("haiku-api", Duration::from_mins(5), "failed");
    let why = pilot.ready().unwrap_err();
    assert!(why.contains("every access path"), "{why}");
    assert!(why.contains("haiku-api for 5 min"), "{why}");
    // A hold that lifted is gone.
    assert_eq!(pilot.live(now + Duration::from_mins(6)).len(), 3);
}

#[test]
fn a_path_that_could_not_answer_is_held_back_as_the_next_takes_over() {
    let held = Held::default();
    let taken = Arc::new(Mutex::new(0));
    let counted = Arc::clone(&taken);
    let next = Standby {
        label: "haiku-api".into(),
        arm: Box::new(move |_: &Limit| {
            *counted.lock().unwrap() += 1;
            Err("TEST: no mind built here".into())
        }),
    };
    let wrapped = holding(next, "haiku-high".into(), Arc::clone(&held));
    let limit = Limit {
        turn: 7,
        why: "429".into(),
        lifts_in: Some(Duration::from_mins(20)),
        failed: false,
        by: None,
    };
    assert!((wrapped.arm)(&limit).is_err());
    assert_eq!(*taken.lock().unwrap(), 1, "the fallback was still asked");
    let until = held.lock().unwrap()["haiku-high"];
    let left = until.saturating_duration_since(Instant::now());
    assert!(left > Duration::from_mins(19) && left <= Duration::from_mins(20));
    // A fallback that played and failed is held back by its own name.
    let house = holding(
        house_at_the_end(
            "expert".into(),
            baylee_seat::house::level("expert").unwrap(),
        ),
        "haiku-high".into(),
        Arc::clone(&held),
    );
    let failed = Limit {
        turn: 9,
        why: "502".into(),
        lifts_in: None,
        failed: true,
        by: Some("haiku-second".into()),
    };
    let swap = (house.arm)(&failed).expect("the house always takes over");
    let declared = swap.declared.expect("the house declares itself");
    assert_eq!(declared.kind, seat_mind::Kind::House as i32);
    assert_eq!(declared.level, "expert");
    assert!(held.lock().unwrap().contains_key("haiku-second"));
}
