//! Myr Mindservant shuffles its controller's library.

use super::*;

fn myr_mindservant() -> CardIndex {
    card_index("c0ea83ab-f62e-43ee-b7ad-ddbbc8916632")
}

#[test]
fn myr_mindservant_taps_and_shuffles_the_library() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[myr_mindservant(), island(), island()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let before = library_size(&e, p0);
    let events = e.state().journal.entries().len();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, myr_mindservant(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, p0), before);
    assert!(e.state().journal.entries()[events..].iter().any(|entry| {
        matches!(entry.event, crate::event::GameEvent::Shuffled { player, .. } if player == p0)
    }));
    assert!(is_tapped(
        &e,
        on_battlefield(&e, p0, myr_mindservant()).unwrap()
    ));
}
