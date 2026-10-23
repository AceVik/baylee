//! `cards/creatures/mv_5/purraj_of_urborg.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

/// "You may pay": declining puts no counter on Purraj.
#[test]
fn purraj_of_urborg_declining_the_payment_adds_no_counter() {
    let mut engine = lk::game(SEED, &[purraj_of_urborg()], &[], &[index::DARK_RITUAL], &[]);
    let purraj = lk::obj(&engine, P0, purraj_of_urborg());

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);

    let ch = engine.state().object(purraj).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(3)));
}

/// "A black spell": a red spell asks nothing, and the floating {B} stays.
#[test]
fn purraj_of_urborg_ignores_a_spell_that_is_not_black() {
    let mut engine = lk::game(
        SEED,
        &[purraj_of_urborg()],
        &[],
        &[index::LIGHTNING_BOLT],
        &[],
    );
    let purraj = lk::obj(&engine, P0, purraj_of_urborg());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Black, 1)],
    );
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[], &[P1]);

    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no payment question for a red spell: {:?}",
        engine.pending()
    );
    let ch = engine.state().object(purraj).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(3)));
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Black), 1);
    assert_eq!(lk::life(&engine, P1), 17);
}

/// "Whenever a player casts": the opponent's black spell trigger too, and
/// Purraj's controller is the one asked to pay.
#[test]
fn purraj_of_urborg_triggers_on_an_opponents_black_spell() {
    let mut engine = lk::game(SEED, &[purraj_of_urborg()], &[], &[], &[index::DARK_RITUAL]);
    let purraj = lk::obj(&engine, P0, purraj_of_urborg());

    reach_their_main_phase(&mut engine, P1);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P0, "Purraj's controller decides, not the caster");
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);

    let ch = engine.state().object(purraj).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(4)));
}
