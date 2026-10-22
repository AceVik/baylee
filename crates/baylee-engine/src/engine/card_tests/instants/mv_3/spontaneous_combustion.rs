//! `cards/instants/mv_3/spontaneous_combustion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn spontaneous_combustion() -> CardIndex {
    card_index("0f81064d-01cd-4a61-8aea-2544e4d14b70")
}

/// `Spontaneous Combustion` ({1}{B}{R}, Instant, `Coverage::Implemented`):
/// `As an additional cost to cast this spell, sacrifice a creature.`
/// `Spontaneous Combustion deals 3 damage to each creature.`
#[test]
fn spontaneous_combustion_sacrifices_a_creature_and_deals_three_damage_to_each_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), mountain(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[spontaneous_combustion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, spontaneous_combustion());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p1, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
}
