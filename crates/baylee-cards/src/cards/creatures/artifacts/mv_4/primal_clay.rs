//! Primal Clay — {4} — Artifact Creature — Shapeshifter
//! Oracle: As this creature enters, it becomes your choice of a 3/3 artifact creature, a 2/2 artifact creature with flying, or a 1/6 Wall artifact creature with defender in addition to its other types. (A creature with defender can't attack.)
//! Set: A25 #228 — Masters 25 | Scryfall ID: 1e51f1b3-d8a3-4746-b0db-67d5ec71fb17 | Oracle ID: dccf2ca8-8c87-41c9-8373-351859396d05
// PARTIAL — the as-it-enters choice is off the card; no modifier offers a mode choice at entry.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRIMAL_CLAY,
    oracle_id = "dccf2ca8-8c87-41c9-8373-351859396d05",
    scryfall_id = "1e51f1b3-d8a3-4746-b0db-67d5ec71fb17",
    coverage = Coverage::Partial(
        "no as-it-enters modifier offers a mode choice, so the 3/3, 2/2 \
         flying and 1/6 defender shapes cannot be written"
    ),
    faces = &[face!(
        name = "Primal Clay",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    ),],
    // NOT SUPPORTED: "As this creature enters, it becomes your choice of a
    // 3/3 artifact creature, a 2/2 artifact creature with flying, or a 1/6
    // Wall artifact creature with defender in addition to its other types."
    // — `EnterModifier` has no mode choice, and a `modal_triggered!` would be
    // the wrong timing: a replacement applies as the permanent enters where a
    // trigger resolves afterwards, long after state-based actions have put
    // the 0/0 into its owner's graveyard. `CopyOnEnter` copies another
    // permanent rather than choosing a body.
    abilities = &[],
);
