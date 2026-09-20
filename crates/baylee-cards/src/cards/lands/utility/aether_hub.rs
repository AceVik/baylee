//! Aether Hub — (no cost) — Land
//! Oracle: When this land enters, you get {E} (an energy counter).
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay {E}: Add one mana of any color.
//! Set: KLD #242 — Kaladesh | Scryfall ID: 25ea04d8-5d85-49d3-8d8d-7fe123d0ed6c | Oracle ID: 61c89b11-65c9-4fda-bbcd-d84de25df801
// PARTIAL — {T}: Add {C} is built; both {E} clauses are dropped, because the
// DSL can neither put a counter on a player nor charge one as a cost.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_HUB,
    oracle_id = "61c89b11-65c9-4fda-bbcd-d84de25df801",
    scryfall_id = "25ea04d8-5d85-49d3-8d8d-7fe123d0ed6c",
    faces = &[face!(name = "Aether Hub", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the two energy clauses: no effect puts a counter on a player and no \
         cost part pays one out of a player's pool",
    ),
    abilities = &[
        // NOT SUPPORTED: "When this land enters, you get {E} (an energy
        // counter)." — Effect::AddCounter puts counters on an object and
        // with no target falls back to the source, so the only thing it
        // could charge is the land itself; nothing moves a counter onto a
        // player, and CounterKind::Energy has no writer.
        // NOT SUPPORTED: "{T}, Pay {E}: Add one mana of any color." — the
        // cost list has no part that spends a counter out of a player's
        // pool: RemoveCounterSelf takes counters off the source permanent,
        // which is a land and never carries energy. Activating this without
        // the price would be an untapped land for any color every turn.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
