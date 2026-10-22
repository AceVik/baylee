//! `cards/creatures/mv_5/hakim_loreweaver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HAKIM_LOREWEAVER, HOLY_STRENGTH};

const P0: PlayerId = PlayerId::new(0);

/// Hakim, Loreweaver (ee17f31b-3a11-41e4-9b7a-633c1b51f113) — {3}{U}{U} 2/4:
/// "Flying. {U}{U}: Return target Aura card from your graveyard to the
/// battlefield attached to Hakim. Activate only during your upkeep and only if
/// Hakim isn't enchanted. {U}{U}, {T}: Destroy all Auras attached to Hakim."
#[test]
fn hakim_loreweaver_destroys_attached_auras() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[HAKIM_LOREWEAVER, plains(), island(), island()])
        .hand(0, &[HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hakim = on_battlefield(&engine, P0, HAKIM_LOREWEAVER).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HOLY_STRENGTH);
    aim_at(&mut engine, P0, hakim);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(hakim).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(6)));

    activate(&mut engine, P0, HAKIM_LOREWEAVER, 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, HOLY_STRENGTH).is_none());
    let chars_after = engine.state().object(hakim).unwrap().characteristics();
    assert_eq!(
        (chars_after.power, chars_after.toughness),
        (Some(2), Some(4))
    );
}
