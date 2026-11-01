//! `cards/creatures/mv_4/sparkcaster.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Sparkcaster — {2}{R}{G} Creature — Kavu 5/3
/// "When this creature enters, return a red or green creature you control to its owner's hand.
/// When this creature enters, it deals 1 damage to target player or planeswalker."
#[test]
fn sparkcaster_returns_red_or_green_creature_you_control_and_deals_one_damage_to_target_player() {
    let sparkcaster = card_index("84c912e8-6a92-47f7-88b6-e777bd0bb424");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[grizzly_bears(), mountain(), mountain(), forest(), forest()],
        )
        .hand(0, &[sparkcaster])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, sparkcaster);

    // Choose target player p1 for the damage ETB trigger, and Grizzly Bears for the gating trigger.
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                let pick = if options.contains(&bears_id) {
                    bears_id
                } else {
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
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    assert!(on_battlefield(&engine, p0, sparkcaster).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[1].life, 19);
}
