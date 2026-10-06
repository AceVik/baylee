//! Colours, identity, rooms, power and toughness.

use super::*;

/// A `{4}` artifact that taps for `{U}` and calls itself colorless.
///
/// Machine God's Effigy, as it actually sat in the pool. The counter-test
/// for [`no_card_hides_a_color_it_prints`], and the reason that lint is
/// worth having beside `xtask validate`'s comparison against the
/// printing: the card is its own evidence, so nothing has to be fetched
/// and the 102 double-faced cards whose payload the cache files under a
/// two-face slug are reached like any other.
fn effigy(identity: ColorSet) -> CardDef {
    use crate::dsl::ability::{ActivationLimit, ActivationTiming, ActivationZone};
    static BLUE: [Effect; 1] = [Effect::mana(ManaColor::Blue, 1)];
    static TAPS_FOR_BLUE: [AbilityDef; 1] = [AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::TAP,
        effects: &BLUE,
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    }];
    static FACES: [FaceDef; 1] = [FaceDef {
        name: "Machine God's Effigy",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        ..FaceDef::DEFAULT
    }];
    CardDef {
        faces: &FACES,
        abilities: &TAPS_FOR_BLUE,
        color_identity: identity,
        ..CardDef::DEFAULT
    }
}

#[test]
fn the_identity_lint_catches_the_effigy_it_was_written_for() {
    assert_eq!(
        identity_gap(&effigy(ColorSet::EMPTY)),
        ColorSet::of(Color::Blue),
        "a colorless artifact that taps for blue was read as colorless"
    );
    assert!(
        identity_gap(&effigy(ColorSet::of(Color::Blue))).is_empty(),
        "the same card with its identity declared is not a fault"
    );
}

/// The other two places a color is printed, each on its own.
///
/// [`effigy`] only proves the card-level ability list, and a walk that
/// had stopped reading either of these would keep passing the pool: 557
/// of the 1365 show a color somewhere, and almost all of them show it in
/// a mana cost.
#[test]
fn a_color_counts_wherever_the_face_prints_it() {
    static COST: [FaceDef; 1] = [FaceDef {
        name: "a black creature",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::CREATURE,
        power: Some(2),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }];
    static GREEN: [Effect; 1] = [Effect::mana(ManaColor::Green, 1)];
    static TAPS_FOR_GREEN: [AbilityDef; 1] = [AbilityDef::Activated {
        cost: crate::dsl::cost::Cost::TAP,
        effects: &GREEN,
        targets: None,
        second_targets: None,
        timing: crate::dsl::ability::ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: crate::dsl::ability::ActivationZone::Battlefield,
        limit: crate::dsl::ability::ActivationLimit::Unlimited,
        cost_reduction: None,
    }];
    static ON_THE_FACE: [FaceDef; 1] = [FaceDef {
        name: "a land that taps for green",
        types: TypeSet::LAND,
        abilities: &TAPS_FOR_GREEN,
        ..FaceDef::DEFAULT
    }];
    static A_DRAW: [Effect; 1] = [Effect::DrawCards {
        amount: crate::dsl::effect::Amount::Fixed(1),
    }];
    static PAID_IN_RED: [AbilityDef; 1] = [AbilityDef::Activated {
        cost: crate::dsl::cost::Cost {
            mana: baylee_core::mana!("{R}"),
            parts: &[],
        },
        effects: &A_DRAW,
        targets: None,
        second_targets: None,
        timing: crate::dsl::ability::ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: crate::dsl::ability::ActivationZone::Battlefield,
        limit: crate::dsl::ability::ActivationLimit::Unlimited,
        cost_reduction: None,
    }];
    assert_eq!(
        identity_gap(&CardDef {
            faces: &COST,
            ..CardDef::DEFAULT
        }),
        ColorSet::of(Color::Black),
        "a face's own mana cost went unread"
    );
    assert_eq!(
        identity_gap(&CardDef {
            faces: &ON_THE_FACE,
            ..CardDef::DEFAULT
        }),
        ColorSet::of(Color::Green),
        "an ability written on the face rather than beside it went unread"
    );
    assert_eq!(
        identity_gap(&CardDef {
            abilities: &PAID_IN_RED,
            ..CardDef::DEFAULT
        }),
        ColorSet::of(Color::Red),
        "a color paid in an activation cost is a symbol in the rules text"
    );
}

