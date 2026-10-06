//! `cards/instants/mv_4/hagra_mauling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hagra Mauling // Hagra Broodpit (`Coverage::Partial`): "This spell costs
/// {1} less to cast if an opponent controls no basic lands. Destroy target
/// creature. // This land enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Partial`, the conditional cost reduction is omitted,
/// but the creature destruction is implemented in full. The test casts Hagra
/// Mauling for its printed cost of `{2}{B}{B}`, targets an opponent's creature,
/// and confirms that the creature is destroyed upon resolution.
#[test]
fn hagra_mauling_destroys_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(49, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hagra_mauling()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's Elf");

    cast_from_hand(&mut engine, p0, hagra_mauling());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "target creature — the Elf qualifies"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted creature is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the destroyed creature is in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, hagra_mauling()).is_some(),
        "the resolved spell is in its caster's graveyard"
    );
}
