//! `cards/creatures/mv_3/coastal_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Coastal Drake is a {2}{U} 2/1 Drake with flying whose whole text is one
/// line: "{1}{U}, {T}: Return target Kavu to its owner's hand." The word that
/// needs a witness is "Kavu", and this pool prints none — but Mutavault
/// animated by its own {1} ability is a 2/2 with *all* creature types, so it
/// is one, while the Elf beside it is a creature and no Kavu. So the ability
/// has to take the first and decline the second, tap the Drake for its {T},
/// spend exactly {1}{U}, and leave the land in its owner's hand and the Elf
/// where it stood.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn coastal_drake_bounces_an_animated_mutavault_and_declines_the_elf_beside_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                mutavault(),
                coastal_drake(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drake = on_battlefield(&engine, p0, coastal_drake()).expect("the Drake is on the table");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("an Elf stands beside it");
    let vault = on_battlefield(&engine, p0, mutavault()).expect("the Mutavault is out");
    assert_eq!(pt(&engine, drake), (2, 1), "the printed 2/1 body");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "and the printed flying"
    );

    // `{1}{U}, {T}` is read off the *pool*, so the mana is floated before
    // anything is claimed about the offer. The Mutavault is the one source
    // kept back: its own animation is the next thing this test does, and
    // tapping it for {C} would make the pool a claim about a land that has
    // already been spent. The Drake prints no mana ability at all, so it
    // stands either way.
    tap_all_mana_but(&mut engine, p0, Some(mutavault()));
    let floating = engine.state().players[0].mana_pool.total();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        5,
        "five Islands, five blue — the Elf beside them is a mana source too, \
         which is why the pool is counted as a whole below"
    );

    // Mutavault's animation is the only Kavu this pool can put on a
    // battlefield, and `Filter::HasSubtype(Kavu)` is what reads "all creature
    // types". Ability 0 is its `{T}: Add {C}`, ability 1 the animation.
    activate(&mut engine, p0, mutavault(), 1);
    // The animation is an ordinary activated ability and uses the stack
    // (CR 605.1a makes the exception a *mana* ability, which this is not),
    // so the land is still a land until it resolves.
    assert!(
        !types(&engine, vault).contains(TypeSet::CREATURE),
        "announced and not yet resolved: the land is still only a land"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        types(&engine, vault).contains(TypeSet::CREATURE),
        "the land is a creature now"
    );

    activate(&mut engine, p0, coastal_drake(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target Kavu\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&vault),
        "a creature with all creature types is a Kavu: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a creature and no Kavu — the filter is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&drake),
        "a Drake is not a Kavu either: {options:?}"
    );

    // CR 601.2c before CR 601.2h: the target is named while the Drake is
    // still untapped, so the tap is read after the answer.
    assert!(
        !is_tapped(&engine, drake),
        "the tap is the last step of the activation, not the first"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![vault],
            },
        )
        .expect("the animated land was one of the options");
    assert!(
        is_tapped(&engine, drake),
        "{{T}} is paid once the target is chosen"
    );
    assert!(!stack_is_empty(&engine), "and bouncing is no mana ability");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, mutavault()).is_some(),
        "\"return target Kavu to its owner's hand\""
    );
    assert!(
        on_battlefield(&engine, p0, mutavault()).is_none(),
        "the land left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature nobody named never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating - 3,
        "{{1}} for the animation and {{1}}{{U}} for the ability: three mana \
         and not one more"
    );
}
