//! `cards/enchantments/mv_4/coastal_piracy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn coastal_piracy() -> CardIndex {
    card_index("8a05ec32-7b0c-4f23-a4f7-413301c2a70a")
}

/// An unblocked Bears of mine, walked to the "may draw" question.
fn a_piracy_asking() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[coastal_piracy(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    attack_with(&mut engine, bears, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// "Whenever a creature you control deals combat damage to an opponent, you
/// may draw a card": asked of me after the damage, and yes draws.
#[test]
fn coastal_piracy_draws_when_my_creature_connects() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_piracy_asking();
    assert_eq!(life_of(&engine, p1), 18, "the damage came first");
    let library = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1, "one card drawn");
}

/// "May": a no draws nothing.
#[test]
fn coastal_piracy_declined_draws_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = a_piracy_asking();
    let library = library_size(&engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library, "declined: no draw");
}
