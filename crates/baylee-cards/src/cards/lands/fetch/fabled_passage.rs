//! Fabled Passage — (no cost) — Land
//! Oracle: {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Then if you control four or more lands, untap that land.
//! Set: FRC #71 — Reality Fracture Commander | Scryfall ID: 76edd22f-808e-4a7c-b941-13c0f5e30418 | Oracle ID: 0c85b8f7-0bd0-4680-9ec5-d4b110460a54
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FABLED_PASSAGE,
    oracle_id = "0c85b8f7-0bd0-4680-9ec5-d4b110460a54",
    scryfall_id = "76edd22f-808e-4a7c-b941-13c0f5e30418",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Fabled Passage", types = TypeSet::LAND,),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[
            Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            },
            Effect::IfCondition {
                condition: Condition::ControlCount(&Filter::LAND, 4),
                then: &[Effect::UntapFound],
                otherwise: &[]
            }
        ]
    )],
);
