//! `cards/instants/mv_3/carrion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CARRION, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);

/// Carrion (84ca8d74-95ec-4a9c-8793-7a195ad28be9) — {1}{B}{B} Instant:
/// "As an additional cost to cast this spell, sacrifice a creature. Create X
/// 0/1 black Insect creature tokens, where X is the sacrificed creature's power."
#[test]
fn carrion_sacrifices_creature_and_creates_insects_equal_to_its_power() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), HILL_GIANT])
        .hand(0, &[CARRION])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, CARRION);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, HILL_GIANT).is_none());
    let insects = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == P0 && obj.token.is_some() && pt(&engine, id) == (0, 1)
        })
        .count();
    assert_eq!(insects, 3);
}
