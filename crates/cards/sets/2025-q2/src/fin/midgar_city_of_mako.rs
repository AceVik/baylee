//! Midgar, City of Mako // Reactor Raid — (no cost) — Land — Town // Sorcery — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: You may sacrifice an artifact or creature. If you do, draw two cards. (Then exile this card. You may play the land later from exile.)
//! Set: FIN #286 — Final Fantasy | Scryfall ID: 8a837256-6bb4-4a60-962d-d2793548d26c | Oracle ID: 4e34a49d-f031-48ac-a458-97b79124b76c
//! Face: Midgar, City of Mako —  — Land — Town
//! Face: Reactor Raid — {2}{B} — Sorcery — Adventure

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIDGAR_CITY_OF_MAKO,
    oracle_id = "4e34a49d-f031-48ac-a458-97b79124b76c",
    scryfall_id = "8a837256-6bb4-4a60-962d-d2793548d26c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Midgar, City of Mako",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Reactor Raid",
            mana_cost = mana!("{2}{B}"),
            types = TypeSet::SORCERY,
            subtypes = &[subtypes::spell::ADVENTURE],
            adventure = true,
            abilities = &[spell!(&[Effect::PlayerMayPayCostThen {
                player: PlayerRel::You,
                cost: &CostPart::Sacrifice(&Filter::ARTIFACT_OR_CREATURE),
                effect: &Effect::draw(2),
            }])],
        ),
    ],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);
