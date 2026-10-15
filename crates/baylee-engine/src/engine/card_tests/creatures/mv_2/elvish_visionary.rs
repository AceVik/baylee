//! Oracle: “When this creature enters, draw a card.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn elvish_visionary() -> CardIndex {
    card_index("c6a3a882-a127-4590-93d7-679ef4313efe")
}

/// Checklist: actually cast the creature; entry draws exactly one, while an
/// unrelated creature entering does not draw for this card.
#[test]
fn elvish_visionary_draws_on_its_entry_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[elvish_visionary()])
        .hand(1, &[llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let before = library_size(&game, p0);
    cast_from_hand(&mut game, p0, elvish_visionary());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(library_size(&game, p0), before - 1);
    assert!(on_battlefield(&game, p0, elvish_visionary()).is_some());

    reach_their_main_phase(&mut game, p1);
    let before_unrelated_entry = library_size(&game, p0);
    cast_from_hand(&mut game, p1, llanowar_elves());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(library_size(&game, p0), before_unrelated_entry);
    assert!(on_battlefield(&game, p1, llanowar_elves()).is_some());
}
