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

#[test]
fn an_optional_cost_that_grants_flash_is_named_as_such() {
    let index = baylee_cards::decks::by_name("Rout").expect("a pool card");
    let option = CastModeDesc {
        index: 0,
        kind: CastModeKind::OptionalCost,
        cost: ManaCost::parse("{5}{W}{W}"),
    };
    let label = cast_mode_label(
        Some(baylee_view::CardIdentity {
            index,
            print: PrintRef::new(0),
            face: 0,
        }),
        &option,
    );
    assert_eq!(
        label,
        "Cast it with flash by paying its optional additional cost for {5}{W}{W}"
    );
}

#[test]
fn another_optional_cost_keeps_its_ordinary_label() {
    let index = baylee_cards::decks::by_name("Voltage Surge").expect("a pool card");
    let option = CastModeDesc {
        index: 0,
        kind: CastModeKind::OptionalCost,
        cost: ManaCost::parse("{1}{R}"),
    };
    let label = cast_mode_label(
        Some(baylee_view::CardIdentity {
            index,
            print: PrintRef::new(0),
            face: 0,
        }),
        &option,
    );
    assert_eq!(
        label,
        "Cast it with its optional additional cost for {1}{R}"
    );
}

#[test]
fn a_non_mana_flashback_names_its_printed_cost_instead_of_zero_mana() {
    let index = baylee_cards::decks::by_name("Lava Dart").expect("a pool card");
    let option = CastModeDesc {
        index: 0,
        kind: CastModeKind::Flashback,
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
    assert!(label.contains("Sacrifice a Mountain"), "{label}");
    assert!(!label.contains("{0}"), "{label}");
}
