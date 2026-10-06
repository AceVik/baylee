use super::*;

/// A file with every field a profile has, and caps.
const FULL: &str = r#"{
  "default": "sonnet",
  "caps": { "day_usd": 10.0, "month_usd": 60.0, "day_tokens": 4000000 },
  "profiles": {
    "sonnet": {
      "provider": "anthropic",
      "model": "claude-sonnet-5-5",
      "effort": "high",
      "answer": "tools",
      "max_tokens": 12000,
      "game_usd": 2.5,
      "game_tokens": 3000000,
      "think_secs": 45,
      "key_env": "ANTHROPIC_API_KEY"
    },
    "deepseek": {
      "provider": "openai",
      "model": "deepseek-chat",
      "answer": "json",
      "price": { "input": 0.3, "output": 1.2 },
      "key_env": "DEEPSEEK_API_KEY",
      "base_url": "https://api.deepseek.com/v1"
    }
  }
}"#;

/// The example in `docs/llm-seat.md`, the first JSON block there.
fn doc_example() -> String {
    let doc = include_str!("../../../../docs/llm-seat.md");
    let start = doc.find("```json\n").expect("a JSON example") + "```json\n".len();
    let end = start + doc[start..].find("```").expect("its end");
    doc[start..end].to_string()
}

#[test]
fn a_file_reads_and_writes_back_the_same() {
    let settings = SeatSettings::parse(FULL).expect("the full file");
    assert_eq!(settings.default.as_deref(), Some("sonnet"));
    assert_eq!(settings.caps.day_usd, Some(10.0));
    assert_eq!(settings.caps.month_tokens, None);
    let deepseek = settings.profile("deepseek").expect("a profile");
    assert_eq!(deepseek.provider, Provider::OpenAi);
    assert_eq!(deepseek.answer, Some(AnswerMode::Json));
    // The schema's mode, as the file spells it.
    let lm = SeatSettings::parse(
        r#"{"profiles": {"lm": {"provider": "openai", "model": "m-1", "answer": "json_schema"}}}"#,
    )
    .expect("json_schema");
    assert_eq!(lm.profiles["lm"].answer, Some(AnswerMode::JsonSchema));
    assert!(lm.to_json().contains(r#""answer": "json_schema""#));
    assert_eq!(deepseek.key_env(), Some("DEEPSEEK_API_KEY"));
    let again = SeatSettings::parse(&settings.to_json()).expect("its own text");
    assert_eq!(again, settings);
    // What is the build's is left out, not written as a default.
    let bare = SeatSettings {
        profiles: BTreeMap::from([(
            "x".to_string(),
            Profile::new(Provider::Anthropic, "claude-opus-5-5"),
        )]),
        ..SeatSettings::default()
    };
    let text = bare.to_json();
    for absent in ["caps", "default", "effort", "game_usd", "key_env", "null"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    assert_eq!(SeatSettings::parse(&text).unwrap(), bare);
    assert_eq!(SeatSettings::parse("{}").unwrap(), SeatSettings::default());
}

#[test]
fn the_documented_example_is_a_file_this_build_reads() {
    let example = doc_example();
    let settings = SeatSettings::parse(&example).expect("the doc's example");
    assert!(settings.default.is_some(), "it names a default");
    assert!(!settings.caps.is_empty(), "it shows the caps");
    assert!(
        settings
            .profiles
            .values()
            .any(|p| p.price().is_none() || p.price.is_some()),
        "it shows a model with no price of this build's"
    );
    assert!(
        settings
            .profiles
            .values()
            .any(|p| p.provider == Provider::Cli && p.fault().is_none()),
        "it shows a CLI"
    );
    assert!(!example.contains("sk-"), "no key in the example");
}

/// A key written into the file is refused with the sentence that says
/// where keys go, and the refusal never repeats it.
#[test]
fn a_key_in_the_file_is_refused_with_where_keys_go() {
    let with = |field: &str, value: &str| {
        format!(
            r#"{{"profiles": {{"x": {{"provider": "anthropic", "model": "claude-sonnet-5-5", {field}: {value}}}}}}}"#
        )
    };
    let key = "\"sk-ant-api03-AAAABBBBCCCCDDDDEEEE\"";
    for field in [
        "\"api_key\"",
        "\"apiKey\"",
        "\"ANTHROPIC_API_KEY\"",
        "\"key\"",
        "\"secret\"",
        "\"token\"",
        "\"x-api-key\"",
    ] {
        let refused = SeatSettings::parse(&with(field, "\"TEST-anything\"")).unwrap_err();
        assert!(
            refused.contains("never goes in the settings file"),
            "{field}: {refused}"
        );
        assert!(refused.contains("key_env"), "{refused}");
        assert!(refused.contains("profiles.x."), "says where: {refused}");
        assert_eq!(refused.lines().count(), 1, "one sentence: {refused}");
    }
    // A value shaped like a key, under any field, the key's own name too.
    for field in ["\"key_env\"", "\"effort\"", "\"base_url\""] {
        let refused = SeatSettings::parse(&with(field, key)).unwrap_err();
        assert!(
            refused.contains("looks like an API key"),
            "{field}: {refused}"
        );
        assert!(!refused.contains("AAAABBBB"), "{refused}");
    }
    let in_model = FULL.replace("claude-sonnet-5-5", "sk-ant-api03-AAAABBBBCCCCDDDDEEEE");
    let refused = SeatSettings::parse(&in_model).unwrap_err();
    assert!(refused.contains("profiles.sonnet.model"), "{refused}");
    assert!(!refused.contains("AAAABBBB"), "{refused}");
    // The file's own fields are not keys.
    SeatSettings::parse(FULL).expect("game_tokens, max_tokens and key_env are fields");
}

#[test]
fn a_field_the_file_does_not_have_is_refused() {
    for (text, named) in [
        (r#"{"profile": {}}"#, "profile"),
        (r#"{"caps": {"week_usd": 3}}"#, "week_usd"),
        (FULL.replace("\"effort\"", "\"efort\"").as_str(), "efort"),
        (
            r#"{"profiles": {"x": {"provider": "gemini", "model": "m"}}}"#,
            "gemini",
        ),
        (r#"{"profiles": {"x": {"model": "m"}}}"#, "provider"),
    ] {
        let refused = SeatSettings::parse(text).unwrap_err();
        assert!(refused.contains(named), "{named}: {refused}");
    }
    assert!(
        SeatSettings::parse("not json")
            .unwrap_err()
            .contains("not JSON")
    );
}

/// Each profile fault is one sentence that names the profile.
#[test]
fn a_profile_that_cannot_play_is_refused_by_name() {
    let one = |fields: &str| {
        format!(
            r#"{{"profiles": {{"cheap": {{"provider": "openai", "model": "m-1", {fields}}}}}}}"#
        )
    };
    let anthropic_json = r#"{"profiles": {"cheap": {"provider": "anthropic", "model": "claude-sonnet-5-5", "answer": "json"}}}"#;
    for (text, said) in [
        (one(r#""effort": "Very High""#), "an effort is a word"),
        (
            anthropic_json.to_string(),
            "answer json is for an OpenAI-compatible endpoint",
        ),
        (
            anthropic_json.replace(r#""json""#, r#""json_schema""#),
            "answer json_schema is for an OpenAI-compatible endpoint",
        ),
        (one(r#""max_tokens": 0"#), "max_tokens"),
        (one(r#""think_secs": 0"#), "think_secs"),
        (
            one(r#""price": {"input": -1, "output": 2}"#),
            "a price is two amounts",
        ),
        (
            one(r#""game_usd": 2"#),
            "no price for «m-1», so game_usd cannot be held",
        ),
        (
            one(r#""key_env": "my key""#),
            "key_env names an environment variable",
        ),
        (
            one(r#""base_url": "http://api.example.com""#),
            "base_url must be an https://",
        ),
        (
            r#"{"profiles": {"x": {"provider": "openai", "model": "no spaces"}}}"#.into(),
            "is not a model id",
        ),
    ] {
        let refused = SeatSettings::parse(&text).unwrap_err();
        assert!(refused.contains(said), "{said}: {refused}");
        assert_eq!(refused.lines().count(), 1, "{refused}");
    }
    let refused = SeatSettings::parse(&one(r#""max_tokens": 0"#)).unwrap_err();
    assert!(refused.starts_with("profile «cheap»"), "{refused}");
    // A price makes a dollar budget holdable, and a server on this machine
    // may speak plain http.
    SeatSettings::parse(&one(
        r#""price": {"input": 0.3, "output": 1.2}, "game_usd": 2"#,
    ))
    .expect("a price and a dollar budget");
    SeatSettings::parse(&one(r#""base_url": "http://127.0.0.1:8080/v1""#)).expect("loopback");
}

#[test]
fn the_default_names_a_profile_and_names_are_words() {
    let refused = SeatSettings::parse(r#"{"default": "opus"}"#).unwrap_err();
    assert!(
        refused.contains("«opus»") && refused.contains("it has none"),
        "{refused}"
    );
    let refused =
        SeatSettings::parse(&FULL.replace("\"default\": \"sonnet\"", "\"default\": \"opus\""))
            .unwrap_err();
    assert!(refused.contains("it has deepseek, sonnet"), "{refused}");
    for bad in ["", "-x", "two words", "a:b", &"x".repeat(33)] {
        let text = format!(
            r#"{{"profiles": {{"{bad}": {{"provider": "anthropic", "model": "claude-sonnet-5-5"}}}}}}"#
        );
        let refused = SeatSettings::parse(&text).unwrap_err();
        assert!(
            refused.contains("is not a profile name"),
            "«{bad}»: {refused}"
        );
    }
    for good in ["sonnet", "Deep_Seek-2", "4o"] {
        assert!(profile_name_is_a_name(good), "{good}");
    }
    let refused = SeatSettings::parse(r#"{"caps": {"day_usd": -3}}"#).unwrap_err();
    assert!(refused.contains("a cap in dollars"), "{refused}");
}

#[test]
fn a_profile_s_price_is_its_own_then_the_build_s() {
    let mut profile = Profile::new(Provider::Anthropic, "claude-opus-5-5");
    assert_eq!(profile.price(), price("claude-opus-5-5"));
    profile.price = Some(GivenPrice {
        input: 1.0,
        output: 3.0,
    });
    assert_eq!(profile.price(), Some(Price::per_million(1.0, 3.0)));
    assert_eq!(Profile::new(Provider::OpenAi, "m-1").price(), None);
    assert_eq!(
        Profile::new(Provider::OpenAi, "m-1").key_env(),
        Some("BAYLEE_LLM_API_KEY")
    );
    // A CLI plays on a subscription: no price, and no key to read.
    let mut cli = Profile::new(Provider::Cli, "claude:opus");
    assert_eq!(cli.key_env(), None);
    cli.key_env = Some("ANTHROPIC_API_KEY".into());
    assert_eq!(
        cli.key_env(),
        None,
        "a cli profile reads none, whatever it says"
    );
    assert_eq!(
        Profile::new(Provider::Cli, "claude-sonnet-5-5").price(),
        None
    );
}

/// A CLI's model names its tool first, then the tool's own model if any.
#[test]
fn a_cli_model_names_its_tool_first() {
    assert_eq!(cli_model("claude"), Ok((CliTool::Claude, None)));
    assert_eq!(
        cli_model("claude:opus"),
        Ok((CliTool::Claude, Some("opus")))
    );
    assert_eq!(
        cli_model("claude:claude-opus-5-5"),
        Ok((CliTool::Claude, Some("claude-opus-5-5")))
    );
    assert_eq!(cli_model("agy"), Ok((CliTool::Agy, None)));
    assert_eq!(
        cli_model("agy:gemini-3.8-flash-high"),
        Ok((CliTool::Agy, Some("gemini-3.8-flash-high")))
    );
    assert_eq!(
        cli_model("gemini:gemini-3.8-flash-high"),
        Ok((CliTool::Agy, Some("gemini-3.8-flash-high")))
    );
    for bad in [
        "codex",
        "claude-opus-5-5",
        "",
        "claude:bad model",
        "sk-ant-api03-AAAABBBBCCCCDDDD",
    ] {
        let why = cli_model(bad).expect_err(bad);
        assert!(!why.contains("AAAABBBB"), "{why}");
    }
    assert!(cli_model("unknown").unwrap_err().contains("claude"));
    assert_eq!(Provider::Cli.default_answer(), AnswerMode::JsonSchema);
    assert_eq!(Provider::Cli.default_key_env(), None);
    assert_eq!(Provider::Cli.default_base(), None);
    assert!(is_absolute_path("/opt/homebrew/bin/claude"));
    assert!(is_absolute_path(r"C:\Program Files\claude.exe"));
    assert!(is_absolute_path("C:/tools/claude.exe"));
    assert!(is_absolute_path(r"\\server\share\claude.exe"));
    for relative in ["claude", "./claude", "bin/claude", "C:claude", ""] {
        assert!(!is_absolute_path(relative), "{relative}");
    }
}

/// A day or month capped in dollars needs a token cap for a model whose
/// dollars nobody can count.
#[test]
fn a_model_with_no_price_needs_a_token_cap_where_dollars_are_capped() {
    let caps = |day_usd, month_usd, day_tokens, month_tokens| Caps {
        day_usd,
        month_usd,
        day_tokens,
        month_tokens,
    };
    assert_eq!(caps(None, None, None, None).unpriced_fault("m"), None);
    let refused = caps(Some(5.0), None, None, None)
        .unpriced_fault("m")
        .unwrap();
    assert!(
        refused.contains("a day_tokens") && refused.contains("«m»"),
        "{refused}"
    );
    let refused = caps(Some(5.0), Some(9.0), Some(1), None)
        .unpriced_fault("m")
        .unwrap();
    assert!(refused.contains("a month_tokens"), "{refused}");
    assert_eq!(
        caps(Some(5.0), Some(9.0), Some(1), Some(2)).unpriced_fault("m"),
        None
    );
    assert_eq!(caps(None, None, Some(1), None).unpriced_fault("m"), None);
}

/// The worst case of a call is never below its bill, for any reply the
/// provider may send under `max_tokens` and any split of the input between
/// plain, cache writes for five minutes or an hour, and cache reads.
#[test]
fn a_call_s_worst_case_is_never_below_its_bill() {
    let sonnet = price("claude-sonnet-5-5").unwrap();
    assert_eq!(worst_tokens(10_000, 16_000), 28_000);
    let worst = worst_usd(10_000, 16_000, sonnet);
    assert!(
        (worst - (12_000.0 * 4.0 + 16_000.0 * 10.0) / 1e6).abs() < 1e-12,
        "{worst}"
    );
    for bytes in [0_u64, 1, 900, 64 * 1024, 1_000_000] {
        let input = bytes; // a token is at least a byte
        for (plain, write, hour) in [
            (input, 0, 0),
            (0, input, 0),
            (0, 0, input),
            (input / 2, input / 3, input / 7),
            (0, 0, 0),
        ] {
            let read = input - plain - write - hour;
            let bill = (plain as f64 * sonnet.input
                + write as f64 * sonnet.cache_write
                + hour as f64 * sonnet.cache_write_hour
                + read as f64 * sonnet.cache_read
                + 16_000.0 * sonnet.output)
                / 1e6;
            assert!(
                worst_usd(bytes, 16_000, sonnet) >= bill,
                "{bytes} {plain} {write}"
            );
            assert!(worst_tokens(bytes, 16_000) >= input + 16_000);
        }
    }
}

#[test]
fn key_shapes_are_blanked_and_words_are_not() {
    assert_eq!(
        blank_key_shapes("key sk-ant-api03-AAAABBBBCCCCDDDD-x ok"),
        "key sk-[redacted] ok"
    );
    assert_eq!(blank_key_shapes("a task-list"), "a task-list");
    assert!(shaped_like_a_key("Bearer abc.def"));
    assert!(!shaped_like_a_key("claude-sonnet-5-5"));
    // A key pasted after a model id, which ends in a digit, is a key. A
    // generic marker (`sk-`) glued to a lowercase word is that word's, and
    // no key, so a path such as `desk-tools-collection` and `risk-free`
    // pass; a provider's long marker counts wherever it stands, so a key
    // glued to a lowercase word is still refused, and a word that merely
    // contains the marker is too short after it to be one.
    let key = "sk-ant-api03-AAAABBBBCCCCDDDDEEEE";
    assert!(shaped_like_a_key(&format!("claude-sonnet-5-5{key}")));
    assert!(shaped_like_a_key(&format!("MODEL={key}")));
    assert!(!shaped_like_a_key("/opt/desk-tools-collection/bin"));
    assert!(shaped_like_a_key(&format!("sonnet{key}")));
    assert!(shaped_like_a_key("sonnetsk-ant-AAAABBBBCCCCDDDDEEEEFFFF"));
    assert!(shaped_like_a_key("sonnetsk-proj-AAAABBBBCCCCDDDDEEEEFFFF"));
    assert!(shaped_like_a_key("my_ghp_AAAABBBBCCCCDDDDEEEEFFFF"));
    assert!(!shaped_like_a_key("risk-free"));
    assert!(!shaped_like_a_key("task-ant-hill"));
    assert!(!shaped_like_a_key("a task-proj-board"));
    // The generic `sk-` keeps its word rule: glued to a word, sixteen key
    // characters after it are still that word's.
    assert!(!shaped_like_a_key("risk-AAAABBBBCCCCDDDDEEEE"));
    assert!(shaped_like_a_key("risk sk-AAAABBBBCCCCDDDDEEEE"));
    assert!(is_loopback("http://localhost:8080/v1"));
    assert!(!is_loopback("https://api.example.com"));
    assert_eq!(address_fault("https://api.example.com/v1", "x"), None);
    assert!(address_fault("http://example.com", "BAYLEE_LLM_BASE_URL").is_some());
}

/// Settings with one profile, `p`, as `edit` leaves it.
fn one_profile(edit: impl FnOnce(&mut Profile)) -> SeatSettings {
    let mut profile = Profile::new(Provider::OpenAi, "m-1");
    edit(&mut profile);
    SeatSettings {
        profiles: BTreeMap::from([("p".to_string(), profile)]),
        ..SeatSettings::default()
    }
}

/// Every refusal of `check` is a fault that says where it is, and `check`
/// says the first of them in the very sentence the fault carries: one
/// predicate, printed for the bridge and placed for a panel.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one case per fault, read top to bottom"
)]
fn every_refusal_is_a_fault_with_a_place() {
    let profile = |field| Place::Profile("p".into(), field);
    let cli = |edit: fn(&mut Profile)| {
        one_profile(|p| {
            p.provider = Provider::Cli;
            p.model = "claude:opus".into();
            edit(p);
        })
    };
    let cases: Vec<(SeatSettings, Place, Why)> = vec![
        (
            one_profile(|p| p.model = "sk-ant-api03-AAAABBBBCCCCDDDDEEEE".into()),
            profile(Field::Model),
            Why::KeyShaped,
        ),
        (
            cli(|p| p.model = "codex:o5".into()),
            profile(Field::Model),
            Why::NoSuchTool,
        ),
        (
            cli(|p| p.answer = Some(AnswerMode::Tools)),
            profile(Field::Answer),
            Why::NotForCli,
        ),
        (
            cli(|p| {
                p.price = Some(GivenPrice {
                    input: 1.0,
                    output: 2.0,
                });
            }),
            profile(Field::PriceInput),
            Why::NotForCli,
        ),
        (
            cli(|p| p.game_usd = Some(2.0)),
            profile(Field::GameUsd),
            Why::NotForCli,
        ),
        (
            cli(|p| p.key_env = Some("ANTHROPIC_API_KEY".into())),
            profile(Field::KeyEnv),
            Why::NotForCli,
        ),
        (
            cli(|p| p.base_url = Some("https://api.anthropic.com".into())),
            profile(Field::BaseUrl),
            Why::NotForCli,
        ),
        (
            cli(|p| p.game_calls = Some(0)),
            profile(Field::GameCalls),
            Why::Zero,
        ),
        (
            cli(|p| p.command = Some("bin/claude".into())),
            profile(Field::Command),
            Why::NotAbsolute,
        ),
        (
            one_profile(|p| p.command = Some("/usr/local/bin/claude".into())),
            profile(Field::Command),
            Why::CliOnly,
        ),
        (
            one_profile(|p| p.key_env = Some("sk-ant-api03-AAAABBBBCCCCDDDDEEEE".into())),
            profile(Field::KeyEnv),
            Why::KeyShaped,
        ),
        (
            {
                let mut settings = one_profile(|_| {});
                let p = settings.profiles.remove("p").unwrap();
                settings.profiles.insert("api_key".into(), p);
                settings
            },
            Place::Name("api_key".into()),
            Why::KeyNamed,
        ),
        (
            SeatSettings {
                default: Some("opus".into()),
                ..one_profile(|_| {})
            },
            Place::Default,
            Why::NoSuchProfile,
        ),
        (
            {
                let mut settings = one_profile(|_| {});
                let p = settings.profiles.remove("p").unwrap();
                settings.profiles.insert("two words".into(), p);
                settings
            },
            Place::Name("two words".into()),
            Why::NotAName,
        ),
        (
            one_profile(|p| p.model = "no spaces".into()),
            profile(Field::Model),
            Why::NotAModelId,
        ),
        (
            one_profile(|p| p.effort = Some("Very High".into())),
            profile(Field::Effort),
            Why::NotAWord,
        ),
        (
            one_profile(|p| {
                p.provider = Provider::Anthropic;
                p.answer = Some(AnswerMode::Json);
            }),
            profile(Field::Answer),
            Why::JsonNeedsOpenAi,
        ),
        (
            one_profile(|p| p.max_tokens = Some(0)),
            profile(Field::MaxTokens),
            Why::Zero,
        ),
        (
            one_profile(|p| {
                p.price = Some(GivenPrice {
                    input: 1.0,
                    output: -2.0,
                });
            }),
            profile(Field::PriceOutput),
            Why::NotAnAmount,
        ),
        (
            one_profile(|p| p.game_usd = Some(f64::INFINITY)),
            profile(Field::GameUsd),
            Why::NotAnAmount,
        ),
        (
            one_profile(|p| p.game_usd = Some(2.0)),
            profile(Field::GameUsd),
            Why::Unpriced,
        ),
        (
            one_profile(|p| p.think_secs = Some(0)),
            profile(Field::ThinkSecs),
            Why::Zero,
        ),
        (
            one_profile(|p| p.key_env = Some("my key".into())),
            profile(Field::KeyEnv),
            Why::NotAVariable,
        ),
        (
            one_profile(|p| p.base_url = Some("http://api.example.com".into())),
            profile(Field::BaseUrl),
            Why::NotSecure,
        ),
        (
            SeatSettings {
                caps: Caps {
                    month_usd: Some(-1.0),
                    ..Caps::default()
                },
                ..SeatSettings::default()
            },
            Place::Cap(CapField::MonthUsd),
            Why::NotAnAmount,
        ),
    ];
    for (settings, place, why) in cases {
        let faults = settings.faults();
        let first = faults
            .first()
            .unwrap_or_else(|| panic!("{place:?}: no fault"));
        assert_eq!((&first.place, first.why), (&place, why), "{faults:?}");
        assert_eq!(settings.check(), Err(first.sentence.clone()));
        assert!(!first.sentence.contains("AAAABBBB"), "{}", first.sentence);
    }
    // Every fault is listed, not only the first.
    let settings = one_profile(|p| {
        p.max_tokens = Some(0);
        p.think_secs = Some(0);
        p.base_url = Some("ftp://x".into());
    });
    let fields: Vec<Place> = settings.faults().into_iter().map(|f| f.place).collect();
    assert_eq!(
        fields,
        [Field::MaxTokens, Field::ThinkSecs, Field::BaseUrl].map(profile)
    );
    assert_eq!(one_profile(|_| {}).faults(), []);
    assert_eq!(one_profile(|_| {}).check(), Ok(()));
}

/// A profile's name is written to the file as surely as its model, so a
/// name shaped like a key is refused like one.
#[test]
fn a_profile_named_by_a_key_is_refused() {
    let key = "sk-ant-api03-AAAABBBBCCCC";
    assert!(profile_name_is_a_name(key), "a word a command line carries");
    let mut settings = one_profile(|_| {});
    let p = settings.profiles.remove("p").unwrap();
    settings.profiles.insert(key.into(), p);
    let refused = settings.check().unwrap_err();
    assert!(refused.contains("looks like an API key"), "{refused}");
    assert!(!refused.contains("AAAABBBB"), "{refused}");
    assert_eq!(settings.faults()[0].place, Place::Name(key.into()));
    let text = format!(r#"{{"profiles": {{"{key}": {{"provider": "anthropic", "model": "m"}}}}}}"#);
    assert!(SeatSettings::parse(&text).is_err());
}

/// A key fault's place is read off the file's own spelling of each field,
/// so the table of spellings has to be the file's: every field a profile
/// writes, and nothing else.
#[test]
fn the_field_paths_are_the_file_s_own() {
    fn leaves(value: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        match value {
            Value::Object(fields) => {
                for (name, inner) in fields {
                    path.push(name.clone());
                    leaves(inner, path, out);
                    path.pop();
                }
            }
            _ => out.push(path.clone()),
        }
    }
    let full = Profile {
        effort: Some("high".into()),
        answer: Some(AnswerMode::Tools),
        max_tokens: Some(1),
        price: Some(GivenPrice {
            input: 1.0,
            output: 2.0,
        }),
        game_usd: Some(1.0),
        game_tokens: Some(1),
        game_calls: Some(1),
        think_secs: Some(1),
        key_env: Some("K".into()),
        base_url: Some("https://x".into()),
        command: Some("/x".into()),
        ..Profile::new(Provider::Anthropic, "m")
    };
    let mut written = Vec::new();
    leaves(
        &serde_json::to_value(&full).unwrap(),
        &mut Vec::new(),
        &mut written,
    );
    written.sort();
    let mut table: Vec<Vec<String>> = Field::ALL
        .iter()
        .map(|field| field.path().iter().map(|s| (*s).to_string()).collect())
        .collect();
    table.sort();
    assert_eq!(written, table);
    for field in Field::ALL {
        let steps: Vec<String> = ["profiles", "p"]
            .iter()
            .chain(field.path())
            .map(|s| (*s).to_string())
            .collect();
        assert_eq!(
            Place::at(&steps, "x"),
            Place::Profile("p".into(), field),
            "{field:?}"
        );
    }
    let caps = serde_json::to_value(Caps {
        day_usd: Some(1.0),
        month_usd: Some(1.0),
        day_tokens: Some(1),
        month_tokens: Some(1),
    })
    .unwrap();
    let names: Vec<&str> = caps
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut table: Vec<&str> = CapField::ALL.iter().map(|cap| cap.name()).collect();
    table.sort_unstable();
    assert_eq!(names, table);
}

/// The models a panel suggests are the ones [`price`] knows, once each.
#[test]
fn the_priced_models_are_the_table_price_reads() {
    for (at, (_, model, listed)) in PRICED.iter().enumerate() {
        assert_eq!(price(model), Some(*listed), "{model}");
        assert!(
            PRICED[..at].iter().all(|(_, other, _)| other != model),
            "{model} twice"
        );
        assert_eq!(model_fault(model), None, "{model}");
    }
    assert_eq!(price(DEFAULT_ANTHROPIC_MODEL).map(|p| p.input), Some(2.0));
    assert_eq!(price("deepseek-chat"), None);
}

#[test]
fn the_period_a_model_with_no_price_cannot_be_counted_in() {
    let caps = Caps {
        month_usd: Some(9.0),
        day_tokens: Some(1),
        ..Caps::default()
    };
    assert_eq!(caps.unpriced_period(), Some(Period::Month));
    assert_eq!(
        Caps {
            day_usd: Some(1.0),
            ..caps
        }
        .unpriced_period(),
        Some(Period::Month)
    );
    assert_eq!(
        Caps {
            month_tokens: Some(1),
            ..caps
        }
        .unpriced_period(),
        None
    );
}

#[cfg(not(target_arch = "wasm32"))]
mod on_disk {
    use super::*;
    use std::path::PathBuf;

    /// A directory of this test's own, empty.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("baylee-llmseat-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn no_file_is_no_settings_and_a_saved_file_reads_back() {
        let dir = scratch("store");
        let path = dir.join(FILE);
        assert_eq!(store::load(&path), Ok(None));
        let settings = SeatSettings::parse(FULL).unwrap();
        store::save(&path, &settings).unwrap();
        assert_eq!(store::load(&path), Ok(Some(settings.clone())));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "the player's own");
        }
        assert!(!path.with_extension("tmp").exists(), "no temporary left");

        // What does not hold together is not written.
        let mut broken = settings;
        broken.default = Some("opus".into());
        assert!(store::save(&path, &broken).is_err());
        assert_eq!(
            store::load(&path).unwrap().unwrap().default.as_deref(),
            Some("sonnet")
        );

        // A file that is not the settings is named in the refusal.
        std::fs::write(&path, r#"{"api_key": "x"}"#).unwrap();
        let refused = store::load(&path).unwrap_err();
        assert!(refused.contains(&path.display().to_string()), "{refused}");
        assert!(refused.contains("key_env"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nothing is saved that loading would refuse: a key pasted into a
    /// model's name, or a profile named like a key, never reaches the disk,
    /// and the refusal does not repeat the key.
    #[test]
    fn a_key_is_never_saved() {
        let dir = scratch("save-key");
        let path = dir.join(FILE);
        let settings = SeatSettings::parse(FULL).unwrap();
        let pasted = "sk-ant-api03-AAAABBBBCCCCDDDDEEEEFFFF";
        let mut in_the_model = settings.clone();
        in_the_model.profiles.get_mut("sonnet").unwrap().model = pasted.into();
        let mut named_like_one = settings;
        let profile = named_like_one.profiles.remove("deepseek").unwrap();
        named_like_one.profiles.insert("api_key".into(), profile);
        for broken in [in_the_model, named_like_one] {
            let refused = broken.check().unwrap_err();
            assert!(refused.contains("key_env"), "{refused}");
            let refused = store::save(&path, &broken).unwrap_err();
            assert!(refused.contains("key_env"), "{refused}");
            assert!(!refused.contains("AAAABBBB"), "{refused}");
            assert!(!path.exists(), "nothing written");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_file_is_in_the_client_s_config_directory() {
        let env = |key: &str| (key == "HOME").then(|| std::ffi::OsString::from("/home/ada"));
        assert_eq!(
            store::default_path(crate::userdirs::Os::Other, &env),
            Some(PathBuf::from("/home/ada/.config/baylee/llm-seat.json"))
        );
        assert_eq!(
            store::default_path(crate::userdirs::Os::Other, &|_| None),
            None
        );
    }
}