/// The exemption, both ways round: five colors offered is "any color"
/// and prints no symbol, four is a card naming four symbols.
#[test]
fn any_color_is_colorless_and_a_short_list_is_not() {
    static FOUR: [ManaColor; 4] = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
    ];
    assert!(
        mana_symbol_colors(&Effect::mana_of_any_color()).is_empty(),
        "\"add one mana of any color\" is a sentence with no mana symbol \
         in it (CR 903.4); Command Tower is a colorless card"
    );
    assert!(
        mana_symbol_colors(&Effect::mana_commander_identity()).is_empty(),
        "a commander's identity is not this card's"
    );
    assert_eq!(
        mana_symbol_colors(&Effect::mana_choice(&FOUR)),
        ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black, Color::Red]),
        "four named colors are four printed symbols"
    );
    assert_eq!(
        mana_symbol_colors(&Effect::mana(ManaColor::Colorless, 1)),
        ColorSet::EMPTY,
        "colorless is the absence of a color, not a sixth one"
    );
}

/// No card in the pool prints a color its identity does not carry.
///
/// 557 of the 1365 show a color somewhere for it to read, measured
/// 2026-09-09; the rest are colorless cards and lands, which the sweep
/// visits and has nothing to say about.
#[test]
fn no_card_hides_a_color_it_prints() {
    let mut wrong = Vec::new();
    for def in crate::all() {
        let gap = identity_gap(def);
        if !gap.is_empty() {
            wrong.push(format!("{} is missing {gap:?}", def.name()));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} card(s) print a mana symbol in a color their \
         `color_identity` does not carry (CR 903.4), which is the color \
         a deckbuilder checks a commander's deck against.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// A Room with both doors unlocked has both halves' rules text
/// (CR 709.5), and the engine reads that off the card-level list
/// (`CardDef::door_abilities`), so the card-level list must be exactly
/// the halves' lists end to end, the left first. Written by hand, it can
/// drift: a sentence added to one half and not to the union would be in
/// play with one door open and gone with both. Equality, not "contains",
/// because an ability's index in the list is its identity on the stack.
#[test]
fn a_rooms_card_list_is_its_doors_lists_end_to_end() {
    let mut rooms = 0_usize;
    let mut wrong = Vec::new();
    for def in crate::all() {
        if !def.has_shared_type_line() {
            continue;
        }
        rooms += 1;
        let halves: Vec<AbilityDef> = def
            .faces
            .iter()
            .flat_map(|face| face.abilities.iter().copied())
            .collect();
        if def.abilities != halves.as_slice() {
            wrong.push(def.name());
        }
    }
    assert!(
        wrong.is_empty(),
        "these Rooms' card-level `abilities` are not their halves' lists \
         end to end: {wrong:?}"
    );
    // Measured 29.09.2026: one Room in the pool, Walk-In Closet. A
    // reader of the shared type line that found none would pass the
    // sweep above with nothing in it.
    assert!(
        (1..=8).contains(&rooms),
        "{rooms} Rooms; `has_shared_type_line` is not reading the pool"
    );
}

#[test]
fn the_pt_lint_catches_both_halves_of_cr_208_1() {
    static VEHICLE: [baylee_core::ids::SubtypeId; 1] =
        [baylee_core::generated::subtypes::artifact::VEHICLE];
    let bodiless = FaceDef {
        types: TypeSet::CREATURE,
        ..FaceDef::DEFAULT
    };
    assert!(
        pt_fault(&bodiless).is_some(),
        "a creature with no power or toughness slipped through"
    );
    let half = FaceDef {
        types: TypeSet::CREATURE,
        power: Some(2),
        ..FaceDef::DEFAULT
    };
    assert!(
        pt_fault(&half).is_some(),
        "a creature with a power and no toughness slipped through"
    );
    let numbered_land = FaceDef {
        types: TypeSet::LAND,
        power: Some(3),
        toughness: Some(3),
        ..FaceDef::DEFAULT
    };
    assert!(
        pt_fault(&numbered_land).is_some(),
        "a land printing 3/3 without becoming a creature slipped through"
    );
    let vehicle = FaceDef {
        types: TypeSet::ARTIFACT,
        subtypes: &VEHICLE,
        power: Some(4),
        toughness: Some(3),
        ..FaceDef::DEFAULT
    };
    assert!(
        pt_fault(&vehicle).is_none(),
        "a Vehicle prints power and toughness and is not a creature \
         until it crews (CR 301.7)"
    );
    let bear = FaceDef {
        types: TypeSet::CREATURE,
        power: Some(2),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    };
    assert!(pt_fault(&bear).is_none(), "a 2/2 is not a fault");
}

/// Every creature in the pool has a body, and nothing else has one it
/// is not entitled to.
///
/// 98 faces carry power or toughness, measured 2026-09-09 — a small
/// number for 1365 cards because the pool is mostly lands, and no
/// smaller than the pool's creature count.
#[test]
fn a_creature_is_exactly_a_face_with_a_body() {
    let mut wrong = Vec::new();
    for def in crate::all() {
        for face in def.faces {
            if let Some(fault) = pt_fault(face) {
                wrong.push(format!("{} ({}) {fault}", def.name(), face.name));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} face(s) disagree with CR 208.1 about their own body.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
