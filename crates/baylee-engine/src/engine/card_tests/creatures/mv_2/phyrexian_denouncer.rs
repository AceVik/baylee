//! `cards/creatures/mv_2/phyrexian_denouncer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Denouncer is a `{1}{B}` 1/1 whose entire text is one line:
/// "{T}, Sacrifice this creature: Target creature gets -1/-1 until end of
/// turn." It is *cast* here rather than seated, because the `{T}` in the cost
/// is what summoning sickness (CR 302.6) makes unpayable in the turn the
/// creature arrives — the offer is read twice, once on each side of that
/// rule, and the second read is a whole round of turns later. The `-1/-1` is
/// aimed at a body large enough to survive it, so the numbers read a *pump*
/// and not a death, and an Elf across the table is the control that
/// separates "target creature" from "creatures you control". The target
/// question is answered before the price is paid (CR 601.2c, then
/// CR 601.2h), so the still-standing creature on the battlefield is the state
/// that proves the ordering — and the last turn cycle is the printed "until
/// end of turn".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn phyrexian_denouncer_taps_and_sacrifices_itself_for_a_minus_one_minus_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), rootbreaker_wurm()])
        .hand(0, &[phyrexian_denouncer()])
        // A creature across the table: "target creature" names no controller,
        // and an offer that only ever held this seat's own board would
        // satisfy every number below.
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );

    // {1}{B} off the two Swamps, and then the offer is read in the turn it
    // arrived: the tap symbol is a price a creature this new may not pay.
    cast_from_hand(&mut engine, p0, phyrexian_denouncer());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, phyrexian_denouncer()).is_some()
    });
    let denouncer =
        on_battlefield(&engine, p0, phyrexian_denouncer()).expect("the Denouncer resolved");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == denouncer),
        "CR 302.6: a {{T}} in the cost of a creature that arrived this turn is \
         unpayable, so the one line the card prints is not offered: {:?}",
        legal.abilities
    );

    // A whole round of turns, which is what CR 302.6 asks for.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "and back to the Denouncer's controller"
    );

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let (power, toughness) = pt(&engine, wurm);
    let elf_body = pt(&engine, elf);
    assert!(
        power > 1 && toughness > 1,
        "a body the -1/-1 cannot kill, so the numbers below read a pump and \
         not a death"
    );
    assert!(
        !is_tapped(&engine, denouncer),
        "nothing has tapped it: the price is still unpaid"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap symbol and the creature, and neither is mana"
    );

    activate(&mut engine, p0, phyrexian_denouncer(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&wurm) && options.contains(&elf),
        "\"target creature\" reaches both sides of the table: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while the question stands the price is
    // unpaid, so the Denouncer is still on the battlefield and still standing.
    assert!(
        on_battlefield(&engine, p0, phyrexian_denouncer()).is_some(),
        "the sacrifice is a cost, and costs are paid after the target"
    );
    assert!(
        !is_tapped(&engine, denouncer),
        "and it is still standing for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the body the question offered was chosen");

    assert!(
        on_battlefield(&engine, p0, phyrexian_denouncer()).is_none(),
        "\"Sacrifice this creature\" takes the whole card"
    );
    assert!(
        in_graveyard(&engine, p0, phyrexian_denouncer()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "shrinking a creature is no mana ability, so it waits on the stack"
    );
    assert_eq!(
        pt(&engine, wurm),
        (power, toughness),
        "and the -1/-1 is the resolving effect, so it has not landed yet"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wurm),
        (power - 1, toughness - 1),
        "-1/-1 on the creature the ability was aimed at"
    );
    assert_eq!(
        pt(&engine, elf),
        elf_body,
        "and nothing at all for the creature it was not aimed at"
    );

    // "until end of turn": a whole round later the body is the one it was
    // printed as, so what landed was a pump and not a counter.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "its controller takes another turn"
    );
    assert_eq!(
        pt(&engine, wurm),
        (power, toughness),
        "the pump expired with the turn it was made in"
    );
}
