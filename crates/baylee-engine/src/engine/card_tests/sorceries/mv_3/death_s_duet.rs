//! `cards/sorceries/mv_3/death_s_duet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn death_s_duet() -> CardIndex {
    card_index("28d24add-da3a-4697-8fd2-5106c9225c0e")
}

/// Death's Duet (`28d24add-da3a-4697-8fd2-5106c9225c0e`) — {2}{B} Sorcery (Implemented):
/// "Return two target creature cards from your graveyard to your hand."
#[test]
fn death_s_duet_returns_two_target_creature_cards_from_graveyard_to_hand() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(
            0,
            &[
                death_s_duet(),
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                swamp(),
            ],
        )
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let c1 = hand_to_graveyard(&mut engine, P0, index::GRIZZLY_BEARS);
    let c2 = hand_to_graveyard(&mut engine, P0, index::HILL_GIANT);
    let noncreature = hand_to_graveyard(&mut engine, P0, swamp());
    let theirs = hand_to_graveyard(&mut engine, PlayerId::new(1), index::GRIZZLY_BEARS);
    cast_from_hand(&mut engine, P0, death_s_duet());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&noncreature));
    assert!(!options.contains(&theirs));
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![c1],
                    players: vec![]
                }
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![c1, c1],
                    players: vec![]
                }
            )
            .is_err()
    );
    kit::settle(&mut engine, P0, |_, p| {
        kit::choose_targets(p, &[c1, c2], &[])
    });

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P0, index::HILL_GIANT).is_some());
}
