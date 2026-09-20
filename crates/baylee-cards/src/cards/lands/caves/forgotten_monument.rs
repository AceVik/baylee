//! Forgotten Monument — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: Other Caves you control have "{T}, Pay 1 life: Add one mana of any color."
//! Set: LCI #272 — The Lost Caverns of Ixalan | Scryfall ID: de8c1c02-e533-46b2-a3eb-91dff561854b | Oracle ID: 71393988-ad6f-43fd-9978-c0de15ae8e87
// PARTIAL — {T}: Add {C} works; the mana grant to other Caves is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FORGOTTEN_MONUMENT,
    oracle_id = "71393988-ad6f-43fd-9978-c0de15ae8e87",
    scryfall_id = "de8c1c02-e533-46b2-a3eb-91dff561854b",
    faces = &[face!(
        name = "Forgotten Monument",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Partial(
        "ability-granting statics are not supported (docs/card-dsl.md, M3+): other Caves you control never gain the \"{T}, Pay 1 life: Add one mana of any color\" ability",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: Other Caves you control have "{T}, Pay 1 life: Add
        // one mana of any color." — a static that grants an activated ability
        // to other objects is not expressible in a way the engine reads.
    ],
);
