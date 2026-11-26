//! `cards/creatures/mv_2/minotaur_explorer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn minotaur_explorer() -> CardIndex {
    card_index("22374ec2-61b3-46eb-82ed-93bbb42403c4")
}

/// Minotaur Explorer discards a card at random to stay on the battlefield when accepted.
#[test]
fn minotaur_explorer_discards_random_to_stay() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[minotaur_explorer(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, minotaur_explorer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, minotaur_explorer()).is_some());
    assert!(in_graveyard(&engine, p0, forest()).is_some());
}

/// Declining Minotaur Explorer's optional discard sacrifices it even with a card in hand.
#[test]
fn minotaur_explorer_sacrifices_when_discard_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[minotaur_explorer(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, minotaur_explorer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, minotaur_explorer()).is_none());
    assert!(in_graveyard(&engine, p0, minotaur_explorer()).is_some());
    assert!(in_hand(&engine, p0, forest()).is_some());
}
