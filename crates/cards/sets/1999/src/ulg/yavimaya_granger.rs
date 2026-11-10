//! Yavimaya Granger — {2}{G} — Creature — Elf
//! Oracle: Echo {2}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Set: ULG #118 — Urza's Legacy | Scryfall ID: 05414a41-b50c-49b6-9c27-f3170017d9b0 | Oracle ID: fac9ff8d-8342-4099-b224-d48a044bcc9a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YAVIMAYA_GRANGER,
    oracle_id = "fac9ff8d-8342-4099-b224-d48a044bcc9a",
    scryfall_id = "05414a41-b50c-49b6-9c27-f3170017d9b0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Yavimaya Granger",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{2}{G}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: true,
            }],
        ),
    ],
);
