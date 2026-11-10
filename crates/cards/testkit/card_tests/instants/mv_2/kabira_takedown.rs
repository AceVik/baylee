//! `cards/instants/mv_2/kabira_takedown.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kabira Takedown // Kabira Plateau (`Coverage::Implemented`): "Kabira
/// Takedown deals damage equal to the number of creatures you control to
/// target creature or planeswalker."
///
/// p0 controls two creatures when they cast the spell, so the target takes
/// 2 damage. The opponent's creature (not controlled by p0) must not be
/// counted in the tally — "you control" is the filter. A 1/1 target survives
/// the first hit but dies at 2, which confirms the arithmetic is what the
/// board says it is.
#[test]
fn kabira_takedown_deals_damage_equal_to_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[kabira_takedown()])
        // The opponent's creature must not count toward p0's total.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");

    // p0 controls two Llanowar Elves, so the spell deals exactly 2 damage.
    cast_from_hand(&mut engine, p0, kabira_takedown());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Takedown asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "target creature or planeswalker — the Elf qualifies: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "2 damage on a 1/1 is lethal — the Elf is gone from the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a destroyed creature goes to its owner's graveyard"
    );
}
