//! How a cast mode is named to the seat.

use super::*;
use baylee_core::ids::PrintRef;

fn face_down(name: &str) -> String {
    let index = baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name}"));
    let card = baylee_view::CardIdentity {
        index,
        print: PrintRef::new(0),
        face: 0,
    };
    let option = CastModeDesc {
        index: 1,
        kind: CastModeKind::Disguise,
        cost: ManaCost::parse("{3}"),
    };
    cast_mode_label(Some(card), &option)
}

/// Bug hunt of 3.11.2026, game 1, q249: Exalted Angel, a morph card, was
/// offered as "Cast it face down with disguise for {3}". The engine has one
/// face-down mode for both keywords; the seat names the card's own.
#[test]
fn a_morph_card_is_cast_face_down_with_morph() {
    let label = face_down("Exalted Angel");
    assert!(label.contains("with morph"), "{label}");
    assert!(!label.contains("disguise"), "{label}");
}

#[test]
fn a_disguise_card_is_cast_face_down_with_disguise() {
    let label = face_down("Branch of Vitu-Ghazi");
    assert!(label.contains("with disguise"), "{label}");
}

#[test]
fn a_free_madness_cost_is_named_as_zero_mana() {
    let index = baylee_cards::decks::by_name("Basking Rootwalla").expect("a pool card");
    let option = CastModeDesc {
        index: 0,
        kind: CastModeKind::Madness,
        cost: ManaCost::ZERO,
    };
    let label = cast_mode_label(
        Some(baylee_view::CardIdentity {
            index,
            print: PrintRef::new(0),
            face: 0,
        }),
        &option,
    );
    assert_eq!(label, "Cast it for its madness cost {0}");
}
