//! The replay contract at its plainest: one seed and one sequence of
//! answers give one game, hash for hash; another seed gives another game;
//! and an answer from a player the engine did not ask is refused without
//! touching the state.

use super::testkit::{Duel, basic_forest, keep_mulligans, pass_until, quiet_creature};
use super::*;

const SEED: u64 = 0x5eed_0f01;

fn game(seed: u64) -> Engine<super::testkit::RegistryLookup> {
    let mut engine = Duel::new(seed, basic_forest()).start();
    keep_mulligans(&mut engine);
    engine
}

/// Passes once for whoever holds priority, answering an empty attack or
/// block where one is asked instead.
fn step(engine: &mut Engine<super::testkit::RegistryLookup>) {
    let before = engine.state().snapshot_hash();
    let turn = engine.state().turn;
    pass_until(engine, |e| {
        e.state().snapshot_hash() != before || e.state().turn != turn
    });
}

#[test]
fn one_seed_and_one_sequence_of_answers_give_one_game() {
    let mut a = game(SEED);
    let mut b = game(SEED);
    assert_eq!(a.state().snapshot_hash(), b.state().snapshot_hash());
    for i in 0..40 {
        step(&mut a);
        step(&mut b);
        assert_eq!(
            a.state().snapshot_hash(),
            b.state().snapshot_hash(),
            "the two games parted after {i} answers"
        );
    }
    assert!(a.state().turn.number > 1, "the walk crossed a turn");
}

/// A game from a deck of two kinds of card, so that a seed has a library order
/// to change and an opening hand to deal.
fn mixed_game(seed: u64) -> Engine<super::testkit::RegistryLookup> {
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo,
        SeatCapabilities, SeatController, SeatSpec,
    };
    let entry = |card| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    };
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| {
            entry(if i % 2 == 0 {
                basic_forest()
            } else {
                quiet_creature()
            })
        })
        .collect();
    let seat = || SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules {
            starting_player: Some(0),
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(), seat()],
    };
    Engine::new(&preset, super::testkit::RegistryLookup).expect("game starts")
}

#[test]
fn two_seeds_give_two_games() {
    let a = mixed_game(SEED);
    let b = mixed_game(SEED ^ 0xffff);
    assert_ne!(a.state().snapshot_hash(), b.state().snapshot_hash());
    let opening = |e: &Engine<super::testkit::RegistryLookup>| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(PlayerId::new(0)))
            .iter()
            .map(|id| e.state().object(*id).and_then(|o| o.card).map(|c| c.index))
            .collect::<Vec<_>>()
    };
    assert_eq!(opening(&a).len(), 7);
    let same = mixed_game(SEED);
    assert_eq!(opening(&a), opening(&same), "one seed deals one hand");
    // Sixty cards in two kinds: some seed in a few must deal another hand.
    let differs = (1..8u64).any(|k| opening(&mixed_game(SEED ^ (k << 8))) != opening(&a));
    assert!(differs, "another seed deals another opening hand");
}

#[test]
fn a_pass_from_a_player_without_priority_is_refused_and_changes_nothing() {
    let mut engine = game(SEED);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let other = PlayerId::new(1 - player.get());
    let before = engine.state().snapshot_hash();
    assert!(
        matches!(
            engine.apply(other, PlayerAction::PassPriority),
            Err(EngineError::IllegalAction(_) | EngineError::MismatchedAction)
        ),
        "only the player with priority may pass it"
    );
    assert_eq!(engine.state().snapshot_hash(), before);
    assert!(engine.apply(player, PlayerAction::PassPriority).is_ok());
}

#[test]
fn a_seat_that_is_not_at_the_table_is_refused() {
    let mut engine = game(SEED);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    let before = engine.state().snapshot_hash();
    assert!(matches!(
        engine.apply(PlayerId::new(7), PlayerAction::PassPriority),
        Err(EngineError::IllegalAction(_) | EngineError::MismatchedAction)
    ));
    assert_eq!(engine.state().snapshot_hash(), before);
}

#[test]
fn the_hash_moves_when_the_game_moves() {
    let mut engine = game(SEED);
    let start = engine.state().snapshot_hash();
    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_ne!(engine.state().snapshot_hash(), start);
}

#[test]
fn reading_the_state_does_not_move_the_hash() {
    let engine = game(SEED);
    let first = engine.state().snapshot_hash();
    let _ = engine.pending();
    let _ = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield);
    assert_eq!(engine.state().snapshot_hash(), first);
    assert_eq!(engine.state().clone().snapshot_hash(), first);
}
