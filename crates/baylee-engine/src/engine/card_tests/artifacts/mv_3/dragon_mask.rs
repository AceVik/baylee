//! `cards/artifacts/mv_3/dragon_mask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn dragon_mask() -> CardIndex {
    card_index("fce29dd8-23b1-465c-a977-526f716dc8f6")
}

#[test]
fn dragon_mask_pumps_your_creature_and_returns_it_to_hand_at_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[dragon_mask()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dragon_mask());
    pass_until(&mut engine, stack_is_empty);

    let mask = on_battlefield(&engine, p0, dragon_mask()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mask,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (4, 4));

    reach_their_main_phase(&mut engine, p1);
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .any(|&id| {
                engine
                    .state()
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == GRIZZLY_BEARS)
            })
    );
}
