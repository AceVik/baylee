//! `cards/instants/mv_5/bounty_of_the_hunt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bounty_of_the_hunt() -> CardIndex {
    card_index("77a43c24-398f-42c6-a0d3-b28b33a0ce12")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// `Bounty of the Hunt` ({3}{G}{G}, Instant, `Coverage::Partial`) may be cast
/// by exiling a green card from your hand rather than paying its mana cost, and
/// puts three +1/+1 counters on target creature.
#[test]
fn bounty_of_the_hunt_pitches_green_card_to_put_three_p1p1_counters_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hill_giant()])
        .hand(0, &[bounty_of_the_hunt(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let pitch = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let bounty = in_hand(&engine, p0, bounty_of_the_hunt()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: bounty })
        .unwrap();
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .unwrap();
                engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![giant],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending during pitch cast: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(pitch).unwrap().zone, Zone::Exile);
    assert_eq!(counters_on(&engine, giant, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, giant), (6, 6));
}
