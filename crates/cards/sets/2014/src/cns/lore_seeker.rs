//! Lore Seeker — {2} — Artifact Creature — Construct
//! Oracle: Reveal this card as you draft it. After you draft this card, you may add a booster pack to the draft. (Your next pick is from that booster pack. Pass it to the next player and it's drafted this draft round.)
//! Set: CNS #62 — Conspiracy | Scryfall ID: 9b8c78c8-b3fa-4a67-bf6c-cf708dedf42e | Oracle ID: 19a3c505-b180-47cc-bef3-9e807a8a4a3d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LORE_SEEKER,
    oracle_id = "19a3c505-b180-47cc-bef3-9e807a8a4a3d",
    scryfall_id = "9b8c78c8-b3fa-4a67-bf6c-cf708dedf42e",
    faces = &[face!(
        name = "Lore Seeker",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
