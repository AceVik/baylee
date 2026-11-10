//! `cards/lands/gates/thran_portal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thran Portal: "This land enters tapped unless you control two or fewer other lands." / "As this land enters, choose a basic land type..."
/// When played while controlling three other lands, Thran Portal enters tapped.
#[test]
fn thran_portal_enters_tapped_with_three_other_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(309, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[thran_portal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, thran_portal());
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::land::FOREST),
        )
        .unwrap();
    assert!(entered_tapped(&engine, land));
}
