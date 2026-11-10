//! `cards/creatures/mv_2/youthful_knight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Youthful Knight prints one line — "First strike" — onto a 2/1 for
/// {1}{W}, and none of it can be read off the card itself. The test
/// actually casts it and reads the projected 2/1 with the projected
/// first strike; then it blocks the 1/1 Elf on the table, which only
/// the line decides — the Knight strikes first and survives, where a
/// 2/1 without first strike would die together with it in one step.
#[test]
fn youthful_knight_costs_two_and_blocks_as_a_2_1_first_striker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[youthful_knight()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, youthful_knight());
    pass_until(&mut engine, stack_is_empty);

    let knight = on_battlefield(&engine, p0, youthful_knight()).expect("the Knight resolved");
    assert_eq!(pt(&engine, knight), (2, 1), "{{1}}{{W}} buys a 2/1");
    assert!(
        keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE),
        "the one line the card prints reaches the permanent through the layers"
    );

    // Summoning sickness keeps the Knight out of this turn's attack, so the
    // proof is on the other side of the table: p1's Elf attacks and the
    // Knight blocks.
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("the untapped Elf may attack");
    // CR 508.2: priority goes round once the attack is declared, so the
    // block question is not the next pending.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("p0 is asked for blockers, got {:?}", engine.pending());
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == knight && b.attackers.contains(&elf)),
        "the Knight is offered as a blocker for the Elf: {blockers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(knight, elf)],
            },
        )
        .expect("the pairing was one of the options");

    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, llanowar_elves()).is_none()
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two first-strike damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    let survivor = on_battlefield(&engine, p0, youthful_knight()).expect("the Knight survived");
    assert_eq!(
        pt(&engine, survivor),
        (2, 1),
        "and took no damage: the Elf was dead before the normal damage step"
    );
}
