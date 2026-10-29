//! Oracle scenarios for Psychatog.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn psychatog() -> CardIndex {
    card_index("d7c4da85-d1a5-4a27-812d-65d8ee1494dd")
}

#[test]
fn psychatog_pumps_from_discarding_card_and_exiling_two_graveyard_cards() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(
            0,
            &[index::ISLAND, index::ISLAND, index::SWAMP, index::PLAINS],
        )
        .hand(0, &[psychatog(), index::MOUNTAIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = on_battlefield(&e, p0, index::PLAINS).unwrap();
    bury(&mut e, &[plains]);

    cast_from_hand(&mut e, p0, psychatog());
    pass_until(&mut e, stack_is_empty);

    let tog = on_battlefield(&e, p0, psychatog()).unwrap();
    assert_eq!(pt(&e, tog), (1, 2));

    // Ability 0: Discard a card -> +1/+1 until end of turn.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: tog,
            ability_index: 0,
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

    // Ability 1: Exile two cards from your graveyard -> +1/+1 until end of turn.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: tog,
            ability_index: 1,
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
