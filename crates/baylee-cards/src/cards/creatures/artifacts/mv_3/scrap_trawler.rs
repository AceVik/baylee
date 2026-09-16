//! Scrap Trawler — {3} — Artifact Creature — Construct
//! Oracle: Whenever this creature dies or another artifact you control is put into a graveyard from the battlefield, return to your hand target artifact card in your graveyard with lesser mana value.
//! Set: MOC #373 — March of the Machine Commander | Scryfall ID: 614be454-3829-4c3b-9485-930755dfa16d | Oracle ID: 164f3f85-21fc-40b7-9871-4f303ba98428
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCRAP_TRAWLER,
    oracle_id = "164f3f85-21fc-40b7-9871-4f303ba98428",
    scryfall_id = "614be454-3829-4c3b-9485-930755dfa16d",
    faces = &[face!(
        name = "Scrap Trawler",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
