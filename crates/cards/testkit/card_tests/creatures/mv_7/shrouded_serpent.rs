//! `cards/creatures/mv_7/shrouded_serpent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn shrouded_serpent() -> CardIndex {
    card_index("6e0deb42-f876-4fb8-9536-6d9a42bd6e5e")
}

/// Shrouded Serpent ({4}{U}{U}{U} 4/4 Creature — Serpent):
/// "Whenever this creature attacks, defending player may pay {4}. If that player doesn't, this creature can't be blocked this turn."
#[test]
fn shrouded_serpent_cannot_be_blocked_when_defending_player_declines_to_pay_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = shrouded_serpent();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[subject])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );

    let serpent = on_battlefield(&engine, p0, subject).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(serpent, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 16);
}

/// When the defending player pays {4}, Shrouded Serpent can be blocked as normal.
#[test]
fn shrouded_serpent_can_be_blocked_when_defending_player_pays_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = shrouded_serpent();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[subject])
        .battlefield(
            1,
            &[index::GRIZZLY_BEARS, forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );

    let serpent = on_battlefield(&engine, p0, subject).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(serpent, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );

    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        blockers
            .iter()
            .any(|b| b.blocker == bears && b.attackers.contains(&serpent)),
        "Grizzly Bears should be offered as a legal blocker when defending player paid {{4}}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, serpent)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
}
