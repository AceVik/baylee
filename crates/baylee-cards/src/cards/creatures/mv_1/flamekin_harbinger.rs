//! Flamekin Harbinger — {R} — Creature — Elemental Shaman
//! Oracle: When this creature enters, you may search your library for an Elemental card, reveal it, then shuffle and put that card on top.
//! Set: HOP #53 — Planechase | Scryfall ID: 2796424c-f6c5-4851-87fb-145a58fe1f60 | Oracle ID: d6585e30-4ca0-4701-b274-b24f3508dd97
// IMPLEMENTED — ETB trigger: the printed "you may search … put that card on top"
// is SearchLibrary with `optional` and a single TOP_OF_LIBRARY find; the reveal
// and the shuffle afterwards are derived by the engine, not declared.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLAMEKIN_HARBINGER,
    oracle_id = "d6585e30-4ca0-4701-b274-b24f3508dd97",
    scryfall_id = "2796424c-f6c5-4851-87fb-145a58fe1f60",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flamekin Harbinger",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::SearchLibrary {
            filter: &Filter::HasSubtype(subtypes::creature::ELEMENTAL),
            finds: &[Find::TOP_OF_LIBRARY],
            optional: true,
        }]
    )],
);
