//! `cards/creatures/mv_2/humble_budoka.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Humble Budoka — {1}{G} 2/2 Human Monk with shroud.
///
/// Shroud carries no "your opponents control" (CR 702.18a), so the spell that
/// proves it is one of the controller's *own*: a Giant Growth aimed at the
/// board has to offer the Llanowar Elves beside it and must not offer the
/// Budoka. Both readings together are the card — a filter that only declined
/// opponents' spells, or one that declined nothing, would show up as the
/// Budoka sitting in the target list, and a shroud that had cost the printed
/// body would still pass the targeting half. The pump is then allowed to
/// resolve on the Elf, so the pair of projected bodies says the two creatures
/// really are different permanents to the engine.
#[test]
fn humble_budoka_is_shrouded_even_from_its_controllers_own_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[humble_budoka(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Forests, and the Elf kept standing: it is the control the offer
    // below is read against, so it must be an untapped 1/1 when the question
    // is asked. `tap_all_mana_but` names it as the printing to leave alone.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and the Elf paid nothing"
    );

    // {1}{G} out of the open pool, leaving the {G} the Growth needs beside it
    // — CR 500.5 keeps a pool until the step ends and the whole scenario lives
    // in this one main phase.
    cast_with_floating(&mut engine, p0, humble_budoka());
    pass_until(&mut engine, stack_is_empty);
    let budoka = on_battlefield(&engine, p0, humble_budoka()).expect("the Budoka resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert_eq!(pt(&engine, budoka), (2, 2), "the printed body");
    assert!(
        keywords(&engine, budoka).contains(KeywordSet::SHROUD),
        "shroud is the card's whole rules text"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "two of the three green paid the {{1}}{{G}} and one is left floating"
    );

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
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert!(
        options.contains(&elves),
        "a creature without shroud is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&budoka),
        "CR 702.18a: shroud declines its own controller's spell too, so the \
         Budoka is not a legal target for Giant Growth: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the Elf is the whole of the menu on this board: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elves),
        (4, 4),
        "the +3/+3 landed on the creature that was named"
    );
    assert_eq!(
        pt(&engine, budoka),
        (2, 2),
        "and the shrouded one is untouched, because it was never a target"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Growth's {{G}} came out of the pool"
    );
}
