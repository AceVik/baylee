use super::*;
use baylee_client_core::llmseat::keys::MemoryKeys;
use baylee_client_core::llmseat::ledger::{Book, Budget};
use baylee_seat::Disclosure;
use std::path::Path;

/// 2026-09-30 12:00 UTC, two hours east.
const NOW: Moment = Moment {
    unix: 1_790_769_600,
    offset: Some(7200),
};

/// A chair's name tells the truth about its mind (the owner's rule):
/// the house signs in as the house and a script as a test, never as a
/// language model, and the seat holds the table to that same name.
fn join(mind: &str) -> Join {
    join_with(mind, &[]).expect("a command line")
}

/// `join` with more arguments after `--mind`.
fn join_with(mind: &str, more: &[&str]) -> Result<Join, clap::Error> {
    let head = ["--mind", mind];
    join_args(&head.iter().chain(more).copied().collect::<Vec<_>>())
}

/// `join TEST-room` with `args`.
fn join_args(args: &[&str]) -> Result<Join, clap::Error> {
    let head = ["baylee-seat", "join", "TEST-room"];
    Cli::try_parse_from(head.iter().chain(args)).map(|cli| match cli.command {
        Command::Join(join) => *join,
        Command::Key(_) => panic!("not a join"),
    })
}

/// A placeholder key, so a language-model mind can be built without
/// one; nothing is ever sent with it. No other variable is set: no
/// config directory, so no settings file, as on a machine without one.
fn placeholder(name: &str) -> Option<String> {
    (name == "ANTHROPIC_API_KEY" || name == "BAYLEE_LLM_API_KEY")
        .then(|| "TEST-placeholder-key".to_string())
}

fn chosen(join: &Join, env: &dyn Fn(&str) -> Option<String>) -> anyhow::Result<Chosen> {
    choose(
        join,
        &[],
        env,
        &MemoryKeys::default(),
        AIProfile::default(),
        NOW,
    )
}

/// A directory of this test's own, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-seat-main-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A settings file at `dir/llm-seat.json`: a default Sonnet profile of
/// $2 a game under a day's cap of $3.
fn settings_in(dir: &Path) -> PathBuf {
    let path = dir.join("llm-seat.json");
    std::fs::write(
        &path,
        r#"{
              "default": "sonnet",
              "caps": {"day_usd": 3},
              "profiles": {
                "sonnet": {"provider": "anthropic", "model": "claude-sonnet-5-5",
                           "game_usd": 2, "think_secs": 30},
                "keyed": {"provider": "anthropic", "model": "claude-opus-5-5",
                          "key_env": "TEST_OWN_KEY"}
              }
            }"#,
    )
    .unwrap();
    path
}

/// The file's content and each file's bytes in `dir`, to see that
/// nothing there was touched.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read(&path).unwrap())
        })
        .collect();
    files.sort();
    files
}

#[test]
fn every_mind_sits_under_the_name_of_what_it_is() {
    let deck = Deck::acceptance("Victory").unwrap();
    let minds = [
        ("house", "HOUSE-house", "HOUSE-"),
        ("scripted", "TEST-scripted", "TEST-"),
        ("anthropic", "LLM-sonnet-5-5", "LLM-"),
        ("anthropic:claude-opus-5-5", "LLM-opus-5-5", "LLM-"),
        ("openai:deepseek-chat", "LLM-deepseek", "LLM-"),
    ];
    for (spec, named, prefix) in minds {
        // A model with no price sits down only with a token budget.
        let join = join_with(spec, &["--spend-tokens", "100000"]).unwrap();
        let chosen = chosen(&join, &placeholder).unwrap();
        let mind = &*chosen.mind;
        assert_eq!(display_name(None, &chosen.label, mind).unwrap(), named);
        let named_too = display_name(Some("x1"), &chosen.label, mind).unwrap();
        assert_eq!(named_too, format!("{prefix}x1"), "--name keeps the prefix");
        let core = seat_core(BridgeConfig::default(), &deck, mind);
        assert_eq!(core.disclosure(), mind.disclosure());
        let llm = matches!(join.mind, Some(MindKind::Llm(_)));
        for name in [named, named_too.as_str()] {
            assert!(core.disclosure().names(name), "the seat refuses «{name}»");
            assert_eq!(Disclosure::Llm.names(name), llm, "«{name}» and a model");
        }
        // What it tells the table answers it, for the game's record.
        let declared = &chosen.declared;
        assert_eq!(baylee_protocol::mind::fault(declared), None, "{spec}");
        let model = match spec {
            "anthropic" => baylee_seat::llm::DEFAULT_ANTHROPIC_MODEL,
            other => other.split_once(':').map_or("", |(_, model)| model),
        };
        assert_eq!(declared.model, model, "{spec}");
        assert_eq!(declared.level, if spec == "house" { "steady" } else { "" });
    }
}

