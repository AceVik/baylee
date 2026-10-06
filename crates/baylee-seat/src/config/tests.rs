//! Which model plays and with what: every precedence rule and every
//! refusal, on settings built in memory. No file is read here.

use super::*;
use crate::llm::{DEFAULT_SPEND_TOKENS, DEFAULT_SPEND_USD, Provider, price};
use baylee_client_core::llmseat::{DEFAULT_THINK_SECS, GivenPrice};

/// A settings file with a default Anthropic profile that sets every field,
/// an OpenAI-compatible one with its own price, and one with no price.
fn file() -> SeatSettings {
    SeatSettings::parse(
        r#"{
          "default": "sonnet",
          "profiles": {
            "sonnet": {
              "provider": "anthropic", "model": "claude-sonnet-5-5",
              "effort": "high", "answer": "tools", "max_tokens": 12000,
              "price": {"input": 3.0, "output": 15.0},
              "game_usd": 2.5, "game_tokens": 3000000, "think_secs": 45,
              "key_env": "TEST_ANTHROPIC_KEY", "base_url": "https://llm.example.com"
            },
            "deepseek": {
              "provider": "openai", "model": "deepseek-chat", "answer": "json",
              "price": {"input": 0.3, "output": 1.2}, "key_env": "DEEPSEEK_API_KEY",
              "base_url": "https://api.deepseek.com/v1"
            },
            "local": {"provider": "openai", "model": "qwen-local"}
          }
        }"#,
    )
    .expect("the test's file")
}

fn paths() -> Paths {
    Paths {
        settings: Some(PathBuf::from("/TEST/llm-seat.json")),
        named: false,
        ledger: None,
    }
}

fn spec(text: &str) -> Spec {
    Spec::parse(text).expect("a model").expect("a valid model")
}

/// `plan` that must plan a model.
fn planned(
    mind: Option<&str>,
    file: Option<&SeatSettings>,
    named: Option<&str>,
    flags: &Overrides,
) -> Plan {
    let mind = mind.map(spec);
    plan(mind.as_ref(), file, &paths(), named, flags)
        .expect("a plan")
        .expect("a model")
}

fn refused(
    mind: Option<&str>,
    file: Option<&SeatSettings>,
    named: Option<&str>,
    flags: &Overrides,
) -> String {
    let mind = mind.map(spec);
    let why = plan(mind.as_ref(), file, &paths(), named, flags).expect_err("refused");
    assert_eq!(why.lines().count(), 1, "one sentence: {why}");
    why
}

/// With no file, a bare bridge is the house and `--mind` plays with the
/// build's defaults, exactly as before the file existed.
#[test]
fn with_no_file_nothing_changes() {
    let none = Overrides::default();
    assert!(plan(None, None, &paths(), None, &none).unwrap().is_none());
    let plan = planned(Some("anthropic"), None, None, &none);
    let mut before = Settings::new(&spec("anthropic"));
    before.budget(None, None, None).unwrap();
    assert_eq!(plan.settings.model, "claude-sonnet-5-5");
    assert_eq!(plan.settings.effort, before.effort);
    assert_eq!(plan.settings.max_tokens, before.max_tokens);
    assert_eq!(plan.settings.price, before.price);
    assert_eq!(plan.settings.spend_usd, Some(DEFAULT_SPEND_USD));
    assert_eq!(plan.settings.spend_tokens, DEFAULT_SPEND_TOKENS);
    assert!(!plan.settings.hard_limit, "no reservation, no hard limit");
    assert_eq!(plan.think_secs, DEFAULT_THINK_SECS);
    assert_eq!(plan.key_env.as_deref(), Some("ANTHROPIC_API_KEY"));
    assert_eq!((plan.base_url, plan.profile, plan.note), (None, None, None));

    let why = refused(None, None, Some("sonnet"), &none);
    assert!(
        why.contains("--profile sonnet") && why.contains("there is none"),
        "{why}"
    );
    assert!(
        why.contains("/TEST/llm-seat.json"),
        "says where it looked: {why}"
    );
}

