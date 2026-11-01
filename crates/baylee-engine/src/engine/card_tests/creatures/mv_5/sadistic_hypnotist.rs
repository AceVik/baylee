//! Oracle scenarios for Sadistic Hypnotist.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn sadistic_hypnotist() -> CardIndex {
    card_index("937de7f3-7f7f-4fd4-8531-0b90a11815c8")
}

#[test]
fn sadistic_hypnotist_sacrifices_creature_at_sorcery_speed_for_target_player_to_discard_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[sadistic_hypnotist()])
        .hand(1, &[index::PLAINS, index::FOREST, index::ISLAND])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, sadistic_hypnotist());
    pass_until(&mut e, stack_is_empty);

    let hyp = on_battlefield(&e, p0, sadistic_hypnotist()).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let hand_before = hand_size(&e, p1);

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: hyp,
            ability_index: 0,
        },
    )
    .unwrap();
    if matches!(e.pending(), Pending::ChooseTargets { .. }) {
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    } else {
        e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    }
    if let Pending::ChooseCards { .. } = e.pending().clone() {
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    }
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected ChooseCards for P1 discard");
    };
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![options[0], options[1]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(hand_size(&e, p1), hand_before - 2);
}
