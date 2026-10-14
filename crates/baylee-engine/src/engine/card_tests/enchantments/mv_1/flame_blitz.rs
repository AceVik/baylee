//! `cards/enchantments/mv_1/flame_blitz.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn flame_blitz() -> CardIndex {
    card_index("817eb91a-e3b4-40c3-9a6e-e4d0387e0f4f")
}

/// "At the beginning of your end step, this enchantment deals 5 damage to each
/// planeswalker."
#[test]
fn flame_blitz_burns_every_planeswalker_at_my_end_step() {
    let mut e = table(
        7291,
        &[oko_thief_of_crowns()],
        &[flame_blitz(), forest(), island()],
        &[],
    );
    cast_with_floating(&mut e, P0, oko_thief_of_crowns());
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, oko_thief_of_crowns()).is_some(),
        "alive in main"
    );
    pass_until(&mut e, |e| {
        e.state().turn.phase == crate::turn::Phase::Ending
    });
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, oko_thief_of_crowns()).is_none(),
        "five damage kills a four-loyalty planeswalker"
    );
    assert!(in_graveyard(&e, P0, oko_thief_of_crowns()).is_some());
}

/// "Cycling {2}": from hand, pay {2} and discard to draw a card.
#[test]
fn flame_blitz_cycles_from_hand() {
    let mut e = table(7292, &[flame_blitz()], &[], &[]);
    let hand = hand_size(&e, P0);
    let library = library_size(&e, P0);
    activate(&mut e, P0, flame_blitz(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, flame_blitz()).is_some());
    assert_eq!(hand_size(&e, P0), hand, "one out, one in");
    assert_eq!(library_size(&e, P0), library - 1);
}
