//! `cards/creatures/mv_3/nim_abomination.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nim Abomination — {2}{B} Zombie, 3/4: "At the beginning of your end step,
/// if this creature is untapped, you lose 3 life."
///
/// That "if" is the whole card, so one permanent is read twice and the only
/// thing that differs between the two readings is its status. The turn it is
/// cast it stands untapped through its controller's end step and three life go
/// missing; the turn after, it attacks — a creature that attacked is a tapped
/// creature — and the same step costs nothing, which a card that simply drained
/// its controller every end step could not do.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn nim_abomination_drains_only_while_it_stands_untapped_at_your_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[nim_abomination()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, nim_abomination());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, nim_abomination()).is_some()
    });
    let zombie = on_battlefield(&engine, p0, nim_abomination()).expect("the Zombie resolved");
    assert_eq!(pt(&engine, zombie), (3, 4), "the body the card prints");
    assert!(!is_tapped(&engine, zombie), "and it arrives untapped");

    // The end step of the turn it was cast. Nothing on this board aims at
    // anything, so the one question the walk can meet is the one the trigger
    // asks — and every answer it takes is p0, which is what "you" means on
    // the card.
    let full = engine.state().players[0].life;
    for _ in 0..200 {
        if engine.state().players[0].life != full {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                let players: Vec<PlayerId> = player_options
                    .iter()
                    .copied()
                    .filter(|seat| *seat == p0)
                    .take(1)
                    .collect();
                let objects = if players.is_empty() {
                    options.into_iter().take(1).collect()
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::ChooseTargets { objects, players })
                    .unwrap();
            }
            Pending::ChoosePlayer { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p0))
                    .unwrap();
            }
            other => panic!("unexpected on the way to the end step: {other:?}"),
        }
    }
    assert_eq!(
        engine.state().players[0].life,
        full - 3,
        "\"at the beginning of your end step, if this creature is untapped, \
         you lose 3 life\" — the Zombie is untapped and standing"
    );

    // The following turn: the Zombie turns sideways, which is the one thing on
    // this board that can leave it tapped when an end step begins.
    let first_turn = engine.state().turn.number;
    let their_life = engine.state().players[1].life;
    let mut attacked = false;
    let mut at_end_step = false;
    for _ in 0..600 {
        if engine.state().turn.number > first_turn
            && matches!(engine.state().turn.phase, Phase::Ending)
            && engine.state().turn.active == p0
            && stack_is_empty(&engine)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            at_end_step = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers {
                player, attackers, ..
            } => {
                if player == p0 && !attacked {
                    assert!(
                        attackers.contains(&zombie),
                        "an untapped 3/4 deployed last turn may attack: {attackers:?}"
                    );
                    engine
                        .apply(
                            p0,
                            PlayerAction::DeclareAttackers {
                                attackers: vec![(zombie, Defender::Player(p1))],
                            },
                        )
                        .unwrap();
                    attacked = true;
                } else {
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                        .unwrap();
                }
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected on the way to the next end step: {other:?}"),
        }
    }
    assert!(
        at_end_step,
        "the walk reaches the Zombie's controller's next end step"
    );
    assert!(
        attacked,
        "the Zombie attacked on its controller's next turn"
    );
    assert!(
        is_tapped(&engine, zombie),
        "and it is still tapped when the end step begins"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life - 3,
        "the 3/4 connected, which is what made it a tapped attacker"
    );
    assert_eq!(
        engine.state().players[0].life,
        full - 3,
        "the same trigger that drained three life while the Zombie stood \
         untapped costs nothing while it is tapped, so the printed \"if\" is a \
         condition and not decoration"
    );
}
