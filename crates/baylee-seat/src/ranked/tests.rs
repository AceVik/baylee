use super::player::public_name;
use super::*;
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
fn a_public_handle_is_read_as_its_name() {
    assert_eq!(public_name("HOUSE-Expert#1a2b"), "HOUSE-Expert");
    assert_eq!(public_name("HOUSE-Expert"), "HOUSE-Expert");
}

#[test]
fn a_receipt_says_who_won_from_the_seat() {
    let receipt = |outcome, seat| Receipt {
        outcome,
        seat,
        elo: 1500,
        delta: 0,
    };
    assert_eq!(receipt(Outcome::FirstWins, 0).won(), Some(true));
    assert_eq!(receipt(Outcome::FirstWins, 1).won(), Some(false));
    assert_eq!(receipt(Outcome::SecondWins, 1).won(), Some(true));
    assert_eq!(receipt(Outcome::Draw, 0).won(), None);
}

#[test]
fn a_shelf_deck_freezes_to_the_rows_the_commitment_binds() {
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
        "a deck of another format is not queued"
    );
}

#[test]
fn the_house_pilots_at_a_fair_level_only() {
    let pilot = Pilot::house("expert").unwrap();
    assert_eq!(pilot.disclosure, crate::Disclosure::House);
    assert_eq!((pilot.make)().disclosure(), crate::Disclosure::House);
    assert!(
        Pilot::house("seer").is_err(),
        "the level that peeks is refused"
    );
    assert!(Pilot::house("no such level").is_err());
}

#[test]
fn an_ai_account_is_labelled_by_centrals_prefix_or_its_minds() {
    use crate::Disclosure;
    assert!(labelled("KI-Haus-Expert", Disclosure::House));
    assert!(labelled("HOUSE-Expert", Disclosure::House));
    assert!(
        !labelled("LLM-Haiku", Disclosure::House),
        "another mind's label"
    );
    assert!(!labelled("KI-", Disclosure::House), "a prefix with no name");
    assert!(!labelled("Alice", Disclosure::House));
}
