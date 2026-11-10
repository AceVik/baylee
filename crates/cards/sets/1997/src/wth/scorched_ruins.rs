//! Scorched Ruins — (no cost) — Land
//! Oracle: If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {C}{C}{C}{C}.
//! Set: WTH #166 — Weatherlight | Scryfall ID: 75a4e843-937c-47fb-8768-0f42c5cb4e4f | Oracle ID: 6ee68855-c8c5-422b-88da-163c09a96416

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCORCHED_RUINS,
    oracle_id = "6ee68855-c8c5-422b-88da-163c09a96416",
    scryfall_id = "75a4e843-937c-47fb-8768-0f42c5cb4e4f",
    faces = &[face!(
        name = "Scorched Ruins",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::SacrificeOrGraveyard {
            filter: &Filter::And(&[Filter::ControlledByYou, Filter::LAND, Filter::Untapped]),
            count: 2
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 4)])],
);