/// With a file, a bare bridge plays the file's default profile, every
/// field of it over the build's defaults.
#[test]
fn the_default_profile_plays_when_nothing_is_named() {
    let file = file();
    let plan = planned(None, Some(&file), None, &Overrides::default());
    assert_eq!(plan.profile.as_deref(), Some("sonnet"));
    assert_eq!(plan.spec, spec("anthropic:claude-sonnet-5-5"));
    let settings = &plan.settings;
    assert_eq!(settings.effort.as_deref(), Some("high"));
    assert_eq!(settings.answer, AnswerMode::Tools);
    assert_eq!(settings.max_tokens, 12_000);
    assert_eq!(settings.price, Some(Price::per_million(3.0, 15.0)));
    assert_eq!(settings.spend_usd, Some(2.5));
    assert_eq!(settings.spend_tokens, 3_000_000);
    assert_eq!(plan.think_secs, 45);
    assert_eq!(plan.key_env.as_deref(), Some("TEST_ANTHROPIC_KEY"));
    assert_eq!(plan.base_url.as_deref(), Some("https://llm.example.com"));

    // A file with no default plays the house when nothing is named.
    let mut undecided = file.clone();
    undecided.default = None;
    assert!(
        plan_of(None, &undecided, None).is_none(),
        "no default, no model named: the house"
    );
}

fn plan_of(mind: Option<&str>, file: &SeatSettings, named: Option<&str>) -> Option<Plan> {
    let mind = mind.map(spec);
    plan(
        mind.as_ref(),
        Some(file),
        &paths(),
        named,
        &Overrides::default(),
    )
    .unwrap()
}

/// `--profile` picks a profile over the default; one the file lacks is
/// refused, naming the ones it has.
#[test]
fn a_named_profile_plays_and_an_unknown_one_is_refused() {
    let file = file();
    let plan = planned(None, Some(&file), Some("deepseek"), &Overrides::default());
    assert_eq!(plan.spec, spec("openai:deepseek-chat"));
    assert_eq!(plan.settings.answer, AnswerMode::Json);
    assert_eq!(plan.settings.effort, None, "the provider's own");
    assert_eq!(plan.settings.max_tokens, 8_000, "the build's for openai");
    assert_eq!(plan.settings.spend_usd, Some(DEFAULT_SPEND_USD));
    assert_eq!(plan.key_env.as_deref(), Some("DEEPSEEK_API_KEY"));
    assert_eq!(plan.think_secs, DEFAULT_THINK_SECS);

    let why = refused(None, Some(&file), Some("opus"), &Overrides::default());
    assert!(why.contains("no profile «opus»"), "{why}");
    assert!(why.contains("it has deepseek, local, sonnet"), "{why}");
}

/// Every flag overrides the profile's field.
#[test]
fn a_flag_overrides_the_profile() {
    let file = file();
    let flags = Overrides {
        effort: Some("low".into()),
        answer: Some(AnswerMode::Tools),
        max_tokens: Some(4_000),
        price: Some(Price::per_million(1.0, 2.0)),
        spend_usd: Some(0.75),
        spend_tokens: Some(900_000),
        spend_calls: Some(40),
        think_secs: Some(20),
        default_effort: false,
    };
    let plan = planned(None, Some(&file), Some("sonnet"), &flags);
    let settings = &plan.settings;
    assert_eq!(settings.effort.as_deref(), Some("low"));
    assert_eq!(settings.max_tokens, 4_000);
    assert_eq!(settings.price, Some(Price::per_million(1.0, 2.0)));
    assert_eq!(settings.spend_usd, Some(0.75));
    assert_eq!(settings.spend_tokens, 900_000);
    assert_eq!(settings.spend_calls, Some(40));
    assert_eq!(plan.think_secs, 20);
    // --answer over a profile's json.
    let tools = Overrides {
        answer: Some(AnswerMode::Tools),
        ..Overrides::default()
    };
    let plan = planned(None, Some(&file), Some("deepseek"), &tools);
    assert_eq!(plan.settings.answer, AnswerMode::Tools);
    // And each alone leaves the rest of the profile standing.
    let one = Overrides {
        think_secs: Some(9),
        ..Overrides::default()
    };
    let plan = planned(None, Some(&file), None, &one);
    assert_eq!((plan.think_secs, plan.settings.spend_usd), (9, Some(2.5)));
}

