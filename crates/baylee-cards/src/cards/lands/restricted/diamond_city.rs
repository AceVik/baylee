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
// is the only counter-removing effect). The gate is nearly sayable and not
// quite: `Condition::ControlCount` over `Filter::EnteredThisTurn` counts the
// creatures that entered this turn *and are still here*, where the card
// counts arrivals — two creatures that entered, one of which has since died,
// are "two or more" on the card and one on that filter.

card!(
    index = index::DIAMOND_CITY,
    oracle_id = "88e29d50-1680-495d-be84-b92b4c9e636f",
    scryfall_id = "3e9bd49a-e9f1-4543-b04a-777a9e5a55ec",
    faces = &[face!(name = "Diamond City", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no CounterKind variant for a shield counter (CR 122.1c) and no effect that moves a counter from the source onto a target, so the land enters with no counter and its third ability is not written; its gate would also need a count of the creatures that entered this turn, where Condition::ControlCount over Filter::EnteredThisTurn counts only those still on the battlefield"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
