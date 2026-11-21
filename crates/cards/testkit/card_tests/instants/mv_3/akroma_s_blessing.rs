//! `cards/instants/mv_3/akroma_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "a3fc6888-0a4b-4e1d-b709-042b8b6b1a04"
fn akroma_s_blessing() -> CardIndex {
    card_index("a3fc6888-0a4b-4e1d-b709-042b8b6b1a04")
}

/// Akroma's Blessing ({2}{W}): chooses a color and grants creatures YOU
/// control (not opponent's) protection from that color until end of turn, then
/// wears off next turn.
#[test]
fn akromas_blessing_protects_only_your_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[akroma_s_blessing(), shock(), shock()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut engine, P0, akroma_s_blessing());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, shock());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Shock target choice, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&bears),
        "own creature has protection from red"
    );
    assert!(
        options.contains(&giant),
        "opponent's creature is NOT protected"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    // Next turn protection wears off and Shock can target Bears again.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, shock(), &[bears], &[]);
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
}

/// Cycling {W}: discards Akroma's Blessing from hand and draws a card.
#[test]
fn akromas_blessing_cycles_for_one_white_and_draws() {
    let mut engine = lk::game(SEED, &[], &[], &[akroma_s_blessing()], &[]);
    let card = in_hand(&engine, P0, akroma_s_blessing()).unwrap();
    let lib_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 0,
            },
        )
        .expect("cycling {W} is legal from hand");
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0);
    assert!(in_graveyard(&engine, P0, akroma_s_blessing()).is_some());
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        lib_before - 1
    );
}
