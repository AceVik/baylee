//! Oracle scenarios for Zombie Assassin.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn zombie_assassin_exiles_two_graveyard_cards_and_itself_to_destroy_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("7d8af783-e73f-49ce-bcdb-ba55e1650481");
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[card, index::SWAMP, index::SWAMP])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let swamps = all_on_battlefield(&e, p0, index::SWAMP);
    for &s in &swamps {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                s,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    let assassin = on_battlefield(&e, p0, card).unwrap();
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p1)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .regeneration_shields = 1;
    let gy_cards = e.state().zones.list(ZoneLocation::Graveyard(p0)).clone();
    assert_eq!(gy_cards.len(), 2);

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: assassin,
            ability_index: 0,
        },
    )
    .unwrap();
    if let Pending::ChooseTargets { .. } = e.pending().clone() {
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    }
    while let Pending::ChooseCards { .. } = e.pending().clone() {
        let Pending::ChooseCards { options, min, .. } = e.pending().clone() else {
            unreachable!()
        };
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..usize::from(min)].to_vec(),
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, card).is_none());
    assert!(on_battlefield(&e, p1, index::GRIZZLY_BEARS).is_none());
    assert_eq!(e.state().zones.list(ZoneLocation::Exile(p0)).len(), 3);
}
