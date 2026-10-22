//! `cards/enchantments/mv_2/spatial_binding.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn spatial_binding() -> CardIndex {
    card_index("52fc8217-5a14-40b5-a8fc-b6fc45af5f33")
}

/// Spatial Binding (`{U}{B}`, `Coverage::Partial`): enchantment with a `Pay 1
/// life` activated ability targeting any permanent.
#[test]
fn spatial_binding_casts_and_activates_pay_one_life_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), swamp(), grizzly_bears()])
        .hand(0, &[spatial_binding()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, spatial_binding());
    pass_until(&mut engine, stack_is_empty);
    let binding = on_battlefield(&engine, p0, spatial_binding()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: binding,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
}
