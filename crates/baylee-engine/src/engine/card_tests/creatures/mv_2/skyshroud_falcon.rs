//! `cards/creatures/mv_2/skyshroud_falcon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyshroud Falcon is a {1}{W} 1/1 Bird whose whole printed text is two
/// keywords, and each one is only legible in a different half of one combat
/// step. The vigilance half is read off the attacker itself: the Falcon is
/// still standing untapped after it was declared, where the Llanowar Elves
/// that attacked beside it is not. The flying half is read off the *block*
/// offer, where the same untapped Elf across the table may block the ground
/// attacker and may not block the Falcon at all.
#[test]
fn skyshroud_falcon_attacks_without_tapping_and_a_ground_creature_cannot_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[skyshroud_falcon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W} off the two Plains, with the Elf named as the printing kept
    // back: it is the creature that attacks beside the Falcon below, and a
    // creature tapped for mana may not be declared as an attacker.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains tapped, and the Elf that has to attack later did not pay"
    );
    cast_with_floating(&mut engine, p0, skyshroud_falcon());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let falcon = on_battlefield(&engine, p0, skyshroud_falcon()).expect("the Falcon resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    assert_eq!(pt(&engine, falcon), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the two mana are what paid for it"
    );

    // Cast this turn, so it is summoning sick (CR 302.6) and the attack
    // declaration it is read in belongs to the *next* turn of p0's.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&falcon) && attackers.contains(&elf),
        "both untapped creatures may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(falcon, Defender::Player(p1)), (elf, Defender::Player(p1))],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, falcon),
        "\"vigilance\": attacking does not tap the Falcon"
    );
    assert!(
        is_tapped(&engine, elf),
        "and the Elf beside it, which has no vigilance, is tapped by the same declaration"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let option = blockers
        .iter()
        .find(|b| b.blocker == their_elf)
        .expect("an untapped creature across the table may block");
    assert!(
        option.attackers.contains(&elf),
        "the ground attacker is reachable by the ground blocker: {:?}",
        option.attackers
    );
    assert!(
        !option.attackers.contains(&falcon),
        "\"flying\": the same untapped Elf cannot be assigned to it at all: {:?}",
        option.attackers
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate would stop the walk *before* combat
    // damage and the life total would read 20. The end step is past it
    // (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two 1/1s went unblocked: one from the Falcon that flew over and one from the Elf"
    );
    assert!(
        !is_tapped(&engine, falcon),
        "and the Falcon is still standing after the combat it attacked through"
    );
}
