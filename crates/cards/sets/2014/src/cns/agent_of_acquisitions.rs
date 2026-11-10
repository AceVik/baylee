//! Agent of Acquisitions — {2} — Artifact Creature — Construct
//! Oracle: Draft this card face up.
//! Oracle: Instead of drafting a card from a booster pack, you may draft each card in that booster pack, one at a time. If you do, turn this card face down and you can't draft cards for the rest of this draft round. (You may look at booster packs passed to you.)
//! Set: CNS #54 — Conspiracy | Scryfall ID: c57def6f-cc2a-4aec-927b-6a561d9fa7a1 | Oracle ID: 19047c4b-0106-455d-ab71-68cabfae7404
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AGENT_OF_ACQUISITIONS,
    oracle_id = "19047c4b-0106-455d-ab71-68cabfae7404",
    scryfall_id = "c57def6f-cc2a-4aec-927b-6a561d9fa7a1",
    faces = &[face!(
        name = "Agent of Acquisitions",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
