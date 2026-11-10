//! `cards/instants/mv_3/smash.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smash is a {2}{R} instant under `Coverage::Implemented` that destroys target artifact and draws a card.
/// When cast targeting an artifact controlled across the table, the target is destroyed and sent to the graveyard.
/// The resolving spell draws a card for its controller, verified by hand and library size changes.
/// An artifact is required as a target, leaving creatures excluded from the offered options.
#[test]
fn smash_destroys_target_artifact_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[smash()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, smash()).expect("Smash is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{R}}, so the instant is not offered: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains produce three mana"
    );
    cast_with_floating(&mut engine, p0, smash());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(player, p0, "the casting seat selects the target");
    assert_eq!((min, max), (1, 1), "one target artifact is required");
    assert!(
        options.contains(&rock),
        "the artifact is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the creature is not an artifact: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target artifact selected");

    // Counted once the target is chosen and Smash is on the stack: while the
    // question stood, the spell being cast was still counted in the hand.
    let lib_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the targeted artifact is destroyed and in graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact is no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the bystander creature is untouched"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "one card was drawn from library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the drawn card reached the hand"
    );
    assert!(
        in_graveyard(&engine, p0, smash()).is_some(),
        "the instant resolved and went to graveyard"
    );
}
