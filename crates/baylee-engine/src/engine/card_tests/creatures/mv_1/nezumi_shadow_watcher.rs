//! `cards/creatures/mv_1/nezumi_shadow_watcher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nezumi Shadow-Watcher — {B}, 1/1 Rat Warrior: "Sacrifice this creature:
/// Destroy target Ninja."
///
/// Nothing on this board prints Ninja, so the Ninja is one a Mutavault makes
/// of itself: "{1}: Mutavault becomes a 2/2 creature with all creature types
/// until end of turn" is the only line available that yields the subtype the
/// filter names, and it is played on p1's own turn so the animation is legal
/// however the engine reads its timing. The two Llanowar Elves are the
/// control — one on each side of the table, so the target menu is shown to
/// hold the printed filter and not "target creature". The claims are ordered
/// by CR 601.2c against CR 601.2h: while the target question stands the Rat
/// is still on the battlefield, and the sacrifice that pays for the ability
/// follows the answer.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn nezumi_shadow_watcher_sacrifices_itself_to_destroy_the_ninja_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                nezumi_shadow_watcher(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[mutavault(), forest(), forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let rat = on_battlefield(&engine, p0, nezumi_shadow_watcher()).expect("the Watcher is out");
    assert_eq!(pt(&engine, rat), (1, 1), "a printed 1/1");
    let vault = on_battlefield(&engine, p1, mutavault()).expect("the Mutavault is a land");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elves are out");
    assert!(
        !types(&engine, vault).contains(TypeSet::CREATURE),
        "a Mutavault is a land until its own ability says otherwise"
    );

    // The mana first, and then the offer read off it: {1} is the animation's
    // whole price. Tapping everything also spends the land's own
    // "{T}: Add {C}", which leaves exactly one ability on it — the index is
    // taken out of the offer rather than guessed.
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p1 holds priority after making mana, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == vault)
        .expect("the animation is the one ability an already-tapped Mutavault still offers");
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| types(e, vault).contains(TypeSet::CREATURE));
    assert!(
        types(&engine, vault).contains(TypeSet::CREATURE),
        "the land is a creature until end of turn, and with all creature types"
    );

    // p0's own priority in that main phase: the Rat's ability is no sorcery,
    // so nothing about it needs p0's own turn.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, nezumi_shadow_watcher(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target Ninja\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!(
        (min, max),
        (1, 1),
        "one Ninja, and the card names no other target"
    );
    assert!(
        player_options.is_empty(),
        "the spec names objects and no player"
    );
    assert!(
        options.contains(&vault),
        "the animated land is the Ninja the filter asks for: {options:?}"
    );
    assert!(
        !options.contains(&mine) && !options.contains(&theirs),
        "an Elf is no Ninja, whichever side of the table it stands on: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that land is the only Ninja on the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, nezumi_shadow_watcher()).is_some(),
        "CR 601.2h: the sacrifice is paid after the target is named, so the Rat is still here"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![vault],
            },
        )
        .expect("the Ninja the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, nezumi_shadow_watcher()).is_none(),
        "the Rat paid the ability with its own body"
    );
    assert!(
        in_graveyard(&engine, p0, nezumi_shadow_watcher()).is_some(),
        "and the card is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, mutavault()).is_none(),
        "the Ninja it named is destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, mutavault()).is_some(),
        "a destroyed land is a card in its owner's graveyard like any other permanent"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "and the Elf beside it never moved: one target, one destruction"
    );
}
