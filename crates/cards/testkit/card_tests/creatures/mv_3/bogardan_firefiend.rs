//! `cards/creatures/mv_3/bogardan_firefiend.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bogardan Firefiend — {2}{R}, a 2/1 Elemental Spirit whose whole text is
/// "When this creature dies, it deals 2 damage to target creature."
///
/// The trigger only exists if the creature really dies, so the scenario kills
/// it the only way a 2/1 dies on this board: it attacks and is blocked by a
/// 1/1, and the two trade. The second Elf across the table is what the
/// ability has to aim at — the blocker is already in the graveyard by the
/// time the trigger reaches the stack, so a Fiend read as "the creature it
/// fought" would have nothing left to point at.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bogardan_firefiend_deals_two_to_a_creature_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[bogardan_firefiend()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bogardan_firefiend());
    pass_until(&mut engine, stack_is_empty);
    let fiend = on_battlefield(&engine, p0, bogardan_firefiend()).expect("the Firefiend resolved");
    assert_eq!(pt(&engine, fiend), (2, 1), "the body the card prints");
    assert_eq!(
        all_on_battlefield(&engine, p1, llanowar_elves()).len(),
        2,
        "one Elf to block with and one to be aimed at"
    );

    // A whole turn cycle, so the Fiend is no longer summoning sick.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    let (blocker, bystander) = (elves[0], elves[1]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(
        attackers.contains(&fiend),
        "an untapped 2/1 with no text of its own may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fiend, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the block declaration")
    };
    let option = blockers
        .iter()
        .find(|o| o.blocker == blocker)
        .expect("a 1/1 may block a 2/1");
    assert!(
        option.attackers.contains(&fiend),
        "and it may block this one: {:?}",
        option.attackers
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, fiend)],
            },
        )
        .unwrap();

    // The trade is simultaneous, and the dies trigger is the next thing the
    // stack asks about — the creature it traded with is already gone.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on the target choice")
    };
    assert_eq!(player, p0, "the Fiend's controller aims its own trigger");
    assert!(
        !options.contains(&blocker),
        "the Elf it traded with died in the same damage step: {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "\"target creature\" reaches the other side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the creature the question offered is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, bogardan_firefiend()).is_some(),
        "the 2/1 died to the block that killed it, which is what the trigger waits for"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and the blocker went with it, in the same damage step"
    );
    assert!(
        all_on_battlefield(&engine, p1, llanowar_elves()).is_empty(),
        "the trigger's two damage killed the creature it was aimed at — an \
         Elf still standing here is the trigger that never fired"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to a creature and never to the player whose board it stood on"
    );
}
