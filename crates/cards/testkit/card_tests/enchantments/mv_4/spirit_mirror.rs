//! `cards/enchantments/mv_4/spirit_mirror.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spirit_mirror() -> CardIndex {
    card_index("42e041c0-ffbc-4bf8-a5aa-f0d14aad7112")
}

/// "At the beginning of your upkeep, if there are no Reflection tokens on the
/// battlefield, create a 2/2 white Reflection creature token": the first
/// upkeep makes one, and the next upkeep, with it still out, makes no second.
#[test]
fn spirit_mirror_makes_a_reflection_only_while_there_is_none() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spirit_mirror()])
        .start();
    keep_mulligans(&mut engine);

    walk_to_step_trigger(&mut engine, p0, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Reflection");
    assert_eq!(pt(&engine, tokens[0]), (2, 2), "a 2/2");

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.number > turn
            && e.state().turn.step == Step::Draw
    });
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "a Reflection is still out, so the second upkeep made none"
    );
}

/// "{0}: Destroy target Reflection": the token dies, and the next upkeep
/// makes a fresh one.
#[test]
fn spirit_mirror_destroys_a_reflection_for_nothing_and_remakes_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spirit_mirror()])
        .start();
    keep_mulligans(&mut engine);
    walk_to_step_trigger(&mut engine, p0, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    let token = tokens_of(&engine, p0)[0];

    activate(&mut engine, p0, spirit_mirror(), 1);
    aim_at(&mut engine, p0, token);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Reflection is destroyed"
    );

    walk_to_step_trigger(&mut engine, p0, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0).len(), 1, "a new Reflection");
}
