//! `cards/sorceries/mv_4/ashen_powder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ASHEN_POWDER, GRIZZLY_BEARS};

#[test]
fn ashen_powder_reanimates_opponent_creature_under_your_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[ASHEN_POWDER])
        .hand(1, &[GRIZZLY_BEARS])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = in_hand(&engine, p1, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, ASHEN_POWDER);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).unwrap();
    assert_eq!(bear_obj.zone, Zone::Battlefield);
    assert_eq!(bear_obj.controller, p0);
}
