//! `cards/enchantments/mv_3/winter_s_night.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SNOW_COVERED_FOREST;

const WINTER_S_NIGHT: &str = "e4f73802-b639-47ab-a6c6-f761afa4e00e";

#[test]
fn winter_s_night_doubles_snow_land_mana_and_suppresses_its_next_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index(WINTER_S_NIGHT);
    let snow_forest = SNOW_COVERED_FOREST;
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(0, &[mountain(), forest(), plains(), snow_forest])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let snow_id = on_battlefield(&engine, p0, snow_forest).unwrap();
    let plain_forest = on_battlefield(&engine, p0, forest()).unwrap();
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();
    let pln = on_battlefield(&engine, p0, plains()).unwrap();

    // Tap the non-snow lands to cast Winter's Night ({R}{G}{W}).
    for land in [mtn, plain_forest, pln] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let hand_id = in_hand(&engine, p0, card).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: hand_id })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Tap Snow-Covered Forest for mana: produces 2 {G} (1 + 1 from Winter's Night)
    // and suppresses its next untap step.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: snow_id })
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );

    // Advance through p1's turn to p0's next main phase:
    // the non-snow Forest untaps, while Snow-Covered Forest stays tapped.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(!is_tapped(&engine, plain_forest));
    assert!(is_tapped(&engine, snow_id));
}
