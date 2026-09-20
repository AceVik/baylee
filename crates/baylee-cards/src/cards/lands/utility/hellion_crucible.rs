//! Hellion Crucible — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{R}, {T}: Put a pressure counter on this land.
//! Oracle: {1}{R}, {T}, Remove two pressure counters from this land and sacrifice it: Create a 4/4 red Hellion creature token with haste. (It can attack and {T} as soon as it comes under your control.)
//! Set: M13 #226 — Magic 2013 | Scryfall ID: ad8274ef-a46a-4f5f-8ad1-6ce828f24210 | Oracle ID: c238ef51-4b46-43d5-a70b-40270a96a1fd
// PARTIAL — {T}: Add {C} is built; the two pressure-counter clauses are
// dropped, for the reasons on the `// NOT SUPPORTED:` lines below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELLION_CRUCIBLE,
    oracle_id = "c238ef51-4b46-43d5-a70b-40270a96a1fd",
    scryfall_id = "ad8274ef-a46a-4f5f-8ad1-6ce828f24210",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "pressure counters have no assigned id in baylee_cards_dsl::counters, and no Hellion token is in the token ledger"
    ),
    faces = &[face!(name = "Hellion Crucible", types = TypeSet::LAND,),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{1}{R}, {T}: Put a pressure counter on this land." — a
// counter is named by an id assigned in `baylee_cards_dsl::counters`, and no
// `PRESSURE` id is assigned there; a card writes `counters::…` and never a
// bare `CounterKind::Custom(n)`, which is the collision that module exists to
// prevent.
// NOT SUPPORTED: "{1}{R}, {T}, Remove two pressure counters from this land
// and sacrifice it: Create a 4/4 red Hellion creature token with haste." —
// the same unassigned counter id, and no Hellion token exists in the ledger
// (`crate::tokens` / `crate::generated_tokens`), where a card must name one:
// a `TokenDef` literal in a card file has no id and reaches the table
// nameless as far as art is concerned.
