//! A deck built through an agent CLI (`baylee_seat::cli::CliMind::converse`,
//! `baylee-seat deck build`), against the fake: the builder's brief, not a
//! seat's, under the same lockdown; the conversation's opening sent once;
//! the deck the model proposes held to the host's check and kept as a
//! version.

use super::*;
use baylee_account::ranked::Format;
use baylee_client_core::cardquery::PoolFactsBody;
use baylee_seat::cli::Brief;
use baylee_seat::deckbuild::facts::{self, Facts};
use baylee_seat::deckbuild::history;

/// Facts in which only `names` are legal, in every format.
fn facts_of(names: &[&str]) -> Facts {
    let mut cards: Vec<Vec<u32>> = names
        .iter()
        .map(|name| vec![baylee_cards::decks::by_name(name).unwrap().get(), 0])
        .collect();
    cards.sort();
    Facts {
        gateway: "https://gateway.test".into(),
        fetched: "2026-11-27T03:00:00Z".into(),
        pool: PoolFactsBody {
            version: "0123456789abcdef".into(),
            formats: vec!["commander".into(), "standard".into(), "vintage".into()],
            sets: Vec::new(),
            verdicts: vec!["lll".into()],
            cards,
        },
        blocked: Vec::new(),
    }
}

fn answer(value: &Value) -> Value {
    json!({"kind": "answer", "answer": value})
}

/// The process is started with the brief it is given, its opening heard
/// once, and the answers it gives read back whole.
#[tokio::test]
async fn a_cli_converses_under_the_brief_it_is_given() {
    let rig = Rig::new(
        "deck-converse",
        cli(json!({})),
        &json!({"steps": [
            answer(&json!({"search": {"text": "island"}})),
            answer(&json!({"read": ["Island"]})),
        ]}),
    );
    let Access::Cli(launch) = llm::check(&rig.plan, &rig.env()).unwrap() else {
        panic!("a cli profile is a CLI");
    };
    let brief = Brief {
        system: "BUILD A DECK".into(),
        schema: json!({"type": "object", "properties": {"read": {"type": "array"}}}),
        task: "Baylee deck building",
        answer: "the action JSON",
    };
    let mind = CliMind::with_brief(rig.plan.settings.clone(), launch, Limits::default(), brief);
    let wait = Duration::from_secs(20);
    let first = mind
        .converse("deck-1", "THE OPENING", "Begin.", wait)
        .await
        .unwrap();
    assert_eq!(first.value, Some(json!({"search": {"text": "island"}})));
    let second = mind
        .converse("deck-1", "THE OPENING", "Results.", wait)
        .await
        .unwrap();
    assert_eq!(second.value, Some(json!({"read": ["Island"]})));
    let starts = rig.starts();
    assert_eq!(starts.len(), 1, "one process holds the conversation");
    let args: Vec<String> = starts[0]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().to_string())
        .collect();
    let at = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .map(|i| args[i + 1].clone())
    };
    assert_eq!(at("--system-prompt").as_deref(), Some("BUILD A DECK"));
    let schema: Value = serde_json::from_str(&at("--json-schema").unwrap()).unwrap();
    assert_eq!(
        schema,
        json!({"type": "object", "properties": {"read": {"type": "array"}}})
    );
    assert!(
        args.iter().any(|a| a == "--tools"),
        "still locked down: {args:?}"
    );
    let messages = rig.messages();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].1, "THE OPENING\n\nBegin.");
    assert_eq!(messages[1].1, "Results.", "the opening once");
}

/// `baylee-seat deck build`, the real binary, with a CLI profile: the model
/// searches, proposes a deck the check refuses, then one it passes; the
/// deck is kept as version 1 with its note and its model, and the deck
/// the next build makes is version 2, the newest.
#[test]
fn the_build_command_keeps_the_deck_a_cli_model_proposed() {
    let deck = |n: u32| {
        json!({"propose": {"name": "Islands", "cards": [{"count": n, "name": "Island"}],
                           "note": "Play lands. Win on the clock."}})
    };
    let rig = Rig::new(
        "deck-build",
        cli(json!({})),
        &json!({"steps": [
            answer(&json!({"search": {"types": "land"}})),
            answer(&deck(59)),
            answer(&deck(60)),
            answer(&deck(61)),
        ]}),
    );
    let dir = rig.home.join("decks");
    facts::save(&facts::path_in(&dir), &facts_of(&["Island"])).unwrap();
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_baylee-seat"))
            .args(["deck", "build", "--format", "standard", "--profile", "cc"])
            .args(["--rounds", "5", "--reply-secs", "20", "--config"])
            .arg(rig.home.join("llm-seat.json"))
            .arg("--dir")
            .arg(&dir)
            .env_clear()
            .env("HOME", &rig.home)
            .env("TMPDIR", &rig.home)
            .env("PATH", &rig.path)
            .env("BAYLEE_KEY_STORE", "off")
            .output()
            .expect("the bridge runs")
    };
    let out = run();
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{said}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(said.contains("refused, 1 faults"), "{said}");
    assert!(
        said.contains("is version 1 of the standard deck, after 3 replies"),
        "{said}"
    );
    let versions = history::versions(&dir, Format::Standard);
    assert_eq!(versions.len(), 1);
    let record = versions[0].read_record().unwrap();
    assert_eq!(record.how, "build");
    assert_eq!(record.model.as_deref(), Some("claude:opus"));
    assert_eq!(record.profile.as_deref(), Some("cc"));
    assert_eq!(record.note, "Play lands. Win on the clock.");
    assert_eq!(record.facts_version, "0123456789abcdef");
    assert_eq!(record.spend.as_ref().map(|s| s.rounds), Some(3));
    let text = std::fs::read_to_string(&versions[0].deck).unwrap();
    assert!(
        text.contains("# format: standard") && text.contains("60 Island"),
        "{text}"
    );
    // The runner reads it as the bridge reads any deck file.
    let read = Deck::from_file(&versions[0].deck).unwrap();
    assert_eq!(read.name, "Islands");
    // A second build is the newest version.
    let out = run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        history::newest(&dir, Format::Standard),
        history::versions(&dir, Format::Standard)
            .pop()
            .map(|v| v.deck)
    );
    assert_eq!(history::versions(&dir, Format::Standard).len(), 2);
}
