//! `cards/creatures/mv_7/dragon_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dragon_mage() -> CardIndex {
    card_index("eab71e4e-27c3-4c41-b95f-259c1d14b97a")
}

/// Dragon Mage — a 5/5 flier for {5}{R}{R}.
#[test]
fn dragon_mage_costs_seven_and_flies() {
    let mut engine = table7(&lots(mountain(), 7), &[dragon_mage()], &[]);
    let mage = cast_and_resolve(&mut engine, dragon_mage());
    assert_body(&engine, mage, (5, 5), KeywordSet::FLYING);
}

/// "Whenever this creature deals combat damage to a player, each player
/// discards their hand, then draws seven cards." The cards each player held
/// are in their graveyards afterwards and both hands hold seven.
#[test]
fn dragon_mage_wheels_both_hands_on_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragon_mage()])
        .hand(0, &[mountain(), mountain()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mage = on_battlefield(&engine, p0, dragon_mage()).expect("out");
    let old0 = engine.state().zones.list(ZoneLocation::Hand(p0)).to_vec();
    let old1 = engine.state().zones.list(ZoneLocation::Hand(p1)).to_vec();
    assert!(!old0.is_empty() && !old1.is_empty());

    fight(&mut engine, &[mage], &[]);
    assert_eq!(life_of(&engine, 1), 15, "5 combat damage");
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 7);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 7);
    let gy0 = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .to_vec();
    let gy1 = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .to_vec();
    assert!(
        old0.iter().all(|c| gy0.contains(c)),
        "p0's old hand discarded"
    );
    assert!(
        old1.iter().all(|c| gy1.contains(c)),
        "p1's old hand discarded"
    );
}
