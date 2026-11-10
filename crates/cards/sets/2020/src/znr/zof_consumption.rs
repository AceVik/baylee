//! Zof Consumption // Zof Bloodbog — {4}{B}{B} — Sorcery // Land
//! Oracle: Each opponent loses 4 life and you gain 4 life.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #132 — Zendikar Rising | Scryfall ID: 98496d5b-1519-4f0c-8b46-0a43be643dfb | Oracle ID: d9f11985-e460-425d-b083-9cb0edf1983a
//! Face: Zof Consumption — {4}{B}{B} — Sorcery
//! Face: Zof Bloodbog —  — Land
// IMPLEMENTED — front drains each opponent for 4 and gains you 4; back
// enters tapped and prints its own {B} mana ability.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ZOF_CONSUMPTION,
    oracle_id = "d9f11985-e460-425d-b083-9cb0edf1983a",
    scryfall_id = "98496d5b-1519-4f0c-8b46-0a43be643dfb",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Zof Consumption",
            mana_cost = mana!("{4}{B}{B}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Zof Bloodbog",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[
        Effect::LoseLife {
            amount: Amount::Fixed(4),
            target: PlayerRel::EachOpponent,
        },
        Effect::gain_life(4),
    ])],
);
