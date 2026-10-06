//! `cards/creatures/mv_4/tuknir_deathlock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tuknir Deathlock — {R}{R}{G}{G} — a legendary 2/2 Human Wizard with flying
/// whose whole text is "{R}{G}, {T}: Target creature gets +2/+2 until end of
/// turn."
///
/// Three Forests and three Mountains pay the cast and leave exactly the
/// {R}{G} the ability charges, so the offer is read the way the engine reads
/// it — out of the pool — and the pump afterwards can only have been bought
/// with that mana, while the tap symbol is read on Tuknir itself and the
/// still-fresh pool during the target question says the cost is paid last
/// (CR 601.2c, then CR 601.2h). The Elf across the table is the control the
/// effect needs: "target creature" offers it and still leaves it a printed
/// 1/1, and by the opponent's next main phase the pumped Elf is a 1/1 again,
/// which is the "until end of turn" the card prints.
#[test]
#[allow(clippy::too_many_lines)]
fn tuknir_deathlock_taps_and_spends_red_green_on_its_pump_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                forest(),
                llanowar_elves(),
                // On the battlefield from the start: the ability's {T} can be
                // paid only by a creature its controller has held since the
                // turn began (CR 302.6).
                tuknir_deathlock(),
            ],
        )
        // A creature across the table, so "target creature" is read as any
        // creature rather than as the activating seat's own.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before anything");

    // Two lands, and the Elf named as the printing kept back: it prints its
    // own `{T}: Add {G}` and `tap_all_mana` would have drunk it, which is both
    // a pool this test never accounted for and a creature the pump below is
    // about to change.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let tuknir = on_battlefield(&engine, p0, tuknir_deathlock()).expect("Tuknir is out");
    assert_eq!(pt(&engine, tuknir), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, tuknir).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert!(!is_tapped(&engine, tuknir), "and it is untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "a Mountain and a Forest, exactly the {{R}}{{G}} the ability charges, \
         and the Elf contributed nothing"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands — which is why the claim
    // is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tuknir, 0)),
        "the one line the card prints, now that its price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, tuknir_deathlock(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&elf) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // halves of the price are still unpaid while this question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cost is the last step of the activation, so nothing is spent yet"
    );
    assert!(
        !is_tapped(&engine, tuknir),
        "and nothing has tapped Tuknir yet"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");

    assert!(is_tapped(&engine, tuknir), "{{T}} is half of the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{R}}{{G}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "pumping a creature is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "+2/+2 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it did not: the effect is pointed, \
         not a board sweep"
    );
    assert_eq!(
        pt(&engine, tuknir),
        (2, 2),
        "nor for Tuknir itself, which was on the menu and was not chosen"
    );

    // "until end of turn": one turn later the creature is still standing and
    // the two counters are not — a static or a permanent grant would still be
    // on it here.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the pumped creature is still on the battlefield a turn later, so what \
         changed is the pump and not the creature"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the +2/+2 lasted the turn it was made in and no longer"
    );
}
