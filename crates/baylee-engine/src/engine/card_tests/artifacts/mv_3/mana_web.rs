//! `cards/artifacts/mv_3/mana_web.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MANA_WEB;

/// `Mana Web` ({3}, Artifact, Partial) taps all lands an opponent controls when
/// a land they control is tapped for mana.
#[test]
fn mana_web_taps_opponent_lands_when_opponent_taps_land_for_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[MANA_WEB])
        .battlefield(1, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, MANA_WEB).is_some());
    reach_their_main_phase(&mut engine, p1);

    let p1_land = on_battlefield(&engine, p1, plains()).unwrap();
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let all_tapped = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| engine.state().object(id).unwrap().controller == p1)
        .all(|&id| is_tapped(&engine, id));
    assert!(all_tapped);
}
