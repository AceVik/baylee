//! `cards/creatures/mv_5/selenia_dark_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn selenia_dark_angel() -> CardIndex {
    card_index("3db1ac09-e35a-4f50-97c4-f65dbc28daef")
}

/// `Selenia, Dark Angel` ({3}{W}{B}, 3/3 Legendary Creature — Phyrexian Angel,
/// `Coverage::Implemented`):
/// `Flying`
/// `Pay 2 life: Return Selenia to its owner's hand.`
#[test]
fn selenia_dark_angel_flies_and_returns_to_owners_hand_for_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[selenia_dark_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let selenia = on_battlefield(&engine, p0, selenia_dark_angel()).unwrap();
    let ch = engine.state().object(selenia).unwrap().characteristics();
    assert!(ch.keywords.contains(KeywordSet::FLYING));
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));

    activate(&mut engine, p0, selenia_dark_angel(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 18);
    assert!(on_battlefield(&engine, p0, selenia_dark_angel()).is_none());
    assert!(in_hand(&engine, p0, selenia_dark_angel()).is_some());
}

#[test]
fn selenia_cannot_pay_two_life_with_only_one_life_available() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game(SEED, &[selenia_dark_angel()], &[], &[], &[]);
    e.dev_state_mut(p0).unwrap().players[0].life = 1;
    e.refresh_offer();
    assert!(!lk::offered(&e, selenia_dark_angel(), p0, 0));
}
