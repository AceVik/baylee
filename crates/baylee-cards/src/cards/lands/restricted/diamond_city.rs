//! Diamond City — (no cost) — Land
//! Oracle: This land enters with a shield counter on it. (If it would be dealt damage or destroyed, remove a shield counter from it instead.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Move a shield counter from this land onto target creature. Activate only if two or more creatures entered the battlefield under your control this turn.
//! Set: PIP #147 — Fallout | Scryfall ID: 3e9bd49a-e9f1-4543-b04a-777a9e5a55ec | Oracle ID: 88e29d50-1680-495d-be84-b92b4c9e636f
// PARTIAL — {T}: Add {C} is built; the two shield-counter clauses are NOT
// SUPPORTED and named below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "This land enters with a shield counter on it. (If it would
// be dealt damage or destroyed, remove a shield counter from it instead.)" —
// a shield counter is a counter kind the rules know (CR 122.1c) rather than a
// `counters::…` word, and `CounterKind` has no `Shield` variant, so
// `EnterModifier::WithCounters` has nothing to name and nothing in the engine
// reads the replacement it implies.
// NOT SUPPORTED: "{T}: Move a shield counter from this land onto target
// creature. Activate only if two or more creatures entered the battlefield
// under your control this turn." — no effect takes a counter off the source
// and puts it on a target (`AddCounter` only adds, `DrainAllCountersIntoSelf`
// is the only counter-removing effect), and no `Condition` counts permanents
// that entered the battlefield this turn.

card!(
    index = index::DIAMOND_CITY,
    oracle_id = "88e29d50-1680-495d-be84-b92b4c9e636f",
    scryfall_id = "3e9bd49a-e9f1-4543-b04a-777a9e5a55ec",
    faces = &[face!(name = "Diamond City", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no CounterKind variant for a shield counter (CR 122.1c), no effect that moves a counter from the source onto a target, and no Condition counting permanents that entered this turn — the land enters with no counter and its third ability is not written"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
