//! `cards/creatures/mv_4/graveborn_muse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Graveborn Muse (`d2a65ff4-ab82-42f7-9d90-feb29132121e`) — {2}{B}{B} Creature — Zombie Spirit 3/3:
/// "At the beginning of your upkeep, you draw X cards and lose X life, where X is the number of Zombies you control."
#[test]
fn graveborn_muse_draws_and_loses_life_for_each_zombie_you_control_on_upkeep() {
    let muse_card = card_index("d2a65ff4-ab82-42f7-9d90-feb29132121e");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[muse_card, index::SCATHE_ZOMBIES])
        .start();
    keep_mulligans(&mut engine);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_main_phase(&mut engine, p0);

    // 2 Zombies controlled (Graveborn Muse + Scathe Zombies): draw 2 cards and lose 2 life on turn 1 upkeep.
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
    let _ = p1;
}
