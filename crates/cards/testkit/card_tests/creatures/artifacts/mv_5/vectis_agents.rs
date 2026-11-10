//! `cards/creatures/artifacts/mv_5/vectis_agents.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vectis_agents() -> CardIndex {
    card_index("fd9bb758-a763-46bf-9e56-4be26e271741")
}

/// Vectis Agents — "{U}{B}: This creature gets -2/-0 until end of turn and
/// can't be blocked this turn."
#[test]
fn vectis_agents_trades_two_power_for_evasion() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vectis_agents(), island(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let agents = on_battlefield(&engine, p0, vectis_agents()).expect("seated");
    assert_eq!(pt(&engine, agents), (4, 3));
    assert!(!keywords(&engine, agents).contains(KeywordSet::UNBLOCKABLE));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, vectis_agents(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, agents), (2, 3), "-2/-0");
    assert!(keywords(&engine, agents).contains(KeywordSet::UNBLOCKABLE));

    attack_alone(&mut engine, p0, agents, p1);
    through_combat(&mut engine);
    assert_eq!(life_of(&engine, 1), 18, "it connected for the reduced two");
}
