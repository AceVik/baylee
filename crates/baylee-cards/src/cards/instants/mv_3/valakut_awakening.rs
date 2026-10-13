//! Valakut Awakening // Valakut Stoneforge — {2}{R} — Instant // Land
//! Oracle: Put any number of cards from your hand on the bottom of your library, then draw that many cards plus one.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #174 — Zendikar Rising | Scryfall ID: 228e551e-023a-4c9a-8f32-58dae6ffdf7f | Oracle ID: ff0ab867-b710-4b1a-baed-95fc3cf68f79
//! Face: Valakut Awakening — {2}{R} — Instant
//! Face: Valakut Stoneforge —  — Land
// IMPLEMENTED — front face: put any number of cards from your hand on the
// bottom of your library, then draw that many cards plus one; back face: a
// land that enters tapped and taps for {R}.

use baylee_cards_dsl::prelude::*;

/// The back face's mana ability sits on the face rather than on the card,
/// because the front face is a spell and prints no mana.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::VALAKUT_AWAKENING,
    oracle_id = "ff0ab867-b710-4b1a-baed-95fc3cf68f79",
    scryfall_id = "228e551e-023a-4c9a-8f32-58dae6ffdf7f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Valakut Awakening",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Valakut Stoneforge",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    abilities = &[spell!(&[
        Effect::PutAnyFromHandOnBottom,
        // "then draw that many cards plus one"
        Effect::DrawCards {
            amount: Amount::Plus {
                base: &Amount::ThisWay(ThisWay::PutIntoLibraryFromHand),
                offset: 1,
            },
        },
    ])],
);
