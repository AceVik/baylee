//! `cards/creatures/mv_3/alpha_kavu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alpha Kavu — {2}{G} 2/2 Kavu: "{1}{G}: Target **Kavu** creature gets
/// -1/+1 until end of turn." The subtype is the whole of the filter, so the
/// board carries three creatures and only two of them are on the menu: the
/// Kavu under this seat and the one across the table are both "target Kavu
/// creature", while the Llanowar Elves beside them must be declined. The pump
/// is read as a *body* — a printed 2/2 becomes a 1/3, which no other reading
/// of "-1/+1" produces — and it lands on the creature that was named while the
/// Elf and the other Kavu keep their printed numbers.
#[test]
fn alpha_kavu_pumps_only_a_kavu_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), alpha_kavu(), llanowar_elves()])
        .battlefield(1, &[alpha_kavu()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, alpha_kavu()).expect("my Kavu is out");
    let theirs = on_battlefield(&engine, p1, alpha_kavu()).expect("their Kavu is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "a printed 2/2 before anything is asked"
    );

    // Mana first: the offer is read off the pool and not off untapped lands.
    // Two Forests and the Elf's own printed `{T}: Add {G}` are three sources.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Forests and one Elf tapped for it"
    );

    activate(&mut engine, p0, alpha_kavu(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target Kavu creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "a Kavu on either side of the table is \"target Kavu creature\": {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "an Elf is no Kavu, whatever else it is: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    // CR 601.2c before CR 601.2h: the target is named while the mana is still
    // floating and no pump has happened yet.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the activation"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Kavu the question offered was chosen");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{G}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "and it is no mana ability");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, mine),
        (1, 3),
        "-1/+1 on the Kavu that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "the Kavu it did not name never moved"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and neither did the creature that is no Kavu at all"
    );
    assert!(
        !is_tapped(&engine, mine),
        "the printed price is mana, not the Kavu's own {{T}}"
    );
}
