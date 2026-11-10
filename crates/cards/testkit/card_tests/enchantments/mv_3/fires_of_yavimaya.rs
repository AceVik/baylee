//! `cards/enchantments/mv_3/fires_of_yavimaya.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fires of Yavimaya — {1}{R}{G} enchantment: "Creatures you control have
/// haste" and "Sacrifice this enchantment: Target creature gets +2/+2 until
/// end of turn." One board reads both printed lines: an Elf under the same seat
/// has the keyword the moment the enchantment lands while the Elf across the
/// table does not, which is the whole of "you control"; and the sacrifice is
/// then played out as the printed price it is, with the offer for the pump
/// naming both Elves — "target creature" is not "target creature you control".
#[test]
fn fires_of_yavimaya_grants_haste_and_sacrifices_itself_for_a_pump() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fires_of_yavimaya()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "a printed Llanowar Elves carries no keyword of its own"
    );

    // {1}{R}{G} off the two Forests and the Mountain, with the Elf named as
    // the source kept back: it is the creature the static is read on, and a
    // source tapped for mana is a creature the rest of this test would be
    // reading in a state nobody asked for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and a Mountain pay {{1}}{{R}}{{G}} exactly"
    );
    cast_with_floating(&mut engine, p0, fires_of_yavimaya());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, fires_of_yavimaya()).is_some(),
        "the enchantment resolved onto the table"
    );

    // "Creatures you control have haste", read on both sides of the table so
    // that the filter is what the assertion is about and not the board.
    assert!(
        keywords(&engine, mine).contains(KeywordSet::HASTE),
        "a creature you control gets the granted haste"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "\"creatures *you* control\": the Elf across the table is untouched"
    );

    // The whole price of the second line is the enchantment itself, so
    // nothing has to float — an offer read off an empty pool is the price
    // being `SacrificeSelf` and no mana.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pump charges no mana at all"
    );
    activate(&mut engine, p0, fires_of_yavimaya(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, fires_of_yavimaya()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h)"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is the one that gets it");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fires_of_yavimaya()).is_some(),
        "\"Sacrifice this enchantment\" is the cost, and a sacrificed \
         permanent goes to its owner's graveyard"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "and the static left with its source: nothing grants haste now"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "+2/+2 on the creature that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the Elf the ability did not name never moved"
    );
}
