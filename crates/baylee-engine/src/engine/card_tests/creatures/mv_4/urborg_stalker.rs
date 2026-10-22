//! `cards/creatures/mv_4/urborg_stalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn urborg_stalker() -> CardIndex {
    card_index("e5ef45e4-8c19-40bf-8756-c1b408dcb2fb")
}

/// `Urborg Stalker` ({3}{B}, 2/4 Horror) deals 1 damage to the active player at
/// the beginning of their upkeep if they control a nonblack, nonland permanent.
#[test]
fn urborg_stalker_deals_one_damage_at_upkeep_when_player_controls_nonblack_nonland_permanent() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[urborg_stalker()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "Urborg Stalker dealt 1 damage to opponent controlling a green creature at upkeep"
    );
}