/// A key on the command line is refused, by its shape or by being the
/// environment's key (a profile's own variable's too), and the refusal
/// does not repeat it.
/// A chair ticket comes on stdin and never on the command line: the
/// switch takes no value, so there is no argument a ticket could ride
/// in, and it is only for a bridge held by its starter (`--tethered`,
/// whose stdin carries it) and named to one chair.
#[test]
fn a_chair_ticket_is_a_switch_and_never_an_argument() {
    let ticket = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let sat = join_args(&["--tethered", "--chair", "1", "--chair-ticket"]).expect("parses");
    assert!(sat.chair_ticket && sat.tethered);
    let valued = format!("--chair-ticket={ticket}");
    for refused in [
        vec!["--tethered", "--chair", "1", valued.as_str()],
        vec!["--tethered", "--chair", "1", "--chair-ticket", ticket],
        vec!["--chair", "1", "--chair-ticket"],
        vec!["--tethered", "--chair-ticket"],
    ] {
        assert!(join_args(&refused).is_err(), "{refused:?} parsed");
    }
    let plain = join_args(&["--tethered", "--chair", "1"]).expect("parses");
    assert!(!plain.chair_ticket, "a guest unless told");
}

#[test]
fn a_key_on_the_command_line_is_refused_without_being_printed() {
    let args = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
    let fine = args(&["join", "room", "--mind", "anthropic"]);
    assert!(no_key_in(&fine, &placeholder, &[]).is_ok());
    let shaped = args(&["join", "room", "--name", "sk-ant-api03-AAAABBBBCCCCDDDD"]);
    let refused = no_key_in(&shaped, &|_| None, &[]).unwrap_err().to_string();
    assert!(refused.contains("argument 3"), "{refused}");
    assert!(!refused.contains("AAAABBBB"), "{refused}");
    let same = args(&["join", "room", "--password", "TEST-placeholder-key"]);
    assert!(no_key_in(&same, &placeholder, &[]).is_err());
    let own = |name: &str| (name == "TEST_OWN_KEY").then(|| "TEST-own-key-value".to_string());
    let theirs = args(&["join", "room", "--password", "TEST-own-key-value"]);
    assert!(
        no_key_in(&theirs, &own, &[]).is_ok(),
        "not a variable it knows"
    );
    assert!(no_key_in(&theirs, &own, &["TEST_OWN_KEY"]).is_err());
    // No key in the environment, no language model.
    let missing = chosen(&join("anthropic"), &|_| None);
    assert!(missing.is_err());
}

