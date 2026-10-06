//! `cards/sorceries/mv_7/bloodsoaked_insight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bloodsoaked Insight` // `Sanguine Morass` (`Coverage::Partial`):
/// "This spell costs {1} less to cast for each 1 life your opponents have lost this turn.
/// Target opponent exiles the top three cards of their library. Until the end of your next turn,
/// you may play those cards. If you cast a spell this way, mana of any type can be spent to cast it. //
/// This land enters tapped. `{{T}}`: Add `{{B}}` or `{{R}}`."
///
/// Under `Coverage::Partial`, the front-face sorcery is omitted, while the back-face land
/// (`Sanguine Morass`) is implemented in full. The test plays the back face as a land, confirms it
/// enters tapped, advances to the next turn so it untaps, activates its mana ability, chooses
/// `ManaColor::Black`, and asserts that one black mana is produced.
#[test]
fn sanguine_morass_enters_tapped_and_taps_for_black_or_red_mana() {
    let (mut engine, land) =
        play_land_face(bloodsoaked_insight(), 1).expect("plays as Sanguine Morass");
    assert!(is_tapped(&engine, land), "Sanguine Morass enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, p0, bloodsoaked_insight(), 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected color choice for dual mana, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&ManaColor::Black), "offers black mana");
    assert!(options.contains(&ManaColor::Red), "offers red mana");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(is_tapped(&engine, land), "tapped to produce mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one black mana in pool"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "no red mana in pool"
    );
}
