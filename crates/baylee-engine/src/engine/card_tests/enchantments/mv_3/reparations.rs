//! `cards/enchantments/mv_3/reparations.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn reparations() -> CardIndex {
    card_index("3cb86deb-f4e5-41c5-bfcd-614a24f3499c")
}

/// Reparations (`{1}{W}{U}`, `Coverage::Partial`): triggers when an opponent
/// casts a spell and lets you optionally draw a card.
#[test]
fn reparations_triggers_on_opponent_spell_and_optionally_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), forest()])
        .battlefield(1, &[mountain()])
        .hand(0, &[reparations()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, reparations());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, reparations()).is_some());

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
