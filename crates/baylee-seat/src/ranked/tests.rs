use super::*;
use baylee_account::ranked::ai::{AiBadge, AiKind};
use baylee_account::ranked::{Format, Outcome};

#[test]
fn the_relay_socket_is_centrals_and_cleartext_only_on_loopback() {
    assert_eq!(
        socket_url("https://central.example").unwrap(),
        "wss://central.example/v1/ranked/socket"
    );
    assert_eq!(
        socket_url("http://127.0.0.1:28794").unwrap(),
        "ws://127.0.0.1:28794/v1/ranked/socket"
    );
    assert_eq!(
        socket_url("http://[::1]:9").unwrap(),
        "ws://[::1]:9/v1/ranked/socket"
    );
    for refused in [
        "http://central.example",
        "http://10.0.0.1:80",
        "https://user:pw@central.example",
        "https://central.example/elsewhere",
        "https://central.example/?q=1",
        "ws://127.0.0.1:1",
        "not an origin",
    ] {
        assert!(socket_url(refused).is_err(), "{refused}");
    }
}

#[test]
fn a_receipt_says_who_won_from_the_seat() {
    let receipt = |outcome, seat| Receipt {
        outcome,
        seat,
        elo: 1500,
        delta: Some(0),
    };
    assert_eq!(receipt(Outcome::FirstWins, 0).won(), Some(true));
    assert_eq!(receipt(Outcome::FirstWins, 1).won(), Some(false));
    assert_eq!(receipt(Outcome::SecondWins, 1).won(), Some(true));
    assert_eq!(receipt(Outcome::Draw, 0).won(), None);
}

#[test]
fn a_deck_freezes_to_the_rows_the_commitment_binds() {
    let cards = vec![
        "20 Forest # the plan".to_owned(),
        "40 Grizzly Bears".to_owned(),
    ];
    let frozen =
        Frozen::from_rows("Bears", "standard", Format::Standard, &cards, &[], &[]).unwrap();
    assert_eq!(frozen.deck.format, Format::Standard);
    assert!(frozen.deck.cards.iter().all(|row| !row.contains('#')));
    let again = baylee_cards::ranked::registration("standard", &frozen.deck.cards, &[], &[])
        .unwrap()
        .digest;
    assert_eq!(again, frozen.digest, "the rows sent are the rows committed");
    let told: u32 = frozen.bridge.list.main.iter().map(|c| c.count).sum();
    assert_eq!(told, 60, "the mind is told the whole deck");
    assert!(
        Frozen::from_rows("Bears", "commander", Format::Standard, &cards, &[], &[]).is_err(),
        "a deck of another format is not offered"
    );
}

/// A scratch directory, removed when dropped.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("baylee-ranked-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A deck version as the deck builder writes it: the deck and its record.
fn write_version(dir: &std::path::Path, number: u32, rows: &str) {
    let at = crate::deckbuild::history::format_dir(dir, Format::Standard);
    std::fs::create_dir_all(&at).unwrap();
    let stem = format!("v{number:04}-20261127T0{number}0000Z");
    std::fs::write(
        at.join(format!("{stem}.txt")),
        format!("# name: Bears {number}\n# format: standard\n{rows}"),
    )
    .unwrap();
    std::fs::write(at.join(format!("{stem}.json")), "{}").unwrap();
}

