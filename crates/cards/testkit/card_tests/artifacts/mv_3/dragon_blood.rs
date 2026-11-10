//! `cards/artifacts/mv_3/dragon_blood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dragon Blood is a `{3}` artifact printing one line: "`{3}`, `{T}`: Put a
/// +1/+1 counter on target creature."
///
/// Six Forests pay for both halves inside one main phase (CR 500.5), so the
/// `{3}` the ability charges is read as a real payment out of the pool and not
/// as a label: the offer is claimed only once the mana is already floating,
/// and the target question stands while the artifact is still untapped and the
/// pool still full (CR 601.2c before CR 601.2h). "Target creature" is the word
/// worth a bystander on each side — the Elf across the table is offered and
/// must stay a printed 1/1 afterwards, and the artifact itself is no creature
/// at all. A `P1P1` counter is read rather than a body, because a pump until
/// end of turn would leave the same `(2, 2)` behind.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn dragon_blood_taps_and_three_mana_for_a_counter_on_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
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
        .hand(0, &[dragon_blood()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the counter");

    // Six Forests and only those: the Elves are named as the printing kept
    // back, because the creature this test reads afterwards is one of them and
    // a host tapped for mana reads as a different board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped: {{3}} for the artifact and the {{3}} the ability \
         charges, and neither Elf contributed"
    );
    cast_with_floating(&mut engine, p0, dragon_blood());
    pass_until(&mut engine, stack_is_empty);
    let blood = on_battlefield(&engine, p0, dragon_blood()).expect("the artifact resolved");
    assert!(
        !is_tapped(&engine, blood),
        "an artifact enters untapped, so its {{T}} is still there to pay"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{3}} is spent and the ability's {{3}} is still floating"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool — which is why the claim is made with the mana already there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(blood, 0)),
        "the one line the card prints, now that its {{3}} is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, dragon_blood(), 0);
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
        options.contains(&elf) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&blood),
        "the artifact is no creature, and it is not a legal target for its own \
         ability: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, blood),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{3}} is still in the pool for the same reason"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered was chosen");

    assert!(is_tapped(&engine, blood), "{{T}} is paid by the artifact");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "putting a counter on a creature is no mana ability, so the ability is \
         on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "\"put a +1/+1 counter on target creature\" — a counter and not a \
         pump, which is why the body below survives the turn"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "one +1/+1 on a printed 1/1");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the creature it targeted and never across the table"
    );
}
