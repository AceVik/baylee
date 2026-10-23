//! `cards/enchantments/auras/mv_3/wellspring.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wellspring() -> CardIndex {
    card_index("8650bd97-ecf6-420f-9b72-a749970afcd4")
}

/// Wellspring (`{1}{G}{W}`, `Coverage::Implemented`): Aura enchanting a land;
/// when it enters, gain control of enchanted land until end of turn, and at
/// the beginning of your upkeep, untap enchanted land and gain control of it
/// until end of turn.
#[test]
fn wellspring_gains_control_of_enchanted_land_on_etb_and_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), plains()])
        .hand(0, &[wellspring()])
        .battlefield(1, &[island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_island = on_battlefield(&engine, p1, island()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, wellspring());
    let menu = aim_at(&mut engine, p0, opp_island);
    assert!(
        !menu.contains(&their_bear),
        "enchant land: a creature is no target"
    );
    pass_until(&mut engine, stack_is_empty);

    // ETB trigger gave p0 control of the Island until end of turn.
    assert_eq!(engine.state().object(opp_island).unwrap().controller, p0);

    // On p1's turn, control returns to p1; p1 taps the Island.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().object(opp_island).unwrap().controller, p1);
    tap_all_mana(&mut engine, p1);
    assert!(is_tapped(&engine, opp_island));

    // On p0's next turn, Wellspring's upkeep trigger untaps the Island and gives p0 control.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().object(opp_island).unwrap().controller, p0);
    assert!(!is_tapped(&engine, opp_island));
    // The upkeep grant is also only until end of turn: back to p1 for theirs.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().object(opp_island).unwrap().controller, p1);
}
