//! `cards/creatures/mv_6/razing_snidd.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Razing Snidd — {4}{B}{R} Creature — Beast 3/3
/// "When this creature enters, return a black or red creature you control to its owner's hand.
/// When this creature enters, each player sacrifices a land of their choice."
#[test]
fn razing_snidd_returns_black_or_red_creature_and_makes_each_player_sacrifice_a_land() {
    let snidd = card_index("bbcd2045-cf51-4346-a561-ebae8ffdc181");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                index::HILL_GIANT,
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[swamp()])
        .hand(0, &[snidd])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant_id = on_battlefield(&engine, p0, index::HILL_GIANT).expect("Hill Giant on board");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, snidd);

    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => {
                let pick = if options.contains(&giant_id) {
                    giant_id
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

    assert!(on_battlefield(&engine, p0, snidd).is_some());
    assert!(on_battlefield(&engine, p0, index::HILL_GIANT).is_none());
    assert!(in_hand(&engine, p0, index::HILL_GIANT).is_some());
    assert_eq!(
        all_on_battlefield(&engine, p0, swamp()).len()
            + all_on_battlefield(&engine, p0, mountain()).len(),
        5,
        "the caster sacrificed one of their six lands"
    );
    assert!(on_battlefield(&engine, p1, swamp()).is_none());
}
