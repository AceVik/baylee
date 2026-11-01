//! Mulch — {1}{G} — Sorcery
//! Oracle: Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.
//! Set: DSC #189 — Duskmourn: House of Horror Commander | Scryfall ID: 9f6afdab-b41f-4713-b74a-e90e8bbbaf0a | Oracle ID: 12c03a03-36bf-487e-b8be-e55399525da5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MULCH,
    oracle_id = "12c03a03-36bf-487e-b8be-e55399525da5",
    scryfall_id = "9f6afdab-b41f-4713-b74a-e90e8bbbaf0a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mulch",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::RevealTopAndSort {
            filter: &Filter::LAND,
            matched: SearchDest::Hand,
            otherwise: SearchDest::Graveyard,
        },
        Effect::RevealTopAndSort {
            filter: &Filter::LAND,
            matched: SearchDest::Hand,
            otherwise: SearchDest::Graveyard,
        },
        Effect::RevealTopAndSort {
            filter: &Filter::LAND,
            matched: SearchDest::Hand,
            otherwise: SearchDest::Graveyard,
        },
        Effect::RevealTopAndSort {
            filter: &Filter::LAND,
            matched: SearchDest::Hand,
            otherwise: SearchDest::Graveyard,
        },
    ]),],
);
