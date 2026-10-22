//! `cards/instants/mv_2/suffocation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn suffocation() -> CardIndex {
    card_index("57aeb106-dc4f-4069-b596-f997263dc0cf")
}

/// Suffocation ({1}{U} Instant):
/// - "Suffocation deals 4 damage to the controller of the last red instant or sorcery spell that dealt damage to you this turn."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn suffocation_deals_four_damage_to_opponent_and_draws_on_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[suffocation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, suffocation());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 16);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
