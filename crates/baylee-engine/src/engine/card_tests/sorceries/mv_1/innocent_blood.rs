//! Oracle scenarios for Innocent Blood.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn innocent_blood() -> CardIndex {
    card_index("6791ec3c-c397-4087-8c8c-84d3797df415")
}

#[test]
fn innocent_blood_makes_each_player_sacrifice_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[index::SWAMP, index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[innocent_blood()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();

    cast_from_hand(&mut e, p0, innocent_blood());
    for _ in 0..20 {
        if stack_is_empty(&e) && matches!(e.pending(), Pending::Priority { .. }) {
            break;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards { player, .. } if player == p0 => {
                e.apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![bears],
                    },
                )
                .unwrap();
            }
            Pending::ChooseCards { player, .. } if player == p1 => {
                e.apply(
                    p1,
                    PlayerAction::ChooseObjects {
                        objects: vec![ogre],
                    },
                )
                .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    assert!(in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, p1, index::GRAY_OGRE).is_some());
}
