//! `cards/enchantments/auras/mv_1/samite_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn samite_blessing() -> CardIndex {
    card_index("a524ae69-2594-467d-af0a-27b9984d4300")
}

/// Samite Blessing (`a524ae69-2594-467d-af0a-27b9984d4300`) — {W} Enchantment — Aura (Partial):
/// "Enchant creature. Enchanted creature has '{T}: Prevent the next 1 damage that would be dealt to target creature this turn.'"
#[test]
fn samite_blessing_grants_enchanted_creature_tap_to_prevent_next_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                plains(),
                mountain(),
            ],
        )
        .hand(0, &[samite_blessing(), index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, samite_blessing());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    let idx = kit::offered(&engine, bear)[0];
    kit::activate_object(&mut engine, P0, bear, idx);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::HILL_GIANT).is_some(),
        "Hill Giant should survive Lightning Bolt after 1 damage is prevented"
    );
    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
}
