//! Leveler's enters trigger exiles the whole library.

use super::*;

fn leveler() -> CardIndex {
    card_index("ba1d34ab-7529-4f47-a5e7-7d8f66d7b901")
}

#[test]
fn leveler_exiles_every_card_from_the_library() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 5])
        .hand(0, &[leveler()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let before = library_size(&e, p0);
    cast_from_hand(&mut e, p0, leveler());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, p0), 0);
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(p0)).len(), before);
    assert!(on_battlefield(&e, p0, leveler()).is_some());
}
