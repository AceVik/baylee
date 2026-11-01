//! `cards/creatures/mv_4/mind_maggots.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn mind_maggots() -> CardIndex {
    card_index("fcaa5a54-eb73-4d78-96a3-826d1902f642")
}

#[test]
fn mind_maggots_discards_a_creature_card_and_gets_two_plus_one_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[mind_maggots(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = in_hand(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, mind_maggots());
    kit::settle(&mut engine, p0, |_, p| {
        if matches!(p, Pending::YesNo { .. }) {
            Some(PlayerAction::YesNo(true))
        } else {
            kit::choose_cards(p, &[bears])
        }
    });

    let maggots = on_battlefield(&engine, p0, mind_maggots()).unwrap();
    assert_eq!(pt(&engine, maggots), (4, 4));
}

#[test]
fn mind_maggots_gets_no_counters_without_a_discardable_creature() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[mind_maggots(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, mind_maggots());
    kit::settle(&mut e, p0, |_, p| {
        if matches!(p, Pending::YesNo { .. }) {
            Some(PlayerAction::YesNo(true))
        } else {
            kit::choose_cards(p, &[])
        }
    });
    let maggots = on_battlefield(&e, p0, mind_maggots()).unwrap();
    assert_eq!(pt(&e, maggots), (2, 2));
    assert!(in_hand(&e, p0, forest()).is_some());
}
