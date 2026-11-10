//! `cards/instants/mv_3/regress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "2fa763ad-4d94-4c3a-a099-13ac22c09ce4"

/// Regress — {2}{U} instant: "Return target permanent to its owner's hand."
///
/// Two printed words are the whole card and each needs its own witness. The
/// target says *permanent* and not creature, so lands stand on the menu beside
/// the creatures; and the destination says *owner's* hand, so the spell is
/// aimed across the table and the Elf has to come back to the seat that owns
/// it rather than to the seat that cast the spell. My own Elf is the control
/// that the return is the named target and not a sweep of the board, and the
/// three Islands are read as an exactly emptied pool so the {2}{U} is a
/// payment rather than a label.
#[test]
fn regress_returns_the_permanent_it_names_to_its_owners_hand_and_not_to_the_casters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[regress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let my_land = on_battlefield(&engine, p0, island()).expect("my Island is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Three Islands are the whole of {2}{U}, and the Elf is named as the
    // printing kept back: it is a creature this test reads off the board
    // afterwards, and a mana creature tapped for the spell would be a
    // different board than the one the assertions below describe.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands in the pool, and the Elf contributed nothing"
    );

    cast_with_floating(&mut engine, p0, regress());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p0, "the seat that cast the spell names the target");
    assert!(
        options.contains(&my_elf) && options.contains(&their_elf),
        "\"target permanent\" is any permanent, on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&my_land) && options.contains(&their_land),
        "\"target permanent\" is not \"target creature\": lands are on the \
         menu too: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while the Elf it \
         names is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the creature across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and not to the caster's hand, which is the reading the printed word \
         forbids — the two differ only by which side of the table the card \
         lands on"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}} came out of the three Islands: one spell, and nothing \
         left floating"
    );
}
