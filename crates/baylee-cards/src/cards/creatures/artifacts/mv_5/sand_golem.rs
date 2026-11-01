//! Sand Golem — {5} — Artifact Creature — Golem
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.
//! Set: MIR #318 — Mirage | Scryfall ID: 84e4a955-ce9a-4386-b6ac-c00fd25de882 | Oracle ID: 0f9b78ca-1556-4157-a91c-55276fd78127
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAND_GOLEM,
    oracle_id = "0f9b78ca-1556-4157-a91c-55276fd78127",
    scryfall_id = "84e4a955-ce9a-4386-b6ac-c00fd25de882",
    faces = &[face!(
        name = "Sand Golem",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
