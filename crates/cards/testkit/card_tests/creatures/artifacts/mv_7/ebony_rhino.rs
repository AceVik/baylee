//! `cards/creatures/artifacts/mv_7/ebony_rhino.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ebony Rhino` prints `Trample` on a 4/5 artifact creature with `Coverage::Implemented`.
/// In this scenario, seat 0 attacks seat 1 with `Ebony Rhino`.
/// Seat 1 blocks with a 1/1 `llanowar_elves()`.
/// Due to `KeywordSet::TRAMPLE`, 1 damage is assigned to destroy the blocking elf, and the remaining 3 damage tramples over to reduce seat 1's life to 17.
#[test]
fn ebony_rhino_tramples_over_blocker_dealing_excess_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ebony_rhino()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rhino = on_battlefield(&engine, p0, ebony_rhino()).expect("rhino seated");
    assert_eq!(pt(&engine, rhino), (4, 5));
    assert!(
        keywords(&engine, rhino).contains(KeywordSet::TRAMPLE),
        "`Ebony Rhino` has trample"
    );
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf seated");

    // Advance to declare attackers.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rhino, Defender::Player(p1))],
            },
        )
        .expect("rhino attacks p1");

    // Seat 1 declares blocker.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, rhino)],
            },
        )
        .expect("elf blocks rhino");

    // Past the damage step, and `stack_is_empty` is not that: the stack is
    // already empty the moment blockers are declared, so waiting for it
    // returns before a single point has been dealt. CR 510.2 is what the two
    // assertions below are read through.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    // Blocker is destroyed.
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "blocking elf was dealt lethal damage"
    );
    // Trample deals remaining 3 damage to p1.
    assert_eq!(
        engine.state().players[1].life,
        17,
        "trample dealt 3 damage over the 1-toughness blocker"
    );
    // Rhino survives with 4 toughness remaining.
    assert!(
        on_battlefield(&engine, p0, ebony_rhino()).is_some(),
        "rhino survives combat"
    );
}

/// Veteran Bodyguard counter-check: a blocked trampler's excess over its
/// blocker reaches the player directly — a blocked attacker is never
/// "unblocked" (CR 509.1h), so the Bodyguard's static does not apply to
/// it — while an unblocked attacker beside it is still redirected.
#[test]
fn a_blocked_tramplers_excess_reaches_the_player_while_an_unblocked_attacker_beside_it_is_redirected()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ebony_rhino(), gray_ogre()])
        .battlefield(1, &[veteran_bodyguard(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rhino = on_battlefield(&engine, p0, ebony_rhino()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    let before_life = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rhino, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .expect("both attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, rhino)],
            },
        )
        .expect("the Elf blocks the trampling Rhino; the Ogre is unblocked");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elf took the Rhino's lethal 1"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life - 3,
        "the Rhino's trample excess (4 minus the Elf's 1 toughness): on the player"
    );
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        2,
        "the unblocked Ogre's damage: still redirected onto the Bodyguard"
    );
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[1],
        3,
        "only the trample excess counted for the player, not the Ogre's redirected 2"
    );
}
