//! `cards/creatures/mv_2/thorin_oakenshield.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thorin Oakenshield is `Coverage::Partial`: the printed **trample** is
/// enforced, while storied and the enduring-story ward grant are not. The
/// board is the smallest one that tells the enforced half from the missing
/// one — a 3/2 trampler attacking into a 1/1 — because the two excess points
/// of damage reaching the defending player are the only evidence that
/// trample is *applied* and not merely printed: a keyword the engine ignored
/// would leave that player at twenty. Casting it off a Mountain and a Plains,
/// and walking through a whole turn so the Dwarf is no longer summoning sick,
/// is the rest of the printed card arriving in a real game before it swings.
#[test]
fn thorin_oakenshield_casts_as_a_three_two_and_its_trample_spills_over_a_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), plains()])
        .hand(0, &[thorin_oakenshield()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A real cast: {R} off the Mountain and {W} off the Plains.
    cast_from_hand(&mut engine, p0, thorin_oakenshield());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, thorin_oakenshield()).is_some()
    });
    let thorin = on_battlefield(&engine, p0, thorin_oakenshield()).expect("Thorin resolved");
    assert_eq!(pt(&engine, thorin), (3, 2), "the printed 3/2 body");
    assert!(
        keywords(&engine, thorin).contains(KeywordSet::TRAMPLE),
        "the printed trample reaches the permanent"
    );

    // Through p1's turn and back, so the Dwarf is no longer summoning sick.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    // Declare Thorin as the only attacker, aimed at the only opponent: the
    // defender is taken straight out of the request the engine published
    // rather than built by hand.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&thorin),
        "an untapped, no-longer-sick Thorin may attack: {attackers:?}"
    );
    assert_eq!(defenders.len(), 1, "one opponent to attack in a duel");
    let target = defenders
        .into_iter()
        .next()
        .expect("the duel publishes its one defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(thorin, target)],
            },
        )
        .unwrap();

    // The 1/1 Elf blocks: one point of lethal damage, two trampling over.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the blocker is out");
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == elves && b.attackers.contains(&thorin)),
        "the Elves may block Thorin: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, thorin)],
            },
        )
        .unwrap();

    // Through the combat damage step.
    pass_until(&mut engine, |e| e.state().players[1].life < 20);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "three trampling power: one point is lethal to the 1/1 and the other \
         two go over the top, which is the keyword being applied"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker took lethal damage and died"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the 1/1's single point of return damage is marked on Thorin, not on its controller"
    );
    assert!(
        on_battlefield(&engine, p0, thorin_oakenshield()).is_some(),
        "a 3/2 with one damage marked survives its own attack"
    );
}