/// `--default-effort` drops the profile's effort for the build's: a chair
/// that plays a model which does not take it (`docs/llm-seat.md` §"A
/// language model at your table"). `--effort` still wins over both.
#[test]
fn the_default_effort_drops_the_profiles() {
    let file = file();
    let profiles = planned(None, Some(&file), Some("sonnet"), &Overrides::default());
    assert_eq!(profiles.settings.effort.as_deref(), Some("high"));
    let builds = Overrides {
        default_effort: true,
        ..Overrides::default()
    };
    let plan = planned(None, Some(&file), Some("sonnet"), &builds);
    assert_eq!(
        plan.settings.effort.as_deref(),
        Some("medium"),
        "Anthropic's"
    );
    let deepseek = planned(None, Some(&file), Some("deepseek"), &builds);
    assert_eq!(deepseek.settings.effort, None, "the endpoint's own");
    let named = Overrides {
        effort: Some("low".into()),
        ..builds
    };
    let plan = planned(None, Some(&file), Some("sonnet"), &named);
    assert_eq!(plan.settings.effort.as_deref(), Some("low"));
}

/// `--mind` over a profile of its provider changes the model and keeps
/// the rest, but not the profile's price, which was its own model's.
#[test]
fn a_model_named_over_a_profile_keeps_its_limits_and_not_its_price() {
    let file = file();
    for named in [Some("sonnet"), None] {
        let plan = planned(
            Some("anthropic:claude-opus-5-5"),
            Some(&file),
            named,
            &Overrides::default(),
        );
        assert_eq!(plan.spec.model, "claude-opus-5-5");
        assert_eq!(plan.profile.as_deref(), Some("sonnet"));
        assert_eq!(plan.settings.price, price("claude-opus-5-5"), "the build's");
        assert_eq!(plan.settings.spend_usd, Some(2.5));
        assert_eq!(plan.settings.max_tokens, 12_000);
        assert_eq!(plan.key_env.as_deref(), Some("TEST_ANTHROPIC_KEY"));
        assert_eq!(plan.note, None);
    }
    // The same model keeps the profile's price.
    let same = planned(
        Some("anthropic:claude-sonnet-5-5"),
        Some(&file),
        None,
        &Overrides::default(),
    );
    assert_eq!(same.settings.price, Some(Price::per_million(3.0, 15.0)));
}

/// `--mind` of another provider than a profile: refused when the profile
/// was named, and played with the build's defaults, said so, when it was
/// only the default.
#[test]
fn a_model_of_another_provider_than_the_profile() {
    let file = file();
    let why = refused(
        Some("openai:gpt-5"),
        Some(&file),
        Some("sonnet"),
        &Overrides::default(),
    );
    assert!(why.contains("profile «sonnet» plays anthropic"), "{why}");
    assert!(why.contains("leave out --mind"), "{why}");

    let flags = Overrides {
        spend_tokens: Some(100_000),
        ..Overrides::default()
    };
    let plan = planned(Some("openai:gpt-5"), Some(&file), None, &flags);
    assert_eq!(plan.profile, None);
    assert_eq!(plan.key_env.as_deref(), Some("BAYLEE_LLM_API_KEY"));
    assert_eq!(plan.base_url, None);
    assert_eq!(plan.settings.max_tokens, 8_000);
    let note = plan.note.expect("said");
    assert!(
        note.contains("default profile «sonnet» plays anthropic"),
        "{note}"
    );
    assert!(note.contains("this build's defaults"), "{note}");
}

