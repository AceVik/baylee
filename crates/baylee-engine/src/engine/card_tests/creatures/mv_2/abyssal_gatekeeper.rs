//! `cards/creatures/mv_2/abyssal_gatekeeper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn abyssal_gatekeeper() -> CardIndex {
    card_index("4954181c-cb46-4cac-adc3-2c8b7693a211")
}

/// `Abyssal Gatekeeper` ({1}{B}, 1/1 Creature — Horror) causes each player to
/// sacrifice a creature when it dies.
#[test]
fn abyssal_gatekeeper_death_makes_each_player_sacrifice_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), abyssal_gatekeeper(), GRIZZLY_BEARS])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gk = on_battlefield(&engine, p0, abyssal_gatekeeper()).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gk],
                players: vec![],
            },
        )
        .unwrap();
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
}

/// "A creature of their choice": the opponent, holding a Bears and a Hill
/// Giant, is the one who is asked and gives up the Giant; the Bears stays.
#[test]
fn abyssal_gatekeeper_lets_the_opponent_choose_what_they_sacrifice() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), abyssal_gatekeeper(), GRIZZLY_BEARS])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gk = on_battlefield(&engine, p0, abyssal_gatekeeper()).unwrap();
    let their_bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gk],
                players: vec![],
            },
        )
        .unwrap();

    let mut asked = Vec::new();
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                asked.push(player);
                let pick = if player == p1 {
                    assert!(
                        options.contains(&their_bears) && options.contains(&giant),
                        "the opponent chooses among its own creatures"
                    );
                    giant
                } else {
                    assert!(!options.contains(&their_bears), "never the other side's");
                    options[0]
                };
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pick],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question {other:?}"),
        }
    }

    assert!(asked.contains(&p1), "the opponent made the choice");
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
}
