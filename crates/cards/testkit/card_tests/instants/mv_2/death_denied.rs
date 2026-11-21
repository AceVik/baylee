//! `cards/instants/mv_2/death_denied.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "94f23a93-cd98-4ee7-9e62-9b7b49ace6fd"
fn death_denied() -> CardIndex {
    card_index("94f23a93-cd98-4ee7-9e62-9b7b49ace6fd")
}

/// Death Denied ({X}{B}{B}): returns X target creature cards ONLY from YOUR
/// graveyard to your hand (excluding noncreature cards and opponent's graveyard).
#[test]
fn death_denied_returns_x_target_creature_cards_from_your_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[
            death_denied(),
            index::GRIZZLY_BEARS,
            index::HILL_GIANT,
            index::LIGHTNING_BOLT,
        ],
        &[index::SERRA_ANGEL],
    );
    hand_to_graveyard(&mut engine, P0, index::GRIZZLY_BEARS);
    hand_to_graveyard(&mut engine, P0, index::HILL_GIANT);
    hand_to_graveyard(&mut engine, P0, index::LIGHTNING_BOLT);
    hand_to_graveyard(&mut engine, P1, index::SERRA_ANGEL);

    let bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = in_graveyard(&engine, P0, index::HILL_GIANT).unwrap();
    let bolt = in_graveyard(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    let angel = in_graveyard(&engine, P1, index::SERRA_ANGEL).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut engine, P0, death_denied());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(options.contains(&bears) && options.contains(&giant));
    assert!(!options.contains(&bolt), "noncreature instant excluded");
    assert!(!options.contains(&angel), "opponent's creature excluded");

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P0, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&engine, P0, index::LIGHTNING_BOLT).is_some());
    assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
}
