//! `cards/creatures/mv_4/clickslither.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clickslither — {1}{R}{R}{R} — Creature — Insect 3/3 with haste: "Sacrifice
/// a Goblin: This creature gets +2/+2 and gains trample until end of turn."
///
/// The cost names no Goblin in particular, so the engine has to ask which one,
/// and that menu is half the card: the Festering Goblin under the same seat is
/// the whole of it, while the Insect that prints the ability and the Elf beside
/// it lack the subtype and the Goblin across the table is not this seat's to
/// give up (CR 701.21a). The other half is what the price buys — a printed 3/3
/// while the question still stands, and (5, 5) with trample once the stack has
/// emptied, on the same creature, because "this creature" is a filter and not a
/// target.
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
#[test]
fn clickslither_eats_a_goblin_of_yours_for_two_two_and_trample() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_creature(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[clickslither()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{R}{R}{R} off the four Mountains and the Elf's own {G}: `tap_all_mana`
    // presses both a basic land and a printed mana creature (#159), so nothing is
    // tapped by hand afterwards.
    cast_from_hand(&mut engine, p0, clickslither());
    pass_until(&mut engine, stack_is_empty);
    let slicer = on_battlefield(&engine, p0, clickslither()).expect("the Insect resolved");
    assert!(
        keywords(&engine, slicer).contains(KeywordSet::HASTE),
        "the printed haste line reaches the permanent"
    );
    assert_eq!(pt(&engine, slicer), (3, 3), "a printed 3/3 before the pump");

    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(slicer, 0)),
        "the price is a Goblin and no mana at all, so the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, clickslither(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Goblin, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one Goblin, no more and no fewer");
    assert_eq!(
        options,
        vec![mine],
        "the Festering Goblin under this seat is the whole menu: not the Insect \
         that prints the ability, not the creature beside it that is no Goblin, \
         and not the Goblin across the table"
    );
    assert_eq!(
        pt(&engine, slicer),
        (3, 3),
        "CR 601.2h pays last, so the pump has not happened while the cost is asked"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Goblin the question offered pays the cost");
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_none(),
        "the sacrificed Goblin left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, festering_goblin()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "the pump is an effect and waits on the stack — with the dead Goblin's \
         own \"when this dies\" trigger above it (CR 603.3b)"
    );

    // The -1/-1 the dead Goblin left behind is aimed at the Elf, so the
    // Clickslither is measured with only the printed +2/+2 on it. The walk
    // answers whatever order the two arrive in.
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                let elf = on_battlefield(&engine, p0, quiet_creature())
                    .expect("the Elf is still out to be aimed at");
                assert!(
                    options.contains(&elf),
                    "any creature is a legal target for the Goblin's own \
                     trigger: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .expect("the Elf was one of the options it enumerated");
            }
            Pending::Priority { player, .. } => {
                if stack_is_empty(&engine) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the sacrifice resolves: {other:?}"),
        }
    }

    assert_eq!(
        pt(&engine, slicer),
        (5, 5),
        "+2/+2 on the creature that paid the price, and on that creature only"
    );
    assert!(
        keywords(&engine, slicer).contains(KeywordSet::TRAMPLE),
        "and the printed trample is granted with it"
    );
    assert!(
        on_battlefield(&engine, p0, clickslither()).is_some(),
        "the Insect outlives the Goblin it ate"
    );
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the opponent's board never moved"
    );
}
