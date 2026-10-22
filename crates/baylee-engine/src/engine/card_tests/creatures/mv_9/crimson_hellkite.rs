//! `cards/creatures/mv_9/crimson_hellkite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CRIMSON_HELLKITE, SERRA_ANGEL};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Crimson Hellkite (37a1f340-10d5-407e-bf2b-cbbbb6cab985) — {6}{R}{R}{R} 6/6 Flying:
/// "{X}, {T}: This creature deals X damage to target creature. Spend only red mana on X."
#[test]
fn crimson_hellkite_deals_x_damage_to_target_creature() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                CRIMSON_HELLKITE,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let angel = on_battlefield(&engine, P1, SERRA_ANGEL).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, CRIMSON_HELLKITE, 0);
    engine.apply(P0, PlayerAction::ChooseNumber(4)).unwrap();
    aim_at(&mut engine, P0, angel);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, SERRA_ANGEL).is_none());
}
