//! `cards/instants/mv_2/disenchant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Disenchant — {1}{W} instant: "Destroy target artifact or enchantment."
///
/// The offer is the half a file reading cannot see: `Filter::ARTIFACT_OR_ENCHANTMENT` has to
/// name both an artifact and an enchantment across the table and decline the Elf standing
/// beside them, which is what says the two words are one filter rather than "target
/// permanent". Only the artifact is then named, so the enchantment is the bystander that
/// proves one target died and not the offer's whole menu — and the caster's own graveyard is
/// where the resolved instant has to land.
#[test]
fn disenchant_destroys_the_artifact_it_names_and_leaves_the_rest_of_the_table_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[disenchant()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let artifact = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let enchantment =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Two Plains into the pool first: whether a {1}{W} spell is castable is
    // read off the pool and not off the lands that are still standing.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, disenchant());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Disenchant targets, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&artifact),
        "an artifact is a legal target: {options:?}"
    );
    assert!(
        options.contains(&enchantment),
        "\"or enchantment\" reads both halves of the filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .expect("the artifact was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact the spell named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and a destroyed permanent goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "the enchantment it did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature the filter declined"
    );
    assert!(
        in_graveyard(&engine, p0, disenchant()).is_some(),
        "the resolved instant is in its caster's graveyard"
    );
}
