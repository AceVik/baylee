//! Cogwork Librarian — {4} — Artifact Creature — Construct
//! Oracle: Draft this card face up.
//! Oracle: As you draft a card, you may draft an additional card from that booster pack. If you do, put this card into that booster pack.
//! Set: CNS #58 — Conspiracy | Scryfall ID: 453de664-0f09-4772-a626-51d58d1173f3 | Oracle ID: ec0d964e-ca2c-4252-8551-cf1916576653
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COGWORK_LIBRARIAN,
    oracle_id = "ec0d964e-ca2c-4252-8551-cf1916576653",
    scryfall_id = "453de664-0f09-4772-a626-51d58d1173f3",
    faces = &[face!(
        name = "Cogwork Librarian",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
