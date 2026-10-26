//! `cards/creatures/mv_8/tidal_kraken.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "104a8f76-d178-4a33-bb59-e8767da3d982"
fn tidal_kraken() -> CardIndex {
    card_index("104a8f76-d178-4a33-bb59-e8767da3d982")
}

/// `Tidal Kraken` (`Coverage::Implemented`):
/// "This creature can't be blocked."
#[test]
fn tidal_kraken_cannot_be_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9709, island())
        .battlefield(0, &[tidal_kraken()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let kraken = on_battlefield(&engine, p0, tidal_kraken()).expect("Tidal Kraken");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(kraken, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);

    assert_eq!(
        engine.state().players[1].life,
        14,
        "Tidal Kraken deals 6 combat damage because it can't be blocked"
    );
}
