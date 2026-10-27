//! Oracle scenarios for Steamclaw.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn steamclaw_exiles_card_from_graveyard_by_tapping_or_sacrificing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("8cbc0fa4-4f62-47e1-90dc-c21d2682a493");
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                card,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    for &id in &[bear, giant] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(p1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    let claw = on_battlefield(&e, p0, card).unwrap();
    let gy_cards = e.state().zones.list(ZoneLocation::Graveyard(p1)).clone();
    assert_eq!(gy_cards.len(), 2);

    tap_mana_where(&mut e, p0, |_| true);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: claw,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![gy_cards[0]],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p1)).len(), 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(p1)).len(), 1);
    assert!(on_battlefield(&e, p0, card).is_some());

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: claw,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![gy_cards[1]],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty());
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(p1)).len(), 2);
    assert!(on_battlefield(&e, p0, card).is_none());
}
