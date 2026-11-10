//! `cards/instants/mv_3/carrion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const P0: PlayerId = PlayerId::new(0);

fn carrion() -> CardIndex {
    card_index("84ca8d74-95ec-4a9c-8793-7a195ad28be9")
}

/// Carrion (84ca8d74-95ec-4a9c-8793-7a195ad28be9) — {1}{B}{B} Instant:
/// "As an additional cost to cast this spell, sacrifice a creature. Create X
/// 0/1 black Insect creature tokens, where X is the sacrificed creature's power."
#[test]
fn carrion_sacrifices_creature_and_creates_insects_equal_to_its_power() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), HILL_GIANT])
        .hand(0, &[carrion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, carrion());
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

/// "As an additional cost ... sacrifice a creature": with no creature to
/// sacrifice the spell cannot be cast, even with the mana for it.
#[test]
fn carrion_cannot_be_cast_without_a_creature_to_sacrifice() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[carrion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, carrion()).unwrap();
    let legal = priority_offer(&engine);

    assert!(!legal.castable.contains(&card), "{legal:?}");
    assert!(!legal.payable.contains(&card), "{legal:?}");
}
