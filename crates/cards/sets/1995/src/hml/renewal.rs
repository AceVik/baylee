//! Renewal — {2}{G} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a land.
//! Oracle: Search your library for a basic land card, put that card onto the battlefield, then shuffle.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: HML #93 — Homelands | Scryfall ID: ab998cd1-2f49-42e7-b889-c6717b0ce884 | Oracle ID: 8df46910-ee13-416e-8282-dfe85a90e6d5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RENEWAL,
    oracle_id = "8df46910-ee13-416e-8282-dfe85a90e6d5",
    scryfall_id = "ab998cd1-2f49-42e7-b889-c6717b0ce884",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Renewal",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::LAND)],
    ),],
    abilities = &[spell!(&[
        Effect::SearchLibrary {
            filter: &Filter::BASIC_LAND,
            finds: &[Find::BATTLEFIELD],
            optional: false,
        },
        Effect::AtNextTurnsUpkeep {
            effects: &[Effect::draw(1)],
        },
    ])],
);