/// A model this build has no price for does not sit down under a
/// dollar budget nobody can hold: it states its price, or a token
/// budget as its limit, and `--help` says so.
#[test]
fn a_model_with_no_price_states_its_price_or_its_token_limit() {
    let sits = |mind: &str, more: &[&str]| {
        let join = join_with(mind, more).expect("a command line");
        chosen(&join, &placeholder)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    let unpriced = "openai:deepseek-chat";
    let refused = sits(unpriced, &[]).unwrap_err();
    for named in [
        "deepseek-chat",
        "--price-in",
        "--price-out",
        "--spend-tokens",
    ] {
        assert!(refused.contains(named), "{refused}");
    }
    assert_eq!(refused.lines().count(), 1, "one sentence: {refused}");
    // A dollar budget for it is refused, token budget or not.
    for more in [
        &["--spend-usd", "3"][..],
        &["--spend-usd", "3", "--spend-tokens", "9"],
    ] {
        let refused = sits(unpriced, more).unwrap_err();
        assert!(refused.contains("--price-in"), "{refused}");
    }
    // The accepted forms: a token limit, a price (with or without a
    // budget of its own), and a model this build has a price for.
    sits(unpriced, &["--spend-tokens", "200000"]).unwrap();
    sits(unpriced, &["--price-in", "0.3", "--price-out", "1.2"]).unwrap();
    let priced = [
        "--price-in",
        "0.3",
        "--price-out",
        "1.2",
        "--spend-usd",
        "2",
    ];
    sits(unpriced, &priced).unwrap();
    sits("anthropic", &[]).unwrap();
    sits("anthropic:claude-opus-5-5", &["--spend-usd", "1"]).unwrap();
    // A price is both halves, and an amount of dollars.
    assert!(join_with(unpriced, &["--price-in", "0.3"]).is_err());
    assert!(join_with(unpriced, &["--price-out", "1.2"]).is_err());
    for bad in ["-1", "NaN", "inf", "a"] {
        let price_in = format!("--price-in={bad}");
        let more = [price_in.as_str(), "--price-out", "1"];
        assert!(join_with(unpriced, &more).is_err(), "{bad}");
        let spend = format!("--spend-usd={bad}");
        assert!(join_with("anthropic", &[&spend]).is_err(), "{bad}");
    }
    // What `join --help` says about it.
    let mut cli = <Cli as clap::CommandFactory>::command();
    let help = cli
        .find_subcommand_mut("join")
        .expect("join")
        .render_long_help()
        .to_string();
    let help = help.split_whitespace().collect::<Vec<_>>().join(" ");
    for said in [
        "A model with no price is refused unless --spend-tokens states its limit",
        "required, for a model this build has no price for",
        "--price-in",
        "--price-out",
        "--profile",
        "BAYLEE_SEAT_CONFIG",
        "llm-spend.json",
    ] {
        assert!(help.contains(said), "--help lacks «{said}»:\n{help}");
    }
}

/// With no settings file the bridge is what it was: the house when told
/// nothing, a model with the build's limits and no spend book, and a
/// profile it cannot have.
#[test]
fn with_no_settings_file_the_bridge_plays_as_before() {
    let bare = chosen(&join_args(&[]).unwrap(), &placeholder).unwrap();
    assert_eq!(bare.label, "house");
    assert!(bare.tally.is_none() && bare.booked.is_none());
    assert_eq!(bare.think_secs, DEFAULT_THINK_SECS);
    let model = chosen(&join("anthropic"), &placeholder).unwrap();
    assert!(model.tally.is_some());
    assert!(model.booked.is_none(), "no file, no book");
    assert_eq!(model.think_secs, 60);
    let refused = chosen(&join_args(&["--profile", "sonnet"]).unwrap(), &placeholder)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("--profile sonnet") && refused.contains("none"),
        "{refused}"
    );
    let refused = chosen(
        &join_with("house", &["--profile", "x"]).unwrap(),
        &placeholder,
    )
    .map(|_| ())
    .unwrap_err()
    .to_string();
    assert!(refused.contains("--mind house plays none"), "{refused}");
}

