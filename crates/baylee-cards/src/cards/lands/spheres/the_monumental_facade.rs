//! The Monumental Facade — (no cost) — Land — Sphere
//! Oracle: This land enters with two oil counters on it.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Remove an oil counter from this land: Put an oil counter on target artifact or creature you control. Activate only as a sorcery.
//! Set: ONE #255 — Phyrexia: All Will Be One | Scryfall ID: d6785057-0d06-4f91-b45f-c05f7c4e2b19 | Oracle ID: 63e8d282-d038-4a8c-a0cb-f51ddf87d8ea
// PARTIAL — written by hand: {T}: Add {C} is the card's whole mana line, and
// the two oil-counter sentences are not expressible today. See the NOT
// SUPPORTED notes beside the abilities.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_MONUMENTAL_FACADE,
    oracle_id = "63e8d282-d038-4a8c-a0cb-f51ddf87d8ea",
    scryfall_id = "d6785057-0d06-4f91-b45f-c05f7c4e2b19",
    faces = &[face!(
        name = "The Monumental Facade",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SPHERE],
    ),],
    coverage = Coverage::Partial(
        "the card's oil counters have no name in the DSL: an oil counter is a \
         CounterKind::Custom id, and the counters this pool needs are assigned as \
         constants in `counters` (QUEST, DEPLETION, MINING, STORAGE) — a bare Custom \
         number is the collision that module exists to prevent, so the land cannot \
         enter with two of them and cannot spend one"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "This land enters with two oil counters on it." —
        // EnterModifier::WithCounters { kind, amount } takes a CounterKind, and no
        // constant in `counters` names an oil counter.
        // NOT SUPPORTED: "{T}, Remove an oil counter from this land: Put an oil
        // counter on target artifact or creature you control. Activate only as a
        // sorcery." — CostPart::RemoveCounterSelf, Effect::AddCounter and the
        // sorcery-speed activation are all sayable; the counter is the same missing
        // id as the line above.
    ],
);
