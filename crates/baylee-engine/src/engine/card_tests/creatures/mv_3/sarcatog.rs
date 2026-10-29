//! Oracle scenarios for Sarcatog.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn sarcatog() -> CardIndex {
    card_index("e5afc145-1fc3-4d87-b175-0dcd9a81ae7c")
}

#[test]
fn sarcatog_pumps_from_sacrificing_artifact_and_exiling_two_graveyard_cards() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::MOUNTAIN,
                index::PLAINS,
                index::ORNITHOPTER,
            ],
        )
        .hand(0, &[sarcatog()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = on_battlefield(&e, p0, index::PLAINS).unwrap();
    bury(&mut e, &[plains]);

    cast_from_hand(&mut e, p0, sarcatog());
    pass_until(&mut e, stack_is_empty);

    let tog = on_battlefield(&e, p0, sarcatog()).unwrap();
    assert_eq!(pt(&e, tog), (1, 2));

    // Ability 1: Sacrifice an artifact -> +1/+1 until end of turn.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: tog,
            ability_index: 1,
        },
    )
    .unwrap();
    if let Pending::ChooseCards { options, .. } = e.pending().clone() {
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, tog), (2, 3));

    // Ability 0: Exile two cards from your graveyard -> +1/+1 until end of turn.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: tog,
            ability_index: 0,
        },
    )
    .unwrap();
    for _ in 0..2 {
        if let Pending::ChooseCards { options, .. } = e.pending().clone() {
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
        }
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, tog), (3, 4));
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 0);
}
