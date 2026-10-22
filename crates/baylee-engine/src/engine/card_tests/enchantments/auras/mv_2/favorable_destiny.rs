//! `cards/enchantments/auras/mv_2/favorable_destiny.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{FAVORABLE_DESTINY, GRIZZLY_BEARS, SERRA_ANGEL};

const P0: PlayerId = PlayerId::new(0);

/// Favorable Destiny (ee04afb0-184f-4a13-8d44-0fbe121b1eff) — {1}{W} Aura:
/// "Enchant creature. Enchanted creature gets +1/+2 as long as it's white.
/// Enchanted creature has shroud as long as you control another creature."
#[test]
fn favorable_destiny_grants_plus_one_plus_two_and_shroud() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), SERRA_ANGEL, GRIZZLY_BEARS])
        .hand(0, &[FAVORABLE_DESTINY])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P0, SERRA_ANGEL).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, FAVORABLE_DESTINY);
    aim_at(&mut engine, P0, angel);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, angel), (5, 6));
    assert!(
        engine
            .state()
            .object(angel)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHROUD)
    );
}