/// A model with no price still needs a token budget or a price, and the
/// refusal says where the profile takes one; a flag gives it too.
#[test]
fn a_profile_with_no_price_needs_a_token_budget() {
    let file = file();
    let why = refused(None, Some(&file), Some("local"), &Overrides::default());
    for said in [
        "qwen-local",
        "--spend-tokens",
        "profile «local» takes a price, or game_tokens",
    ] {
        assert!(why.contains(said), "«{said}» in {why}");
    }
    let flags = Overrides {
        spend_tokens: Some(200_000),
        ..Overrides::default()
    };
    let plan = planned(None, Some(&file), Some("local"), &flags);
    assert_eq!((plan.settings.price, plan.settings.spend_usd), (None, None));
    assert_eq!(plan.settings.spend_tokens, 200_000);

    let mut with_tokens = file.clone();
    with_tokens.profiles.get_mut("local").unwrap().game_tokens = Some(300_000);
    let plan = plan_of(None, &with_tokens, Some("local")).unwrap();
    assert_eq!(plan.settings.spend_tokens, 300_000);
    let mut with_price = file;
    with_price.profiles.get_mut("local").unwrap().price = Some(GivenPrice {
        input: 0.1,
        output: 0.2,
    });
    let plan = plan_of(None, &with_price, Some("local")).unwrap();
    assert_eq!(plan.settings.spend_usd, Some(DEFAULT_SPEND_USD));
}

#[test]
fn a_flag_that_is_not_a_value_is_refused() {
    let file = file();
    for (flags, said) in [
        (
            Overrides {
                effort: Some("Max!".into()),
                ..Overrides::default()
            },
            "an effort is a word",
        ),
        (
            Overrides {
                answer: Some(AnswerMode::Json),
                ..Overrides::default()
            },
            "--answer json is for an OpenAI-compatible endpoint",
        ),
        (
            Overrides {
                answer: Some(AnswerMode::JsonSchema),
                ..Overrides::default()
            },
            "--answer json-schema is for an OpenAI-compatible endpoint",
        ),
        (
            Overrides {
                max_tokens: Some(0),
                ..Overrides::default()
            },
            "--max-tokens",
        ),
        (
            Overrides {
                think_secs: Some(0),
                ..Overrides::default()
            },
            "--think-secs",
        ),
    ] {
        let why = refused(Some("anthropic"), Some(&file), None, &flags);
        assert!(why.contains(said), "«{said}» in {why}");
    }
    assert_eq!(Provider::Anthropic.name(), "anthropic");
}

/// A cli profile plays its tool with no key: its token budget and call
/// cap from the profile or the flags, its command, and no answer by tools.
#[test]
fn a_cli_profile_plays_with_no_key_and_counts_its_calls() {
    let file = SeatSettings::parse(
        r#"{"default": "cc", "profiles": {
              "cc": {"provider": "cli", "model": "claude:opus", "effort": "low",
                     "game_tokens": 9000000, "game_calls": 300,
                     "command": "/opt/homebrew/bin/claude"},
              "bare": {"provider": "cli", "model": "claude"}}}"#,
    )
    .expect("the test's file");
    let none = Overrides::default();
    let plan = planned(None, Some(&file), None, &none);
    assert_eq!(plan.spec, spec("cli:claude:opus"));
    assert_eq!(plan.key_env, None, "a CLI reads no key");
    assert_eq!(plan.command.as_deref(), Some("/opt/homebrew/bin/claude"));
    let settings = &plan.settings;
    assert_eq!(settings.answer, AnswerMode::JsonSchema);
    assert_eq!(settings.effort.as_deref(), Some("low"));
    assert_eq!(
        (settings.spend_tokens, settings.spend_calls, settings.price),
        (9_000_000, Some(300), None)
    );
    let bare = planned(None, Some(&file), Some("bare"), &none);
    assert_eq!(bare.command, None, "found on PATH");
    assert_eq!(
        (bare.settings.spend_tokens, bare.settings.spend_calls),
        (
            crate::llm::DEFAULT_CLI_SPEND_TOKENS,
            Some(crate::llm::DEFAULT_CLI_CALLS)
        )
    );
    let flags = Overrides {
        spend_calls: Some(40),
        answer: Some(AnswerMode::Json),
        ..Overrides::default()
    };
    let flagged = planned(None, Some(&file), None, &flags);
    assert_eq!(flagged.settings.spend_calls, Some(40));
    assert_eq!(flagged.settings.answer, AnswerMode::Json);
    for (flags, said) in [
        (
            Overrides {
                answer: Some(AnswerMode::Tools),
                ..Overrides::default()
            },
            "--answer tools is for an API",
        ),
        (
            Overrides {
                spend_calls: Some(0),
                ..Overrides::default()
            },
            "--spend-calls",
        ),
        (
            Overrides {
                spend_usd: Some(1.0),
                ..Overrides::default()
            },
            "subscription, which has no price",
        ),
        (
            Overrides {
                price: Some(Price::per_million(1.0, 2.0)),
                ..Overrides::default()
            },
            "subscription, which has no price",
        ),
    ] {
        let why = refused(None, Some(&file), None, &flags);
        assert!(why.contains(said), "«{said}» in {why}");
        assert!(!why.contains("game_tokens"), "no API's advice: {why}");
    }
    let why = refused(
        Some("cli:claude"),
        None,
        None,
        &Overrides {
            answer: Some(AnswerMode::Tools),
            ..Overrides::default()
        },
    );
    assert!(why.contains("--answer tools"), "{why}");
}

