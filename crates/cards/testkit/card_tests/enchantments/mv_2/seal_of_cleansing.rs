//! `cards/enchantments/mv_2/seal_of_cleansing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seal of Cleansing — {1}{W} enchantment: "Sacrifice this enchantment:
/// Destroy target artifact or enchantment."
///
/// The whole price is the Seal itself and no mana at all, so the ability is
/// offered on a board with an empty pool — `LegalActions::abilities` is
/// filtered through `can_afford`, and a cost that names no mana is payable
/// from a pool holding none. The target menu is the card's filter read on a
/// real board: artifacts on *either* side of the table and an enchantment
/// across it, but neither the Elf beside the Seal nor the Forest it stands
/// next to, which is what separates "target artifact or enchantment" from
/// "target permanent". And the sacrifice is read after the target: targets
/// are chosen at CR 601.2c and costs paid at CR 601.2h, so while the question
/// stands the Seal is still on the battlefield and only afterwards is it in
/// its owner's graveyard.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn seal_of_cleansing_sacrifices_itself_to_destroy_an_artifact_or_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                seal_of_cleansing(),
                quiet_artifact(),
                quiet_creature(),
                forest(),
            ],
        )
        .battlefield(1, &[quiet_artifact(), their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let seal = on_battlefield(&engine, p0, seal_of_cleansing()).expect("the Seal is out");
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let aura = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");

    // Nothing needs tapping: the price is the Seal. That is exactly why the
    // offer may be read here with an empty pool — and it is asserted first,
    // so a missing line could not be blamed on mana that was never floated.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing was tapped for this scenario and nothing needs to be"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(seal, 0)),
        "the one line the Seal prints costs only itself, so it is offered on \
         an empty pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, seal_of_cleansing(), 0);
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
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one target");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" reaches artifacts on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&aura),
        "and an enchantment across the table is the other half of the \
         filter: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is neither an artifact nor an enchantment: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "nor is a land, so neither may be offered: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, seal_of_cleansing()).is_some(),
        "CR 601.2c before CR 601.2h: the sacrifice has not happened while the \
         target question is still open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![aura],
            },
        )
        .expect("the enchantment was one of the options it enumerated");

    assert!(
        in_graveyard(&engine, p0, seal_of_cleansing()).is_some(),
        "\"Sacrifice this enchantment\" is the last thing paid, and it takes \
         the Seal with it"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "the targeted enchantment is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and only the permanent that was named: the artifact across the table \
         still stands"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "as does this seat's own — one target, one destroyed permanent"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "the creature the filter declined never moved"
    );
}
