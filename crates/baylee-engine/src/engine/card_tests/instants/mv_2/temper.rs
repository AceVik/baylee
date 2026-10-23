//! `cards/instants/mv_2/temper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn temper() -> CardIndex {
    card_index("9cefff18-96b1-4090-93cc-1ca981a58fbd")
}

/// Temper (`9cefff18-96b1-4090-93cc-1ca981a58fbd`) — {X}{1}{W} Instant (Partial):
/// "Prevent the next X damage that would be dealt to target creature this turn."
#[test]
fn temper_prevents_next_x_damage_to_target_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
            ],
        )
        .hand(0, &[temper(), index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, temper());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears should survive 3 damage after Temper(X=2) prevents 2"
    );
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);
}
