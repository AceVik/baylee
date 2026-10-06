//! `cards/creatures/mv_1/elvish_lookout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elvish Lookout is a {G} 1/1 Elf whose whole printed text is "Shroud
/// (This creature can't be the target of spells or abilities.)" (CR 702.11),
/// and the card is cast rather than presumed so the keyword has a permanent
/// to land on. It is then aimed at twice by its **own** controller — a Giant
/// Growth that would help it and a Swords to Plowshares that would remove it
/// — because shroud is not the opponent's word: the menu each spell publishes
/// has to decline it either way. The Llanowar Elves beside it stays on both
/// menus, so the exclusion is a missing card and not an empty board, and the
/// Swords is then actually resolved on the Elves, so the missing card is not
/// a spell that was never cast.
#[test]
fn elvish_lookout_is_never_on_the_menu_of_a_spell_that_would_target_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(
            0,
            &[forest(), forest(), plains(), plains(), llanowar_elves()],
        )
        .hand(
            0,
            &[elvish_lookout(), giant_growth(), swords_to_plowshares()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two Forests, two Plains and the Elves' own `{T}: Add {G}` (CR 305.6 and
    // CR 605.1 alike), tapped once because a pool survives until the step ends
    // (CR 500.5) and this whole scenario plays inside one first main phase.
    // Three green and two white pay the {G}, the {G} and the {W}.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "two Forests and the Llanowar Elves beside them"
    );

    cast_with_floating(&mut engine, p0, elvish_lookout());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let lookout = on_battlefield(&engine, p0, elvish_lookout()).expect("the Lookout resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, lookout), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, lookout).contains(KeywordSet::SHROUD),
        "the keyword reaches the permanent"
    );

    // The pump: "target creature" reaches any creature on either side of the
    // table, and the seat casting it is the one that controls the Lookout.
    cast_with_floating(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Giant Growth targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it chooses");
    assert!(
        options.contains(&elves),
        "the Elves are a legal target, so the menu is not empty: {options:?}"
    );
    assert!(
        !options.contains(&lookout),
        "shroud: a spell its own controller cast may not target it \
         (CR 702.11b): {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(pt(&engine, elves), (4, 4), "the Elves took the pump");

    // The removal, off the white still in the pool: the same two creatures on
    // the board and the same answer, which is what "spells" and not
    // "your opponents' spells" means. No life total is asserted here — the
    // four the exile would hand back are the pump's reading and not shroud's.
    cast_with_floating(&mut engine, p0, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves) && !options.contains(&lookout),
        "the same menu, the same refusal: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![lookout],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the removal exiled the creature it was allowed to name, so the spell \
         really cast and resolved"
    );
    assert!(
        on_battlefield(&engine, p0, elvish_lookout()).is_some(),
        "and the Lookout, which was never on either menu, is still standing"
    );
}
