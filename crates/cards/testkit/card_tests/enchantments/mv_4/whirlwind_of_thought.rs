//! `cards/enchantments/mv_4/whirlwind_of_thought.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn whirlwind_of_thought() -> CardIndex {
    card_index("6467cbb7-1e4e-482d-a20f-6cb9fc0f1ad1")
}

/// "Whenever you cast a noncreature spell, draw a card": Sol Ring draws one
/// (the library shrinks by one), a creature spell draws none.
#[test]
fn whirlwind_of_thought_draws_for_noncreature_spells_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[whirlwind_of_thought()])
        .hand(0, &[grizzly_bears(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library = library_size(&engine, p0);
    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library,
        "a creature spell draws nothing"
    );

    float(&mut engine, p0, ManaColor::Green, 1);
    cast_with_floating(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1, "one card drawn");
}

/// An opponent's noncreature spell is not "you cast".
#[test]
fn whirlwind_of_thought_ignores_the_opponents_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[whirlwind_of_thought()])
        .hand(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let library = library_size(&engine, p0);
    float(&mut engine, p1, ManaColor::Green, 1);
    cast_with_floating(&mut engine, p1, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library, "I draw nothing");
}
