//! `cards/instants/mv_2/psychotic_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1bf982e9-5868-46bf-81e3-f29ada33d169"
fn psychotic_fury() -> CardIndex {
    card_index("1bf982e9-5868-46bf-81e3-f29ada33d169")
}

/// Psychotic Fury ({1}{R}): targets ONLY a multicolored creature (excluding
/// monocolored and colorless creatures), grants double strike until EOT, and
/// draws a card.
#[test]
fn psychotic_fury_grants_double_strike_only_to_multicolored_creature_and_draws() {
    let mut engine = lk::game(
        SEED,
        &[index::WATCHWOLF, index::GRIZZLY_BEARS, index::ORNITHOPTER],
        &[],
        &[psychotic_fury()],
        &[],
    );
    let wolf = lk::obj(&engine, P0, index::WATCHWOLF);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let thopter = lk::obj(&engine, P0, index::ORNITHOPTER);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    cast_with_floating(&mut engine, P0, psychotic_fury());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![wolf],
        "only multicolored Watchwolf is legal (excluded mono {bears:?} and colorless {thopter:?})"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![wolf],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(keywords(&engine, wolf).contains(KeywordSet::DOUBLE_STRIKE));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "cast 1 spell, drew 1 card -> net 0"
    );

    let _ = lk::attack(&mut engine, &[wolf]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P1),
        14,
        "3/3 double strike dealt 6 damage"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, wolf).contains(KeywordSet::DOUBLE_STRIKE));
}