#[test]
fn a_shelf_offers_the_newest_deck_that_freezes_and_keeps_what_it_offered() {
    let scratch = Scratch::new();
    write_version(&scratch.0, 1, "60 Forest\n");
    let mut shelf = Shelf::directory(scratch.0.clone(), vec![Format::Standard]);
    let mut said = Vec::new();
    shelf.refresh(None, |line| said.push(line));
    let first = shelf.offers();
    assert_eq!(first.len(), 1, "{said:?}");
    // A newer version that does not freeze is passed over, and told once.
    write_version(&scratch.0, 2, "60 No Such Card Anywhere\n");
    shelf.refresh(None, |line| said.push(line));
    shelf.refresh(None, |line| said.push(line));
    assert_eq!(shelf.offers(), first, "the version before it stands");
    let passed = said.iter().filter(|l| l.contains("passing over")).count();
    assert_eq!(passed, 1, "{said:?}");
    // A newer one that freezes is offered, and the old deck stays on the
    // shelf for a match made with it.
    write_version(&scratch.0, 3, "20 Forest\n40 Grizzly Bears\n");
    shelf.refresh(None, |line| said.push(line));
    let third = shelf.offers();
    assert_ne!(third, first);
    assert!(shelf.by_digest(&first[0].deck).is_some());
    assert!(shelf.by_digest(&third[0].deck).is_some());
}

#[test]
fn the_house_pilots_at_a_fair_level_only_and_says_its_key() {
    let pilot = HousePilot::new("expert").unwrap();
    assert_eq!(pilot.kind(), AiKind::House);
    assert_eq!(pilot.profile(), "house:expert");
    assert!(
        HousePilot::new("seer").is_err(),
        "the level that peeks is refused"
    );
    assert!(HousePilot::new("no such level").is_err());
}

#[test]
fn a_pilot_plays_only_the_account_centrals_settings_name() {
    let settings = |kind, profile: &str| ai::Settings {
        badge: AiBadge {
            kind,
            label: "Haus-KI · Expert".into(),
        },
        profile: profile.into(),
        enabled: true,
        max_games: 1,
    };
    let pilot = HousePilot::new("expert").unwrap();
    assert!(fits(&pilot, &settings(AiKind::House, "house:expert")).is_ok());
    assert!(fits(&pilot, &settings(AiKind::House, "house:hard")).is_err());
    assert!(fits(&pilot, &settings(AiKind::Model, "house:expert")).is_err());
}

#[test]
fn the_runner_token_comes_from_a_file_or_the_store_and_is_never_quoted() {
    use baylee_client_core::llmseat::keys::{KeyEntry, KeyStore as _, MemoryKeys};
    let token = baylee_account::Token::generate().unwrap();
    let text = baylee_account::sdk::ai_runner::token_text(&token);
    let central = "http://127.0.0.1:28794";
    let endpoint = baylee_account::Endpoint::development(central).unwrap();
    let keys = Arc::new(MemoryKeys::default());
    keys.set(&KeyEntry::new("KI_RUNNER", central).unwrap(), &text)
        .unwrap();
    let stored = RunnerToken::from_store(&endpoint, "KI_RUNNER".into(), keys.clone());
    assert_eq!(stored.read().unwrap().digest(), token.digest());
    let missing = RunnerToken::from_store(&endpoint, "OTHER".into(), keys);
    let why = missing.read().unwrap_err().to_string();
    assert!(why.contains("--key-env OTHER"), "{why}");

    let scratch = Scratch::new();
    let file = scratch.0.join("runner-token");
    std::fs::write(&file, format!("{}\n", *text)).unwrap();
    let from_file = RunnerToken::from_file(file.clone());
    assert_eq!(from_file.read().unwrap().digest(), token.digest());
    // A password, or a gateway's token, is no runner token, and the error
    // does not repeat what the file holds.
    std::fs::write(&file, "TEST-synthetic-password-0123\n").unwrap();
    let why = from_file.read().unwrap_err().to_string();
    assert!(why.contains("holds no runner token"), "{why}");
    assert!(!why.contains("TEST-synthetic"), "{why}");
    std::fs::write(&file, "\n").unwrap();
    assert!(from_file.read().is_err(), "an empty file is no token");
    assert_eq!(format!("{from_file:?}"), "RunnerToken(…)");
}

