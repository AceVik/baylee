//! `cards/creatures/mv_5/karplusan_yeti.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn karplusan_yeti() -> CardIndex {
    card_index("d6d84884-4a58-4187-aa5c-20b39b379590")
}

/// Karplusan Yeti (d6d84884-4a58-4187-aa5c-20b39b379590) — {3}{R}{R} 3/3 Creature — Yeti:
/// - "{T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature."
#[test]
fn karplusan_yeti_deals_power_to_target_creature_and_takes_targets_power_back() {
    let mut engine = lk::game(SEED, &[karplusan_yeti()], &[index::GRIZZLY_BEARS], &[], &[]);
    let yeti = lk::obj(&engine, P0, karplusan_yeti());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    activate(&mut engine, P0, karplusan_yeti(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, yeti));
    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "3 damage from Karplusan Yeti kills the 2/2 Grizzly Bears"
    );
    assert_eq!(
        engine.state().object(yeti).unwrap().damage,
        2,
        "Grizzly Bears deals its power (2) back to the 3/3 Karplusan Yeti"
    );
}

#[test]
fn karplusan_yeti_uses_pumped_power_and_dies_when_target_deals_lethal_back() {
    let mut engine = lk::game(
        SEED,
        &[karplusan_yeti(), forest()],
        &[index::SHIVAN_DRAGON],
        &[giant_growth()],
        &[],
    );
    let yeti = lk::obj(&engine, P0, karplusan_yeti());
    let dragon = lk::obj(&engine, P1, index::SHIVAN_DRAGON);

    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, giant_growth());
    aim_at(&mut engine, P0, yeti);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, yeti), (6, 6));

    activate(&mut engine, P0, karplusan_yeti(), 0);
    aim_at(&mut engine, P0, dragon);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, index::SHIVAN_DRAGON).is_some(),
        "pumped 6/6 Karplusan Yeti deals 6 damage and kills the 5/5 Shivan Dragon"
    );
    assert_eq!(
        engine.state().object(yeti).unwrap().damage,
        5,
        "Shivan Dragon deals 5 damage back to the 6/6 Karplusan Yeti"
    );
}
