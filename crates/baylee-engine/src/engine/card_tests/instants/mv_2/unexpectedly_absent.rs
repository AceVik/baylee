//! `cards/instants/mv_2/unexpectedly_absent.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("e8d78a83-c932-4b55-8f75-7094c672c3a9")
}

#[test]
fn unexpectedly_absent_puts_the_target_beneath_exactly_x_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 4])
        .hand(0, &[subject()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let target = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    let old_library = e.state().zones.list(ZoneLocation::Library(p1)).len();
    cast_with_floating(&mut e, p0, subject());
    e.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    aim_at(&mut e, p0, target);
    pass_until(&mut e, stack_is_empty);
    let library = e.state().zones.list(ZoneLocation::Library(p1));
    assert_eq!(library.len(), old_library + 1);
    assert_eq!(
        library[library.len() - 3],
        target,
        "two cards remain above it"
    );
    assert!(on_battlefield(&e, p1, llanowar_elves()).is_none());
}
