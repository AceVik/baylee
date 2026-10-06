//! `cards/enchantments/mv_2/underworld_breach.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Underworld Breach: the sentence it keeps is the one that takes it away.
///
/// Escape is refused by name — no `Modifier` grants a casting permission out
/// of a graveyard with its own cost — so what is left is "at the beginning of
/// the end step, sacrifice this enchantment", and a card that did nothing at
/// all would sit on the battlefield forever.
#[test]
fn underworld_breach_sacrifices_itself_at_the_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(407, forest())
        .battlefield(0, &[underworld_breach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, underworld_breach()).is_some(),
        "it is on the battlefield during the main phase"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, underworld_breach()).is_none(),
        "and it sacrifices itself at the beginning of the end step"
    );
    assert!(
        in_graveyard(&engine, p0, underworld_breach()).is_some(),
        "a sacrifice puts it in its owner's graveyard"
    );
}
