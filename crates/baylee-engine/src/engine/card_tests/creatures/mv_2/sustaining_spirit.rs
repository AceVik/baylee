//! `cards/creatures/mv_2/sustaining_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sustaining_spirit() -> CardIndex {
    card_index("683c81ae-38cd-4db1-b7e7-a09d34b6431e")
}

/// Sustaining Spirit ({1}{W} 0/3 Creature — Angel Spirit):
/// - Cumulative upkeep {1}{W}
/// - "Damage that would reduce your life total to less than 1 reduces it to 1 instead."
#[test]
fn sustaining_spirit_floors_damage_life_loss_at_one_and_has_cumulative_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[sustaining_spirit(), plains(), plains(), mountain()],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);

    // Upkeep: pay cumulative upkeep {1}{W} for the first age counter.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    engine.dev_state_mut(p0).unwrap().players[0].life = 2;
    engine
        .dev_state_mut(p0)
        .unwrap()
        .players[0]
        .mana_pool
        .add(ManaColor::Red, 1);
    engine.refresh_offer();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 3 damage from Lightning Bolt at 2 life reduces P0's life total to 1 instead of -1.
    assert_eq!(engine.state().players[0].life, 1);
}
