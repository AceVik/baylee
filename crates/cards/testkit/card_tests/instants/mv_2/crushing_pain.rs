//! `cards/instants/mv_2/crushing_pain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "34424168-1353-46a9-9e60-d09a344c3573"
fn crushing_pain() -> CardIndex {
    card_index("34424168-1353-46a9-9e60-d09a344c3573")
}

/// Crushing Pain ({1}{R}): cannot target an undamaged creature (or one damaged
/// on an earlier turn); deals 6 damage to a creature dealt damage this turn.
#[test]
fn crushing_pain_targets_only_creature_dealt_damage_this_turn_for_six() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::SERRA_ANGEL, index::HILL_GIANT],
        &[crushing_pain(), shock()],
        &[],
    );
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    assert!(
        !lk::castable(&engine, P0, crushing_pain()),
        "no creature has been dealt damage this turn"
    );

    // Shock the 4/4 Angel (2 damage); Hill Giant remains undamaged.
    lk::cast_at(&mut engine, P0, shock(), &[angel], &[]);

    cast_with_floating(&mut engine, P0, crushing_pain());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![angel],
        "only the damaged Angel is a legal target (excludes undamaged {giant:?})"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some(),
        "6 more damage destroyed the 4/4 Serra Angel"
    );
}
