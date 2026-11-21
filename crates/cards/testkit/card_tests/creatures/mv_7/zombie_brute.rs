//! `cards/creatures/mv_7/zombie_brute.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::CounterKind;
use baylee_core::ids::Defender;
use baylee_engine::choice::{ChoicePrompt, NumberPrompt};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zombie_brute() -> CardIndex {
    card_index("377177f3-65e5-4909-bf12-48a473b1861d")
}

/// Zombie Brute ({6}{B}, 5/4 Zombie):
/// Amplify 1; Trample.
#[test]
fn zombie_brute_amplifies_zombies_and_tramples_excess_damage() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 7])
        .battlefield(1, &[grizzly_bears()])
        .hand(
            0,
            &[
                zombie_brute(),
                scathe_zombies(),
                scathe_zombies(),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zombie_brute());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    let bear_in_hand = in_hand(&engine, P0, grizzly_bears()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2);
    assert!(!options.contains(&bear_in_hand));
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let brute = on_battlefield(&engine, P0, zombie_brute()).unwrap();
    assert_eq!(counters_on(&engine, brute, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, brute), (7, 6));

    // Turn 3: Zombie Brute (7/6 trample) attacks and is blocked by 2/2 Grizzly Bears -> 5 tramples over.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0)
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brute, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
    );
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, brute)],
            },
        )
        .unwrap();
    loop {
        if engine.state().turn.phase == Phase::SecondMain && stack_is_empty(&engine) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player,
                min,
                reason: NumberPrompt::CombatDamage { .. },
                ..
            } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(min))
                    .unwrap();
            }
            _ => {
                pass_until(&mut engine, |e| {
                    matches!(e.pending(), Pending::ChooseNumber { .. })
                        || (e.state().turn.phase == Phase::SecondMain && stack_is_empty(e))
                });
            }
        }
    }
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[1].life, 15);
}

#[test]
fn zombie_brute_zero_reveals_is_five_four() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 7])
        .hand(0, &[zombie_brute(), scathe_zombies()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zombie_brute());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let brute = on_battlefield(&engine, P0, zombie_brute()).unwrap();
    assert_eq!(counters_on(&engine, brute, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, brute), (5, 4));
}
