//! Llanowar Druid — {1}{G} — Creature — Elf Druid
//! Oracle: {T}, Sacrifice this creature: Untap all Forests.
//! Set: WTH #133 — Weatherlight | Scryfall ID: ffad279c-762a-42cf-ac20-f4e48734c194 | Oracle ID: b8cadeee-36b7-4a61-91ff-7d6c7c780464
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::LLANOWAR_DRUID,
    oracle_id = "b8cadeee-36b7-4a61-91ff-7d6c7c780464",
    scryfall_id = "ffad279c-762a-42cf-ac20-f4e48734c194",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Llanowar Druid",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::UntapAll { filter: &FOREST }]
    ),],
);
