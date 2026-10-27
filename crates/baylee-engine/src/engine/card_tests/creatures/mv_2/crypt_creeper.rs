//! Oracle scenarios for Crypt Creeper.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn crypt_creeper() -> CardIndex {
    card_index("00b3971b-5bf7-4a3f-9607-6265f9af9098")
}

#[test]
fn crypt_creeper_sacrifices_itself_to_exile_target_card_from_a_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[index::SWAMP, index::SWAMP])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[crypt_creeper()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p1)
        .unwrap()
        .move_object(
            bears,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();

    cast_from_hand(&mut e, p0, crypt_creeper());
    pass_until(&mut e, stack_is_empty);

    activate(&mut e, p0, crypt_creeper(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, crypt_creeper()).is_some());
    assert!(in_graveyard(&e, p1, index::GRIZZLY_BEARS).is_none());
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&bears)
    );
}
