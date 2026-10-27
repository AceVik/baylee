//! Oracle scenarios for Immobilizing Ink.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn immobilizing_ink() -> CardIndex {
    card_index("154e865f-1aab-4d72-9e84-44e5316bbf3c")
}

#[test]
fn immobilizing_ink_stops_untapping_and_grants_discard_to_untap_ability() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(
            0,
            &[
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[immobilizing_ink(), index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(bears)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    e.refresh_offer();
    assert!(is_tapped(&e, bears));

    cast_from_hand(&mut e, p0, immobilizing_ink());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    reach_their_main_phase(&mut e, PlayerId::new(1));
    assert!(walk_to_own_main(&mut e, p0));
    assert!(
        is_tapped(&e, bears),
        "enchanted creature does not untap during its controller's untap step"
    );

    tap_all_mana(&mut e, p0);
    let grant = crate::effects::granted_activated(e.state(), bears)
        .next()
        .unwrap()
        .id;
    let discard_card = in_hand(&e, p0, index::FOREST).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: bears,
            ability_index: grant,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard_card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(!is_tapped(&e, bears));
    assert!(in_graveyard(&e, p0, index::FOREST).is_some());
}
