//! Gauntlets of Chaos — {5} — Artifact
//! Oracle: {5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them.
//! Set: ME3 #196 — Masters Edition III | Scryfall ID: 8ea1b001-1eb6-4bb9-b8f3-1aaea14a9a13 | Oracle ID: 924ef69f-9977-439c-ace9-353b429e4be6
// PARTIAL — nothing is built: the exchange cannot be restricted to
// permanents that share a type, and the Auras would not be destroyed.
// NOT SUPPORTED: "{5}, Sacrifice this artifact: Exchange control of target
// artifact, creature, or land you control and target permanent an opponent
// controls that shares one of those types with it. If those permanents are
// exchanged this way, destroy all Auras attached to them." — no
// `TargetSpec` constrains the second target to share a card type with the
// first, and no effect destroys the Auras attached to the exchanged
// permanents; `Effect::ExchangeControl` alone would exchange any two
// permanents and leave their Auras attached.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAUNTLETS_OF_CHAOS,
    oracle_id = "924ef69f-9977-439c-ace9-353b429e4be6",
    scryfall_id = "8ea1b001-1eb6-4bb9-b8f3-1aaea14a9a13",
    faces = &[face!(
        name = "Gauntlets of Chaos",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no target constraint makes the second permanent share a type with the \
         first, and no effect destroys the Auras attached to them",
    ),
);
