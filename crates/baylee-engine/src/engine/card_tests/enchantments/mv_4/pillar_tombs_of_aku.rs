//! `cards/enchantments/mv_4/pillar_tombs_of_aku.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn pillar_tombs_of_aku() -> CardIndex {
    card_index("b8cdf79f-b247-4346-8122-9a2d7d23b3c3")
}

/// `Pillar Tombs of Aku` ({2}{B}{B}, World Enchantment, `Coverage::Partial`)
/// causes the active player at the beginning of their upkeep to lose 5 life if
/// they don't sacrifice a creature.
#[test]
fn pillar_tombs_of_aku_causes_active_player_without_creatures_to_lose_5_life_at_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[pillar_tombs_of_aku()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, pillar_tombs_of_aku());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, pillar_tombs_of_aku()).is_some());

    // Advance to P1's main phase; P1 has no creatures to sacrifice at upkeep and loses 5 life.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 15);
}
