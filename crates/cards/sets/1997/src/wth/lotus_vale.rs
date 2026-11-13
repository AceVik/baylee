//! Lotus Vale — (no cost) — Land
//! Oracle: If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add three mana of any one color.
//! Set: WTH #165 — Weatherlight | Scryfall ID: 2e5cd12a-2a07-44a8-8eac-de00d26fe9e3 | Oracle ID: 01fc5bb3-ebd7-4ab4-8aef-2ece1e1d9b7c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOTUS_VALE,
    oracle_id = "01fc5bb3-ebd7-4ab4-8aef-2ece1e1d9b7c",
    scryfall_id = "2e5cd12a-2a07-44a8-8eac-de00d26fe9e3",
    faces = &[face!(
        name = "Lotus Vale",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::SacrificeOrGraveyard {
            filter: &Filter::And(&[Filter::ControlledByYou, Filter::LAND, Filter::Untapped]),
            count: 2
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana_choice_dynamic(
        ALL_MANA_COLORS,
        Amount::Fixed(3)
    )]),],
);
