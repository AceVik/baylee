//! `cards/sorceries/mv_4/barter_in_blood.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn barter_in_blood() -> CardIndex {
    card_index("9167998d-5cac-47d7-99f2-f38122f7b8e7")
}

/// Barter in Blood — {2}{B}{B} Sorcery
/// "Each player sacrifices two creatures of their choice."
#[test]
fn barter_in_blood_makes_each_player_sacrifice_two_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[barter_in_blood()])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                GRIZZLY_BEARS,
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[llanowar_elves(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, barter_in_blood());
    // Answer each keep-creatures choice as it comes up (P0 keeps Hill Giant, sacrificing Llanowar Elves and Grizzly Bears).
    while !stack_is_empty(&engine) || !matches!(engine.pending(), Pending::Priority { .. }) {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let objects = if player == p0 {
                    vec![p0_giant]
                } else {
                    options.into_iter().take(usize::from(min)).collect()
                };
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p0, HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
}
