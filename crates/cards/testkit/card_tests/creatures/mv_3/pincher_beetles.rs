//! `cards/creatures/mv_3/pincher_beetles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pincher Beetles prints Shroud and nothing else (CR 702.18: "This creature
/// can't be the target of spells or abilities"), so the whole card is a
/// targeting restriction and it cuts both ways. One board is asked both
/// halves: the Beetles' own controller cannot point Giant Growth at it, and
/// the opponent cannot point Swords to Plowshares at it, while the plain
/// Llanowar Elves beside it is offered to both spells — which rules out "there
/// was nothing to target" as the reason a menu looks short. What is read is
/// the offer itself, because shroud never refuses a spell; it declines to put
/// the permanent on the list, and a spell whose only candidate was shrouded
/// would not even be castable.
#[test]
fn pincher_beetles_shroud_keeps_it_off_its_own_controllers_and_its_opponents_target_menus() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), pincher_beetles(), llanowar_elves()])
        .hand(0, &[giant_growth()])
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beetles = on_battlefield(&engine, p0, pincher_beetles()).expect("the Beetles is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert!(
        keywords(&engine, beetles).contains(KeywordSet::SHROUD),
        "shroud is what the layers hand the permanent, got {:?}",
        keywords(&engine, beetles)
    );

    // The offer is read off the pool, so the mana is floated first — and the
    // Elf is a mana source of its own, which is why this is two green and not
    // the Forest's one.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "the Forest and the Llanowar Elves are both mana sources here"
    );

    // Half one: shroud is not "your opponents' spells". Giant Growth's own
    // controller is the one casting it, and the Beetles is still not offered.
    cast_with_floating(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Giant Growth targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "the Elf beside it has no shroud, so it is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&beetles),
        "\"can't be the target of spells\" is its own controller's spell as \
         much as anyone's: {options:?}"
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

    // Half two: the opponent's targeted removal, which is the whole reason the
    // keyword is printed on a 3/1.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Swords to Plowshares targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "\"target creature\" reaches across the table, so the Elf is offered: {options:?}"
    );
    assert!(
        !options.contains(&beetles),
        "and the shrouded one is not — which is the whole of what the card \
         prints: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, pincher_beetles()).is_some(),
        "nothing ever named the Beetles, so it is still standing"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "while the Elf that was named is gone: both spells really resolved \
         rather than being refused for want of a target"
    );
}
