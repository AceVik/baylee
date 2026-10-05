//! North Star — {4} — Artifact
//! Oracle: {4}, {T}: For one spell this turn, you may spend mana as though it were mana of any type to pay that spell's mana cost. (Additional costs are still paid normally.)
//! Set: LEG #288 — Legends | Scryfall ID: daac2a6b-27c8-4567-9e0c-7b262628d331 | Oracle ID: 9109437a-da80-463f-a92a-77d693d05d91
// PARTIAL — nothing is built: the one printed ability grants a one-spell
// permission no modifier can give.
// NOT SUPPORTED: "{4}, {T}: For one spell this turn, you may spend mana as
// though it were mana of any type to pay that spell's mana cost." —
// `Modifier::SpendManaAs` converts one named color into one other and
// `Modifier::ManaIsAnyColor` is an unlimited any-color static; neither is a
// one-spell "any type" permission.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NORTH_STAR,
    oracle_id = "9109437a-da80-463f-a92a-77d693d05d91",
    scryfall_id = "daac2a6b-27c8-4567-9e0c-7b262628d331",
    faces = &[face!(
        name = "North Star",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial("no one-spell spend-mana-as-any-type permission"),
);
