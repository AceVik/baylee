//! `cards/creatures/mv_6/disciple_of_freyalise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Disciple of Freyalise, played as its back face: the land pays 3 life to
/// arrive untapped and then makes green.
///
/// The front face's enter trigger is off the card by name, so the back is
/// where this printing is testable at all — and it is the shape a modal
/// double-faced land carries: `EnterModifier::TappedOrPayLife(3)` asks, and
/// the answer decides whether the mana is available this turn.
#[test]
fn garden_of_freyalise_pays_three_life_to_arrive_untapped() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(disciple_of_freyalise(), 1).expect("the back face is a land");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(engine.state().players[0].life, 17, "and the 3 life is gone");
    // A *printed* mana ability is enumerated into `LegalActions::abilities`
    // like any other activated ability; `mana_abilities` is the CR 305.6 land
    // shortcut and what a continuous effect granted.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green in the pool"
    );
}
