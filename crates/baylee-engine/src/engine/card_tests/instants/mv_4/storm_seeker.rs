//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn storm_seeker_uses_its_targets_current_hand_size() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[index::STORM_SEEKER])
        .hand(1, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 4);
    e.refresh_offer();
    cast_from_hand(&mut e, p0, index::STORM_SEEKER);
    e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    let card = in_hand(&e, p1, forest()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            card,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 17);
    assert_eq!(e.state().players[0].life, 20);
}
