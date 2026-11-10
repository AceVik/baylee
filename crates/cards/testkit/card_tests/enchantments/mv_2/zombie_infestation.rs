//! Oracle scenarios for Zombie Infestation.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn zombie_infestation_discards_two_cards_to_create_zombie_token() {
    let p0 = PlayerId::new(0);
    let card = card_index("bdce1af0-3643-4e77-88f9-320206a191d4");
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[card])
        .hand(0, &[index::SWAMP, index::PLAINS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let inf = on_battlefield(&e, p0, card).unwrap();

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: inf,
            ability_index: 0,
        },
    )
    .unwrap();
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

    assert!(e.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(p0)).len(), 2);
    assert_eq!(
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|&&id| e.state().object(id).unwrap().card.is_none())
            .count(),
        1
    );
}
