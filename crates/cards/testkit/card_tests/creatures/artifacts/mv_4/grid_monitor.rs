//! Grid Monitor restricts only its controller's creature spells.

use super::*;

fn grid_monitor() -> CardIndex {
    card_index("b947b9a7-7a42-447d-b3fd-e20dba8a8f41")
}

#[test]
fn grid_monitor_stops_my_creature_spell_but_not_the_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grid_monitor()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(0, &[gray_ogre()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mine = in_hand(&e, p0, gray_ogre()).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority")
    };
    assert!(!legal.castable.contains(&mine));
    assert!(e.apply(p0, PlayerAction::CastSpell { card: mine }).is_err());

    reach_their_main_phase(&mut e, p1);
    cast_from_hand(&mut e, p1, gray_ogre());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p1, gray_ogre()).is_some());
}
