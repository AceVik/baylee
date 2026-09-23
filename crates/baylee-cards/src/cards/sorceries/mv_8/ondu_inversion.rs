//! Ondu Inversion // Ondu Skyruins — {6}{W}{W} — Sorcery // Land
//! Oracle: Destroy all nonland permanents.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #30 — Zendikar Rising | Scryfall ID: b6e6be8c-41c3-4348-a8dd-b40ceb24e9b4 | Oracle ID: 15fc4e74-300e-4c2d-8ed7-004553b2f7c2
//! Face: Ondu Inversion — {6}{W}{W} — Sorcery
//! Face: Ondu Skyruins —  — Land
// IMPLEMENTED — front face is a one-sided wrath on nonland permanents;
// back face enters tapped and taps for {W}.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::ONDU_INVERSION,
    oracle_id = "15fc4e74-300e-4c2d-8ed7-004553b2f7c2",
    scryfall_id = "b6e6be8c-41c3-4348-a8dd-b40ceb24e9b4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Ondu Inversion",
            mana_cost = mana!("{6}{W}{W}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Ondu Skyruins",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::destroy_all(&Filter::NONLAND)])],
);
