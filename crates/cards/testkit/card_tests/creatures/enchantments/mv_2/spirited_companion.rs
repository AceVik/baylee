//! Oracle: “When this creature enters, draw a card.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn spirited_companion() -> CardIndex {
    card_index("9c5f0d91-9d86-4e66-94fd-4af93ad01838")
}

/// Checklist: cast and resolve this creature; its entry draws exactly one.
/// Merely having it on the battlefield at setup does not count as an entry.
#[test]
fn spirited_companion_draws_once_when_cast_and_resolved() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[spirited_companion()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let library_before = library_size(&game, p0);
    cast_from_hand(&mut game, p0, spirited_companion());
    pass_until(&mut game, stack_is_empty);

    assert!(on_battlefield(&game, p0, spirited_companion()).is_some());
    assert_eq!(library_size(&game, p0), library_before - 1);
    assert!(in_graveyard(&game, p0, spirited_companion()).is_none());
}
