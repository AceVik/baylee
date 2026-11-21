//! `cards/instants/mv_2/words_of_wisdom.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "cff1b9bb-2a4c-4522-94a4-a674f942e376"
fn words_of_wisdom() -> CardIndex {
    card_index("cff1b9bb-2a4c-4522-94a4-a674f942e376")
}

/// Words of Wisdom ({1}{U}): caster draws 2 cards and each other player draws
/// 1 card.
#[test]
fn words_of_wisdom_draws_two_for_you_and_one_for_each_other_player() {
    let mut engine = lk::game(SEED, &[], &[], &[words_of_wisdom()], &[]);
    let p0_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    let p0_lib = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    let p1_lib = engine.state().zones.list(ZoneLocation::Library(P1)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, words_of_wisdom());
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand + 1,
        "cast 1 spell, drew 2 cards -> net +1"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand + 1,
        "opponent drew 1 card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        p0_lib - 2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P1)).len(),
        p1_lib - 1
    );
}
