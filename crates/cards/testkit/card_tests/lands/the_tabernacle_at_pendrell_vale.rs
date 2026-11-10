//! `cards/lands/the_tabernacle_at_pendrell_vale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Tabernacle at Pendrell Vale: "All creatures have 'At the beginning of your upkeep, destroy this creature unless you pay {1}.'"
/// The static ability grants an upkeep trigger to every creature on the battlefield.
/// On turn 1 upkeep, Llanowar Elves' granted trigger resolves; when declining to pay {1}, the creature is destroyed.
#[test]
fn the_tabernacle_at_pendrell_vale_taxes_creatures_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(118, forest())
        .battlefield(0, &[the_tabernacle_at_pendrell_vale(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until matched PayTax");
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 1 });

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "creature destroyed"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "creature in graveyard"
    );
    // Which object died, said out loud. The sentence is granted to the
    // creature, so `TargetSpec::ThisObject` is the creature and never the
    // land that granted it — and a reading that took the granting permanent
    // instead produced the same visible outcome as the broken one, because
    // both destroyed nothing (#147).
    assert!(
        on_battlefield(&engine, p0, the_tabernacle_at_pendrell_vale()).is_some(),
        "the land that granted the ability destroyed itself"
    );
}
