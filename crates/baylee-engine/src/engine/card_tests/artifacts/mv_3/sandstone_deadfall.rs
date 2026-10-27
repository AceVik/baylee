//! Oracle scenarios for Sandstone Deadfall.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn sandstone_deadfall() -> CardIndex {
    card_index("42c77495-5cdb-4140-827c-b1e017640c3d")
}

#[test]
fn sandstone_deadfall_taps_and_sacrifices_two_lands_and_self_to_destroy_attacking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &[index::PLAINS; 3])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[sandstone_deadfall()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, sandstone_deadfall());
    pass_until(&mut e, stack_is_empty);

    let deadfall = on_battlefield(&e, p0, sandstone_deadfall()).unwrap();
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: deadfall,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    for _ in 0..2 {
        let Pending::ChooseCards { options, .. } = e.pending().clone() else {
            panic!("expected ChooseCards for sacrificing a land");
        };
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, sandstone_deadfall()).is_some());
    assert!(in_graveyard(&e, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(all_on_battlefield(&e, p0, index::PLAINS).len(), 1);
}
