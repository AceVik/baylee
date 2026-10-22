//! `cards/instants/mv_3/bone_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BONE_HARVEST, GRIZZLY_BEARS};

#[test]
fn bone_harvest_puts_creature_card_on_top_of_library_and_draws_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[BONE_HARVEST, GRIZZLY_BEARS])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = in_hand(&engine, p0, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bear,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, BONE_HARVEST);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Library);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().object(bear).unwrap().zone,
        Zone::Hand,
        "Bone Harvest should draw the card placed on top of the library at the next turn's upkeep"
    );
}
