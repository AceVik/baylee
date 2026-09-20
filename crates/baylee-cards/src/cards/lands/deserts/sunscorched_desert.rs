//! Sunscorched Desert — (no cost) — Land — Desert
//! Oracle: When this land enters, it deals 1 damage to target player or planeswalker.
//! Oracle: {T}: Add {C}.
//! Set: AKH #249 — Amonkhet | Scryfall ID: 405434c7-9206-45b7-af0f-d59aae294d39 | Oracle ID: 256b8c23-589e-429d-9e6e-433d55079eb4
// PARTIAL — the mana ability is written; the enter trigger is dropped rather
// than widened (see the NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "When this land enters, it deals 1 damage to target player
// or planeswalker." No TargetSpec names that set. `AnyTarget` (CR 115.4) is
// the nearest and is strictly wider — it also offers creatures and battles,
// so the land could burn a creature the printed card cannot point at — and
// every other spec is narrower: `Object(&Filter::PLANESWALKER)` reaches no
// player, `Player(…)` / `AnyPlayer` / `AnyOpponent` reach no planeswalker,
// and a `TargetReq` carries one spec with no union of two. The ability comes
// off the card rather than shipping a wider offer, and the {T} ability below
// is the whole of it.

card!(
    index = index::SUNSCORCHED_DESERT,
    oracle_id = "256b8c23-589e-429d-9e6e-433d55079eb4",
    scryfall_id = "405434c7-9206-45b7-af0f-d59aae294d39",
    faces = &[face!(
        name = "Sunscorched Desert",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Partial(
        "the enter trigger's target set, 'target player or planeswalker', has no \
         TargetSpec: AnyTarget also reaches creatures and battles, and every other \
         spec reaches one kind and not the other"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
