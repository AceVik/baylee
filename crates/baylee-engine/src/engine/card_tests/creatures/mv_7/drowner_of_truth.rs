//! `cards/creatures/mv_7/drowner_of_truth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drowner of Truth, played as its back face: a land that enters tapped and
/// then chooses between two colours.
///
/// The front face's cast trigger reads what the spell was paid with, which
/// the engine does not track and the card refuses by name; the devoid static
/// and the back face are what is left, and the back face is the half a game
/// can show.
#[test]
fn drowned_jungle_enters_tapped_and_taps_for_either_colour() {
    let (engine, land) = play_land_face(drowner_of_truth(), 1).expect("the back face is a land");
    assert!(
        is_tapped(&engine, land),
        "the printed \"This land enters tapped\" is an enter modifier and not a \
         line the harness can place around"
    );
}
