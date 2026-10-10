//! `cards/enchantments/mv_1/storm_world.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const STORM_WORLD: &str = "868f4ab2-a846-4ad0-8720-95fd234dd36b";

/// Each player's upkeep: 4 minus their hand, floored at zero.
#[test]
fn storm_world_deals_four_minus_the_active_players_hand() {
    let p1 = PlayerId::new(1);
    let storm = card_index(STORM_WORLD);
    for (cards, damage) in [(0, 4), (1, 3), (4, 0), (6, 0)] {
        let mut engine = Duel::new(1030, forest())
            .battlefield(0, &[storm])
            .hand(1, &vec![forest(); cards])
            .start();
        keep_mulligans(&mut engine);
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && e.state().turn.step == crate::turn::Step::Upkeep
        });
        pass_until(&mut engine, |e| {
            e.state().turn.step != crate::turn::Step::Upkeep
        });
        assert_eq!(engine.state().players[1].life, 20 - damage, "{cards} cards");
    }
}

/// It also fires on its controller's upkeep, damaging the controller.
#[test]
fn storm_world_also_damages_its_controller_at_their_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let storm = card_index(STORM_WORLD);
    let mut engine = Duel::new(1031, forest())
        .battlefield(0, &[storm])
        .hand(0, &[forest(); 2])
        .hand(1, &[forest(); 3])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    let before = engine.state().players[0].life;
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.step != crate::turn::Step::Upkeep
    });
    let expected = before - 4i32.saturating_sub(i32::try_from(hand).unwrap()).max(0);
    assert_eq!(engine.state().players[0].life, expected);
    assert!(
        expected < before,
        "the controller took damage (hand {hand})"
    );
}
