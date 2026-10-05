//! Golgothian Sylex — {4} — Artifact
//! Oracle: {1}, {T}: Each nontoken permanent with a name originally printed in the Antiquities expansion is sacrificed by its controller.
//! Set: ATQ #51 — Antiquities | Scryfall ID: 856be1dd-a20b-49c2-be9d-7db76c7efd8b | Oracle ID: 95197f8f-c24e-4d13-a106-7186b3cc59e0
// PARTIAL — the sacrifice ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOLGOTHIAN_SYLEX,
    oracle_id = "95197f8f-c24e-4d13-a106-7186b3cc59e0",
    scryfall_id = "856be1dd-a20b-49c2-be9d-7db76c7efd8b",
    faces = &[face!(
        name = "Golgothian Sylex",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Filter names a set of printings and no Effect sacrifices every \
         permanent a filter matches"
    ),
    // NOT SUPPORTED: "{1}, {T}: Each nontoken permanent with a name originally
    // printed in the Antiquities expansion is sacrificed by its controller."
    // — the cost is ordinary, but no `Filter` reaches a card's printings
    // (`Filter::Named` matches one name and nothing expands a set of them),
    // and no `Effect` sacrifices every permanent a filter matches:
    // `Effect::SacrificeFilter` buys one per player, their choice. So the
    // ability comes off the card.
    abilities = &[],
);
