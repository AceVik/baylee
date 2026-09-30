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
    assert_eq!(deepseek.key_env(), "DEEPSEEK_API_KEY");
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
        "BAYLEE_LLM_API_KEY"
    );
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
/// plain, cache writes and cache reads.
#[test]
fn a_call_s_worst_case_is_never_below_its_bill() {
    let sonnet = price("claude-sonnet-5-5").unwrap();
    assert_eq!(worst_tokens(10_000, 16_000), 28_000);
    let worst = worst_usd(10_000, 16_000, sonnet);
    assert!(
        (worst - (12_000.0 * 2.5 + 16_000.0 * 10.0) / 1e6).abs() < 1e-12,
        "{worst}"
    );
    for bytes in [0_u64, 1, 900, 64 * 1024, 1_000_000] {
        let input = bytes; // a token is at least a byte
        for (plain, write) in [(input, 0), (0, input), (input / 2, input / 3), (0, 0)] {
            let read = input - plain - write;
            let bill = (plain as f64 * sonnet.input
                + write as f64 * sonnet.cache_write
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
    assert!(is_loopback("http://localhost:8080/v1"));
    assert!(!is_loopback("https://api.example.com"));
    assert_eq!(address_fault("https://api.example.com/v1", "x"), None);
    assert!(address_fault("http://example.com", "BAYLEE_LLM_BASE_URL").is_some());
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
