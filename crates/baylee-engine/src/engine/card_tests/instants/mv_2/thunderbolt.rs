//! `cards/instants/mv_2/thunderbolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AIR_ELEMENTAL, THUNDERBOLT};

/// `Thunderbolt` ({1}{R}, Instant) deals 3 damage to target player or
/// planeswalker or 4 damage to target creature with flying.
#[test]
fn thunderbolt_deals_four_damage_to_flying_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[AIR_ELEMENTAL])
        .hand(0, &[THUNDERBOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elemental = on_battlefield(&engine, p1, AIR_ELEMENTAL).unwrap();
    cast_from_hand(&mut engine, p0, THUNDERBOLT);
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    aim_at(&mut engine, p0, elemental);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, AIR_ELEMENTAL).is_some());
}
