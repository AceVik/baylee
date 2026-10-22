//! `cards/creatures/mv_5/purraj_of_urborg.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "ae86965c-3330-48bf-abf7-3142c61099ea"
fn purraj_of_urborg() -> CardIndex {
    card_index("ae86965c-3330-48bf-abf7-3142c61099ea")
}

#[test]
fn purraj_of_urborg_gets_a_plus_one_counter_when_paying_black_on_black_spell_and_has_first_strike_while_attacking()
 {
    let mut engine = lk::game(SEED, &[purraj_of_urborg()], &[], &[index::DARK_RITUAL], &[]);
    let purraj = lk::obj(&engine, P0, purraj_of_urborg());
    assert!(
        !engine
            .state()
            .object(purraj)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);

    let ch = engine.state().object(purraj).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(4)));

    // Declare Purraj attacking to verify conditional first strike.
    let _offer = lk::attack(&mut engine, &[purraj]);
    assert!(
        engine
            .state()
            .object(purraj)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
}