/// Given `--config` (or `BAYLEE_SEAT_CONFIG`), the player's own config
/// directory is neither read nor written, whatever it holds: here a
/// file that would refuse if it were read, beside which no book grows.
#[test]
fn a_named_file_keeps_the_real_config_directory_out_of_it() {
    let dir = scratch("real-dir");
    let real = dir.join("xdg").join("baylee");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("llm-seat.json"), r#"{"caps": {"week_usd": 1}}"#).unwrap();
    let before = snapshot(&real);
    let mine = dir.join("mine");
    std::fs::create_dir_all(&mine).unwrap();
    let config = settings_in(&mine);
    let xdg = dir.join("xdg").display().to_string();
    let env = |name: &str| match name {
        "XDG_CONFIG_HOME" => Some(xdg.clone()),
        _ => placeholder(name),
    };
    // The premise: without the flag, the real directory is where the
    // bridge looks, and its file is refused.
    let refused = chosen(&join_args(&[]).unwrap(), &env)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(refused.contains("week_usd"), "{refused}");

    let named = join_args(&["--config", config.to_str().unwrap()]).unwrap();
    let game = chosen(&named, &env).expect("the named file plays");
    assert_eq!(game.label, "sonnet-5-5");
    assert_eq!(game.think_secs, 30);
    drop(game);
    let by_env = |name: &str| match name {
        "BAYLEE_SEAT_CONFIG" => Some(config.display().to_string()),
        _ => env(name),
    };
    drop(chosen(&join_args(&[]).unwrap(), &by_env).expect("the file the environment names"));
    assert_eq!(snapshot(&real), before, "the real directory is untouched");
    let book = Book::beside(&config).read().unwrap();
    assert_eq!(
        book.games.len(),
        2,
        "both games are in the book beside the named file"
    );
    assert!(book.games.iter().all(|g| g.settled.is_some()));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file named must be there: a mistyped path never drops the caps.
#[test]
fn a_named_settings_file_must_be_there() {
    let dir = scratch("missing");
    let gone = dir.join("nothing-here.json");
    let named = join_args(&["--config", gone.to_str().unwrap()]).unwrap();
    let refused = chosen(&named, &placeholder)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(refused.contains("there is no settings file"), "{refused}");
    let by_env = |name: &str| match name {
        "BAYLEE_SEAT_CONFIG" => Some(gone.display().to_string()),
        _ => placeholder(name),
    };
    assert!(chosen(&join("anthropic"), &by_env).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Under the file, a game reserves before it sits down and plays under
/// what it was granted; with the day's cap taken it is refused with a
/// sentence, and a game that ended gives back what it did not spend.
#[test]
fn under_a_settings_file_a_game_reserves_before_it_sits_down() {
    let dir = scratch("reserve");
    let config = settings_in(&dir);
    let named = join_args(&["--config", config.to_str().unwrap()]).unwrap();
    let first = chosen(&named, &placeholder).unwrap();
    assert_eq!(
        first.booked.as_ref().unwrap().grant().budget,
        Budget::Usd(2.0)
    );
    let second = chosen(&named, &placeholder).unwrap();
    let left = match second.booked.as_ref().unwrap().grant().budget {
        Budget::Usd(usd) => usd,
        Budget::Tokens(_) => panic!("dollars"),
    };
    assert!((left - 1.0).abs() < 1e-9, "what the day's cap left: {left}");
    let refused = chosen(&named, &placeholder)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(refused.contains("the day's cap of $3.00"), "{refused}");
    assert!(refused.contains("local time, UTC+02:00"), "{refused}");
    drop(first);
    let third = chosen(&named, &placeholder).expect("the first game gave its $2 back");
    drop((second, third));

    // --mind over the file still counts in its book; the house does not.
    let model = join_with(
        "anthropic:claude-opus-5-5",
        &["--config", config.to_str().unwrap()],
    )
    .unwrap();
    assert!(chosen(&model, &placeholder).unwrap().booked.is_some());
    let house = join_with("house", &["--config", config.to_str().unwrap()]).unwrap();
    assert!(chosen(&house, &placeholder).unwrap().booked.is_none());
    let games = Book::beside(&config).read().unwrap().games.len();
    assert_eq!(games, 4, "the refusal reserved nothing, the house nothing");

    // A profile's key comes from its own variable.
    let keyed = join_args(&["--config", config.to_str().unwrap(), "--profile", "keyed"]).unwrap();
    let refused = chosen(&keyed, &placeholder)
        .map(|_| ())
        .unwrap_err()
        .to_string();
    assert!(refused.contains("set TEST_OWN_KEY"), "{refused}");
    let own = |name: &str| (name == "TEST_OWN_KEY").then(|| "TEST-own-key-value".to_string());
    assert!(chosen(&keyed, &own).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

/// An agent CLI sits under its tool's and model's name and reads no
/// key; its program is checked before the game reserves anything, so a
/// CLI that is not there costs the book nothing. The program here is
/// the test itself: found, and never started.
#[test]
fn a_cli_sits_with_no_key_and_is_checked_before_it_reserves() {
    let dir = scratch("cli");
    let config = dir.join("llm-seat.json");
    let program = std::env::current_exe().unwrap();
    let file = serde_json::json!({
        "default": "cc",
        "caps": {"day_tokens": 50_000_000},
        "profiles": {
            "cc": {"provider": "cli", "model": "claude:opus", "game_calls": 200,
                   "command": program},
            "gone": {"provider": "cli", "model": "claude",
                     "command": dir.join("nowhere").join("claude")}
        }
    });
    std::fs::write(&config, file.to_string()).unwrap();
    let empty = dir.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let env = |name: &str| (name == "PATH").then(|| empty.display().to_string());
    let path = config.to_str().unwrap();
    let cc = join_args(&["--config", path]).unwrap();
    let chosen_cc = chosen(&cc, &env).unwrap();
    assert_eq!(chosen_cc.label, "claude-opus");
    let mind = &*chosen_cc.mind;
    assert_eq!(
        display_name(None, &chosen_cc.label, mind).unwrap(),
        "LLM-claude-opus"
    );
    assert_eq!(mind.disclosure(), Disclosure::Llm);
    assert_eq!(
        chosen_cc.booked.as_ref().unwrap().grant().budget,
        Budget::Tokens(20_000_000)
    );
    let tally = chosen_cc.tally.as_ref().unwrap();
    assert_eq!(tally.lock().unwrap().calls_cap, Some(200));
    drop(chosen_cc);
    for (join, said) in [
        (
            join_args(&["--config", path, "--profile", "gone"]).unwrap(),
            "is not a program this user may run",
        ),
        // With no profile to name it, the tool is looked for on PATH.
        (join("cli:claude"), "claude is not on PATH"),
    ] {
        let refused = chosen(&join, &env).map(|_| ()).unwrap_err().to_string();
        assert!(refused.contains(said), "{refused}");
    }
    let games = Book::beside(&config).read().unwrap().games.len();
    assert_eq!(games, 1, "a CLI that is not there reserved nothing");
    // The flag over the profile's cap.
    let flagged = join_args(&["--config", path, "--spend-calls", "7"]).unwrap();
    let tally = chosen(&flagged, &env).unwrap().tally.unwrap();
    assert_eq!(tally.lock().unwrap().calls_cap, Some(7));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--ledger` puts the book where it says, file or no file.
#[test]
fn the_book_is_where_ledger_says() {
    let dir = scratch("ledger");
    let book = dir.join("elsewhere.json");
    let named = join_with("anthropic", &["--ledger", book.to_str().unwrap()]).unwrap();
    drop(chosen(&named, &placeholder).unwrap());
    assert_eq!(Book::new(book).read().unwrap().games.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A reader of orders for a bridge started as `join` would be, seated
/// as `disclosure`.
fn reader(join: Join, disclosure: Disclosure, booked: Option<Booked>) -> OrderReader {
    OrderReader {
        join,
        disclosure,
        booked: Arc::new(Mutex::new(booked)),
        keys: Arc::new(MemoryKeys::default()),
    }
}

/// An order is chosen as a sit-down is: the profile's provider puts the
/// model in `--mind`, the effort over the profile's (none named: the
/// build's default), its game reserved in the book; and the old
/// reservation settles only once the new mind is taken.
#[test]
fn an_order_is_chosen_as_a_sit_down_and_settles_the_old_game_when_taken() {
    let dir = scratch("orders");
    let config = settings_in(&dir);
    let path = config.to_str().unwrap();
    let started = join_args(&["--config", path, "--profile", "sonnet", "--tethered"]).unwrap();
    let first = chosen(&started, &placeholder).unwrap();
    assert_eq!(first.mind.disclosure(), Disclosure::Llm);
    let reading = reader(started, Disclosure::Llm, first.booked);
    let book = Book::beside(&config);
    assert_eq!(book.read().unwrap().games.len(), 1);
    // An order for Opus over the same profile, at the build's effort.
    let line = Order::Model {
        profile: "sonnet".into(),
        model: "claude-opus-5-5".into(),
        effort: None,
    }
    .line();
    let order = Order::parse(&line).unwrap();
    let join = reading.ordered(&order, &placeholder).unwrap();
    assert!(matches!(&join.mind, Some(MindKind::Llm(spec)) if spec.model == "claude-opus-5-5"));
    assert!(join.default_effort && join.effort.is_none());
    assert_eq!(join.profile.as_deref(), Some("sonnet"));
    let (swap, label) = reading.swap(&line, &placeholder).unwrap();
    assert_eq!(label, "opus-5-5");
    // It thinks as long as its profile says, not the build's default.
    assert_eq!(swap.think, Some(Duration::from_secs(30)));
    let games = book.read().unwrap().games;
    assert_eq!(games.len(), 2, "reserved before it is handed over");
    assert!(games.iter().all(|g| g.settled.is_none()));
    (swap.taken.unwrap())();
    let games = book.read().unwrap().games;
    assert!(
        games[0].settled.is_some(),
        "the old game settles when taken"
    );
    assert!(games[1].settled.is_none(), "the new one plays on");
    // The house may play a language model's chair.
    let (house, label) = reading
        .swap(r#"{"mind":"house","level":"sharp"}"#, &placeholder)
        .unwrap();
    assert_eq!(label, "house");
    assert_eq!(house.think, Some(Duration::from_secs(DEFAULT_THINK_SECS)));
    let _ = std::fs::remove_dir_all(&dir);
}

/// An order the chair's name would lie about, a profile the file does
/// not have, a key and a missing one are refused, each in a sentence,
/// and nothing is reserved.
#[test]
fn an_order_the_chair_cannot_take_is_refused() {
    let dir = scratch("orders-refused");
    let config = settings_in(&dir);
    let path = config.to_str().unwrap();
    let started = join_args(&["--config", path, "--tethered"]).unwrap();
    let model = Order::Model {
        profile: "sonnet".into(),
        model: "claude-sonnet-5-5".into(),
        effort: Some("high".into()),
    }
    .line();
    let house = reader(started.clone(), Disclosure::House, None);
    let why = house.swap(&model, &placeholder).unwrap_err().to_string();
    assert!(why.contains("HOUSE-"), "{why}");
    let llm = reader(started, Disclosure::Llm, None);
    let gone = r#"{"mind":"model","profile":"gone","model":"claude-opus-5-5"}"#;
    let why = llm.swap(gone, &placeholder).unwrap_err().to_string();
    assert!(why.contains("no profile «gone»"), "{why}");
    let why = llm.swap(&model, &|_| None).unwrap_err().to_string();
    assert!(why.contains("ANTHROPIC_API_KEY"), "{why}");
    let key = format!(r#"{{"mind":"house","level":"sk-ant-{}"}}"#, "B".repeat(30));
    let why = llm.swap(&key, &placeholder).unwrap_err().to_string();
    assert!(!why.contains("BBBB"), "{why}");
    assert_eq!(Book::beside(&config).read().unwrap().games.len(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `baylee-seat key` as the client runs it, against a store in memory.
fn key_args(args: &[&str]) -> KeyArgs {
    let head = ["baylee-seat", "key"];
    match Cli::try_parse_from(head.iter().chain(args))
        .unwrap()
        .command
    {
        Command::Key(key) => key,
        Command::Join(_) => panic!("not a key command"),
    }
}

/// The client keeps, replaces and forgets a key through the bridge,
/// which reads it from stdin and answers only whether one is kept; a
/// CLI's profile has none, and a store that cannot be opened says so.
#[test]
fn a_key_is_kept_and_forgotten_and_never_printed() {
    let dir = scratch("key-command");
    let file = settings_in(&dir);
    let config = file.to_string_lossy().into_owned();
    let store = MemoryKeys::default();
    let none = |_: &str| None;
    let run = |action: &str, profile: &str, typed: &str, store: &MemoryKeys| {
        let args = key_args(&[action, "--profile", profile, "--config", &config]);
        key_line(&args, &none, store, &mut typed.as_bytes())
    };
    assert_eq!(run("status", "keyed", "", &store).unwrap(), "absent");
    let said = run("set", "keyed", "TEST-kept-0123456789abcdef\n", &store).unwrap();
    assert_eq!(said, "set");
    let at = KeyEntry::new("TEST_OWN_KEY", "https://api.anthropic.com").unwrap();
    assert_eq!(
        store.get(&at).unwrap().as_deref(),
        Some("TEST-kept-0123456789abcdef"),
        "under its variable and its host"
    );
    let refused = run("set", "keyed", "two words\n", &store).unwrap_err();
    assert!(!format!("{refused:#}").contains("two"), "{refused:#}");
    assert_eq!(run("delete", "keyed", "", &store).unwrap(), "absent");
    assert!(run("status", "nobody", "", &store).is_err());
    let off = MemoryKeys::unavailable("TEST: no store here");
    assert_eq!(
        run("status", "keyed", "", &off).unwrap(),
        "unavailable: TEST: no store here"
    );
    assert!(run("set", "keyed", "TEST-kept-0123456789abcdef\n", &off).is_err());
    // By its parts, as the client names it: the same entry.
    let parts = key_args(&[
        "status",
        "--key-env",
        "TEST_OWN_KEY",
        "--host",
        "api.anthropic.com",
    ]);
    store.set(&at, "TEST-kept-0123456789abcdef").unwrap();
    assert_eq!(
        key_line(&parts, &none, &store, &mut "".as_bytes()).unwrap(),
        "set"
    );
    let bad = key_args(&["status", "--key-env", "TEST_OWN_KEY", "--host", "a/b"]);
    assert!(key_line(&bad, &none, &store, &mut "".as_bytes()).is_err());
    assert!(
        Cli::try_parse_from(["baylee-seat", "key", "status", "--key-env", "X"]).is_err(),
        "a variable without its host"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A key kept in the store sits a language model down where its
/// variable is unset, and only for the host it was kept for.
#[test]
fn a_kept_key_sits_a_model_down_without_the_environment() {
    let dir = scratch("key-kept");
    let file = settings_in(&dir);
    let config = file.to_string_lossy().into_owned();
    let join = join_args(&["--profile", "keyed", "--config", &config]).unwrap();
    let none = |_: &str| None;
    let store = MemoryKeys::default();
    let pick = |store: &MemoryKeys| {
        choose(&join, &[], &none, store, AIProfile::default(), NOW).map(|c| c.label)
    };
    let refused = pick(&store).expect_err("no key anywhere");
    assert!(
        format!("{refused:#}").contains("TEST_OWN_KEY"),
        "{refused:#}"
    );
    let elsewhere = KeyEntry::new("TEST_OWN_KEY", "https://llm.example.org").unwrap();
    store.set(&elsewhere, "TEST-kept-0123456789abcdef").unwrap();
    assert!(pick(&store).is_err(), "kept for another host");
    let here = KeyEntry::new("TEST_OWN_KEY", "https://api.anthropic.com").unwrap();
    store.set(&here, "TEST-kept-0123456789abcdef").unwrap();
    assert!(pick(&store).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}