/// The settings file is `--config`, else the environment's, else the
/// client's config directory; the book is `--ledger`, else beside a file
/// that was read, else none.
#[test]
fn where_the_file_and_the_book_are() {
    let env = |vars: &'static [(&'static str, &'static str)]| {
        move |key: &str| {
            vars.iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    };
    let home = env(&[("XDG_CONFIG_HOME", "/TEST/xdg")]);
    let default = Paths::resolve(None, None, &home);
    assert_eq!(
        default,
        Paths {
            settings: Some(PathBuf::from("/TEST/xdg/baylee/llm-seat.json")),
            named: false,
            ledger: None,
        }
    );
    assert_eq!(
        default.book(true).unwrap().path(),
        Path::new("/TEST/xdg/baylee/llm-spend.json")
    );
    assert!(default.book(false).is_none(), "no file read, no book");

    let named = env(&[
        ("XDG_CONFIG_HOME", "/TEST/xdg"),
        (CONFIG_ENV, "/TEST/env/seat.json"),
    ]);
    let by_env = Paths::resolve(None, None, &named);
    assert_eq!(by_env.settings, Some(PathBuf::from("/TEST/env/seat.json")));
    assert!(by_env.named);
    assert_eq!(
        by_env.book(true).unwrap().path(),
        Path::new("/TEST/env/llm-spend.json")
    );
    let by_flag = Paths::resolve(
        Some(Path::new("/TEST/flag/x.json")),
        Some(Path::new("/TEST/book.json")),
        &named,
    );
    assert_eq!(by_flag.settings, Some(PathBuf::from("/TEST/flag/x.json")));
    assert_eq!(
        by_flag.book(false).unwrap().path(),
        Path::new("/TEST/book.json")
    );
    // No directory at all: nothing to read.
    let bare = Paths::resolve(None, None, &|_| None);
    assert_eq!(bare.settings, None);
    assert_eq!(bare.load(), Ok(None));
}

fn scratch_file(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("llm-seat.json");
    std::fs::write(&path, text).unwrap();
    path
}

