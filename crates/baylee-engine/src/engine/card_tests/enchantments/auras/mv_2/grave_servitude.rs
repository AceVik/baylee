//! `cards/enchantments/auras/mv_2/grave_servitude.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index::{GRAVE_SERVITUDE, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);

/// Grave Servitude (8d275914-953c-4107-9f2b-1648e918c77f) — {1}{B} Aura:
/// "Flash. Enchant creature. Enchanted creature gets +3/-1 and is black."
#[test]
fn grave_servitude_grants_plus_three_minus_one_and_sets_color_to_black() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), GRIZZLY_BEARS])
        .hand(0, &[GRAVE_SERVITUDE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, GRAVE_SERVITUDE);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(bears).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(5), Some(1)));
    assert_eq!(chars.colors, ColorSet::from_slice(&[Color::Black]));
}
