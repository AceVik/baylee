//! `cards/artifacts/mv_4/seer_s_sundial.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn seer_s_sundial() -> CardIndex {
    card_index("9929d7ba-cdc5-4099-a0ee-3a15073336f3")
}

/// "Landfall — Whenever a land you control enters, you may pay {2}. If you do, draw a card."
#[test]
fn seer_s_sundial_draws_a_card_on_landfall_when_two_mana_is_paid() {
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[seer_s_sundial(), forest(), forest()])
            .hand(0, &[forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
        if pay {
            tap_all_mana(&mut engine, p0);
        }
        play_land(&mut engine, p0, forest());
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { .. },
                    ..
                }
            )
        });
        engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        let expected_hand = if pay { hand_before } else { hand_before - 1 };
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
            expected_hand
        );
    }
}