/// The host's facts: Forest at `verdict` in Standard, and the blocks given.
fn facts(
    verdict: &str,
    blocked: Vec<baylee_client_core::lobby::BlockedCard>,
) -> crate::deckbuild::facts::Facts {
    let forest = baylee_cards::decks::by_name("Forest").expect("a pool card");
    crate::deckbuild::facts::Facts {
        gateway: "https://host.test".into(),
        fetched: "2026-11-27T03:00:00Z".into(),
        pool: baylee_client_core::cardquery::PoolFactsBody {
            version: "0123456789abcdef".into(),
            formats: vec!["standard".into()],
            sets: Vec::new(),
            verdicts: vec![verdict.into()],
            cards: vec![vec![forest.get(), 0]],
        },
        blocked,
    }
}

#[test]
fn a_deck_is_offered_only_where_the_hosts_facts_admit_it() {
    let forest = baylee_cards::decks::by_name("Forest").unwrap();
    let deck = Frozen::from_rows(
        "Forest",
        "standard",
        Format::Standard,
        &["60 Forest".into()],
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(deck.admitted(&facts("l", Vec::new())), Ok(()));
    assert!(
        deck.admitted(&facts("b", Vec::new())).is_err(),
        "banned there"
    );
    let block = |until: Option<&str>| baylee_client_core::lobby::BlockedCard {
        index: forest.get(),
        name: "Forest".into(),
        reason: "a fault".into(),
        until: until.map(Into::into),
    };
    let why = deck.admitted(&facts("l", vec![block(None)])).unwrap_err();
    assert!(why.contains("blocked"), "{why}");
    assert_eq!(
        deck.admitted(&facts("l", vec![block(Some("2026-01-01T00:00:00Z"))])),
        Ok(()),
        "a block that ended blocks nothing"
    );

    // On the shelf: a newer version the host would not seat is passed over.
    let scratch = Scratch::new();
    write_version(&scratch.0, 1, "60 Forest\n");
    write_version(&scratch.0, 2, "20 Forest\n40 Grizzly Bears\n");
    let mut shelf = Shelf::directory(scratch.0.clone(), vec![Format::Standard]);
    let mut said = Vec::new();
    // Grizzly Bears has no verdict in these facts.
    shelf.refresh(Some(&facts("l", Vec::new())), |line| said.push(line));
    assert_eq!(shelf.offers()[0].deck, deck.digest, "{said:?}");
    assert!(
        said.iter().any(|l| l.contains("would not seat")),
        "{said:?}"
    );
}

#[test]
fn a_restarted_runner_finds_an_older_version_a_match_was_made_with() {
    let scratch = Scratch::new();
    write_version(&scratch.0, 1, "60 Forest\n");
    write_version(&scratch.0, 2, "20 Forest\n40 Grizzly Bears\n");
    let mut shelf = Shelf::directory(scratch.0.clone(), vec![Format::Standard]);
    shelf.refresh(None, |_| {});
    let old = Frozen::from_rows(
        "Old",
        "standard",
        Format::Standard,
        &["60 Forest".into()],
        &[],
        &[],
    )
    .unwrap();
    assert!(
        shelf.by_digest(&old.digest).is_none(),
        "the newest is offered"
    );
    let found = shelf
        .recall(Format::Standard, &old.digest)
        .expect("version 1");
    assert_eq!(found.digest, old.digest);
    assert!(shelf.by_digest(&old.digest).is_some(), "kept from then on");
    assert!(shelf.recall(Format::Standard, &[0; 32]).is_none());
}

#[test]
fn a_commander_deck_file_freezes_with_its_commander() {
    let scratch = Scratch::new();
    let path = scratch.0.join("v0001-20261127T010000Z.txt");
    std::fs::write(
        &path,
        "# name: Aang\n# format: commander\nCMD: 1 Aang and Katara\n99 Forest\n",
    )
    .unwrap();
    let frozen = Frozen::from_file(&path, Format::Commander).unwrap();
    assert_eq!(frozen.deck.commanders, ["Aang and Katara"]);
    assert_eq!(frozen.bridge.commanders, ["Aang and Katara"]);
    let main: u32 = frozen.bridge.list.main.iter().map(|c| c.count).sum();
    assert_eq!(main, 99, "the commander is not among the main deck's cards");
}
