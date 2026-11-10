//! `cards/artifacts/mv_1/obelisk_of_undoing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Obelisk of Undoing — {1} artifact: "{6}, {T}: Return target permanent you
/// both own and control to your hand."
///
/// The two filters are the whole card and each needs its own bystander: an Elf
/// under the same seat and an Elf across the table are both permanents a bare
/// "target permanent" would reach, so an offer holding exactly the first is
/// what reads `OwnedByYou` and `ControlledByYou` rather than `Filter::Any`.
/// The {6} is a real payment — the Elf is kept back, so the pool the ability
/// empties is the six the six Forests actually filled — and the permanent
/// lands in its *owner's* hand while the Elf across the table never moves.
#[test]
fn obelisk_of_undoing_returns_the_permanent_you_own_and_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                obelisk_of_undoing(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let obelisk = on_battlefield(&engine, p0, obelisk_of_undoing()).expect("the Obelisk is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // The offer is read off the *pool*, so with nothing floating the {6} is
    // unpayable and the line is not there at all — the half a test that only
    // ever taps first would never see.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Obelisk holds it");
    assert!(
        !legal.abilities.contains(&(obelisk, 0)),
        "{{6}} is not six, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Six Forests, and the Elf kept back: it is the permanent the ability is
    // about to return, and a mana creature tapped for the cost would make
    // "exactly six" a count of seven.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests in the pool, and the Elf contributed nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(obelisk, 0)),
        "with six floating and the Obelisk untapped, its one line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, obelisk_of_undoing(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent you both own and control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&mine),
        "the Elf under my own control is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an Elf across the table is a permanent and still no legal target — \
         `ControlledByYou` is what declines it: {options:?}"
    );

    // CR 601.2c before CR 601.2h: the target is named while the mana is
    // still floating and the Obelisk still untapped, so both prices are read
    // after the answer.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    assert!(
        is_tapped(&engine, obelisk),
        "{{T}} is the other half of the cost, paid by the Obelisk itself"
    );
    assert!(!stack_is_empty(&engine), "and it is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "and it is in its *owner's* hand — one card, not a copy in each"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf the ability did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{6}} it charged came out of the pool"
    );
}