/// A file named must be there; the default one may be missing; an empty
/// variable names nothing; a refused file is one sentence, not a default.
#[test]
fn a_named_file_must_exist_and_a_default_one_may_not() {
    let missing = std::env::temp_dir().join("baylee-config-nowhere/llm-seat.json");
    let named = Paths::resolve(Some(&missing), None, &|_| None);
    let why = named.load().unwrap_err();
    assert!(why.contains("there is no settings file at"), "{why}");
    assert!(why.contains(CONFIG_ENV), "{why}");
    let default = Paths {
        settings: Some(missing),
        named: false,
        ledger: None,
    };
    assert_eq!(default.load(), Ok(None));

    let empty_var = |key: &str| (key == CONFIG_ENV).then(String::new);
    let paths = Paths::resolve(None, None, &empty_var);
    assert!(!paths.named, "an empty variable names no file");

    let good = scratch_file(
        "good",
        r#"{"default": "local", "profiles": {"local": {"provider": "openai", "model": "qwen-local"}}}"#,
    );
    let loaded = Paths::resolve(Some(&good), None, &|_| None).load().unwrap();
    assert_eq!(loaded.unwrap().default.as_deref(), Some("local"));
    let bad = scratch_file("bad", "{ nope");
    let why = Paths::resolve(Some(&bad), None, &|_| None)
        .load()
        .unwrap_err();
    assert!(!why.is_empty() && why.lines().count() == 1, "{why}");
    for path in [good, bad] {
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}

/// A key in the file is refused by the file's reader, before any plan.
#[test]
fn a_key_in_the_settings_file_is_refused() {
    let keyed = scratch_file(
        "keyed",
        r#"{"profiles": {"x": {"provider": "openai", "model": "m",
            "api_key": "sk-ant-api03-AAAABBBBCCCCDDDDEEEE"}}}"#,
    );
    let why = Paths::resolve(Some(&keyed), None, &|_| None)
        .load()
        .unwrap_err();
    assert!(!why.contains("AAAABBBB"), "the key is never echoed: {why}");
    let _ = std::fs::remove_dir_all(keyed.parent().unwrap());
}

/// Anthropic's models answer with tools: a flag or a profile that asks for
/// JSON of them is refused in one sentence.
#[test]
fn an_anthropic_model_cannot_answer_in_json() {
    let file = file();
    for answer in [AnswerMode::Json, AnswerMode::JsonSchema] {
        let why = refused(
            None,
            Some(&file),
            Some("sonnet"),
            &Overrides {
                answer: Some(answer),
                ..Overrides::default()
            },
        );
        assert!(why.contains("OpenAI-compatible endpoint or a CLI"), "{why}");
    }
    let tools = planned(
        None,
        Some(&file),
        Some("sonnet"),
        &Overrides {
            answer: Some(AnswerMode::Tools),
            ..Overrides::default()
        },
    );
    assert_eq!(tools.settings.answer, AnswerMode::Tools);
}

/// A flag beats the profile it sits over, field by field, and a field the
/// flag leaves alone stays the profile's.
#[test]
fn flags_beat_the_profile_field_by_field() {
    let file = file();
    let flags = Overrides {
        think_secs: Some(7),
        max_tokens: Some(99),
        spend_tokens: Some(1_234_567),
        ..Overrides::default()
    };
    let plan = planned(None, Some(&file), Some("sonnet"), &flags);
    assert_eq!(plan.think_secs, 7);
    assert_eq!(plan.settings.max_tokens, 99);
    assert_eq!(plan.settings.spend_tokens, 1_234_567);
    assert_eq!(
        plan.settings.effort.as_deref(),
        Some("high"),
        "the profile's"
    );
    assert_eq!(plan.base_url.as_deref(), Some("https://llm.example.com"));
    let plain = planned(None, Some(&file), Some("sonnet"), &Overrides::default());
    assert_eq!(plain.think_secs, 45);
    let tokens = Overrides {
        spend_tokens: Some(500_000),
        ..Overrides::default()
    };
    let local = planned(None, Some(&file), Some("local"), &tokens);
    assert_eq!(local.think_secs, DEFAULT_THINK_SECS);
    assert_eq!(local.profile.as_deref(), Some("local"));
    assert_eq!(local.note, None);
}

/// A profile that is not named and not the default plays nothing: with no
/// `--mind` the house plays.
#[test]
fn with_no_default_and_no_mind_the_house_plays() {
    let text =
        r#"{"profiles": {"a": {"provider": "openai", "model": "m", "game_tokens": 1000000}}}"#;
    let no_default = SeatSettings::parse(text).expect("a file");
    assert!(
        plan(
            None,
            Some(&no_default),
            &paths(),
            None,
            &Overrides::default()
        )
        .unwrap()
        .is_none()
    );
    let named = planned(None, Some(&no_default), Some("a"), &Overrides::default());
    assert_eq!(named.profile.as_deref(), Some("a"));
}
