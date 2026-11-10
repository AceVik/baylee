//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn jovial_evil_counts_the_chosen_opponents_white_creatures_on_resolution() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, forest(), 3)
        .hand(0, &[index::JOVIAL_EVIL])
        .battlefield(0, &[index::SAVANNAH_LIONS])
        .battlefield(1, &[index::SAVANNAH_LIONS, index::SAVANNAH_LIONS])
        .battlefield(
            2,
            &[
                index::SAVANNAH_LIONS,
                index::SAVANNAH_LIONS,
                index::SAVANNAH_LIONS,
                quiet_creature(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 3);
    e.refresh_offer();
    cast_from_hand(&mut e, p0, index::JOVIAL_EVIL);
    let Pending::ChoosePlayer { options, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&p0));
    e.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    let lion = on_battlefield(&e, p2, index::SAVANNAH_LIONS).unwrap();
    bury(&mut e, &[lion]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[p2.get() as usize].life, 16);
    assert_eq!(e.state().players[p1.get() as usize].life, 20);
    assert_eq!(e.state().players[0].life, 20);
}
