//! `cards/instants/mv_2/naturalize.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Naturalize — {1}{G} instant: "Destroy target artifact or enchantment."
///
/// Three words carry the card and the board gives each of them a witness: a
/// Sol Ring under the caster, a Sol Ring and an enchantment across the table,
/// and a creature and two lands that "artifact or enchantment" must decline —
/// so the offer read while the spell is on the stack is a filter and not the
/// whole board. The enchantment is the one that dies, which is the half an
/// artifact-only reading would have skipped, and the permanents beside it are
/// what say one named target was destroyed rather than a sweep.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn naturalize_destroys_the_artifact_or_enchantment_its_caster_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), quiet_artifact()])
        .hand(0, &[naturalize()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my artifact is out");
    let my_land = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let doomed =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Mana before the claim: `legal.castable` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands. The Sol
    // Ring is named as the thing kept back so that "two" is the two Forests
    // and nothing else.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Forests, and the artifact left standing"
    );
    cast_with_floating(&mut engine, p0, naturalize());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact or enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster is the one that chooses");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&mine),
        "\"target artifact\" reaches this seat's own board too: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and the artifact across the table: {options:?}"
    );
    assert!(
        options.contains(&doomed),
        "and the enchantment beside it — the second word of the filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );
    assert!(
        !options.contains(&my_land),
        "nor is a land, whatever an unfiltered target spec would have offered: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    // Still in **hand**, and that is this engine's announcement rather than a
    // bug: `cast_wizard` asks every question CR 601.2b–h poses and moves the
    // card to the stack last, at CR 601.2i, so the whole announcement is
    // atomic from the outside. Nobody can tell: no player gets priority
    // until 601.2i, and CR 115.5 makes a spell an illegal target for
    // itself, so there is no legal question whose answer differs.
    assert!(
        in_hand(&engine, p0, naturalize()).is_some(),
        "the card has not reached the stack yet (CR 601.2i comes last)"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "and nothing has been destroyed yet — the removal happens on resolution"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the enchantment was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it is in its owner's graveyard, not merely gone"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "nor did this seat's own artifact"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor the creature that was never a legal target"
    );
    assert!(
        in_graveyard(&engine, p0, naturalize()).is_some(),
        "and the instant itself went to its owner's graveyard as it resolved"
    );
}
