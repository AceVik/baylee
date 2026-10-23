//! `cards/enchantments/mv_4/null_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "cb75aca6-dfc7-460d-b90c-80b7988f39a8"
fn null_chamber() -> CardIndex {
    card_index("cb75aca6-dfc7-460d-b90c-80b7988f39a8")
}

/// Casts the Chamber from P0's hand and names `name` as it enters.
fn cast_naming(engine: &mut Engine<RegistryLookup>, name: CardIndex) {
    lk::float(
        engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
    );
    cast_with_floating(engine, P0, null_chamber());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: name,
                face: 0,
            },
        )
        .unwrap();
    lk::settle(engine);
}

#[test]
fn null_chamber_enters_and_chooses_a_card_name() {
    let mut engine = lk::game(SEED, &[], &[], &[null_chamber()], &[]);

    cast_naming(&mut engine, index::GRIZZLY_BEARS);

    let chamber = lk::obj(&engine, P0, null_chamber());
    assert!(
        engine
            .state()
            .object(chamber)
            .unwrap()
            .chosen_name
            .is_some()
    );
}

/// "Spells with the chosen names can't be cast": the named spell cannot be
/// cast by either player, a spell with another name still can.
#[test]
fn null_chamber_stops_both_players_casting_the_chosen_name_and_nothing_else() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[null_chamber(), index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::GRIZZLY_BEARS],
    );
    cast_naming(&mut engine, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 4),
            (ManaColor::Green, 1),
            (ManaColor::Red, 1),
        ],
    );
    assert!(
        !lk::castable(&engine, P0, index::GRIZZLY_BEARS),
        "the controller's own Grizzly Bears is locked too"
    );
    assert!(
        lk::castable(&engine, P0, index::HILL_GIANT),
        "a spell with another name is not"
    );

    reach_their_main_phase(&mut engine, P1);
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    assert!(
        !lk::castable(&engine, P1, index::GRIZZLY_BEARS),
        "the opponent's Grizzly Bears cannot be cast either"
    );
}

/// "Lands with the chosen names can't be played": a named nonbasic land is
/// refused, a basic land with another name is played.
#[test]
fn null_chamber_stops_playing_a_land_with_the_chosen_name() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[null_chamber(), index::TUNDRA, plains()],
        &[],
    );
    cast_naming(&mut engine, index::TUNDRA);

    let tundra = in_hand(&engine, P0, index::TUNDRA).unwrap();
    assert!(
        engine
            .apply(P0, PlayerAction::PlayLand { card: tundra })
            .is_err(),
        "a land with the chosen name cannot be played"
    );
    let basic = in_hand(&engine, P0, plains()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: basic })
        .expect("a land with another name can be played");
}

/// "Other than a basic land card name": naming a basic land locks nothing.
#[test]
fn null_chamber_does_not_lock_a_basic_land_name() {
    let mut engine = lk::game(SEED, &[], &[], &[null_chamber(), plains()], &[]);
    cast_naming(&mut engine, plains());

    let basic = in_hand(&engine, P0, plains()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: basic })
        .expect("a basic land name is no lock");
}
