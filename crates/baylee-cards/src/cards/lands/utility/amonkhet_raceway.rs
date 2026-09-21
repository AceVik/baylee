//! Amonkhet Raceway — (no cost) — Land
//! Oracle: Start your engines! (If you have no speed, it starts at 1. It increases once on each of your turns when an opponent loses life. Max speed is 4.)
//! Oracle: {T}: Add {C}.
//! Oracle: Max speed — {T}: Target creature gains haste until end of turn.
//! Set: DFT #248 — Aetherdrift | Scryfall ID: 4f312807-ea2a-4385-8774-4e23b4a5d4a6 | Oracle ID: fe174586-d36b-40f6-babd-1f98e76eec22
// PARTIAL — {T}: Add {C} only; the speed mechanic is not expressible.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Start your engines!" — nothing tracks a player's speed: there is no counter or
// state for it, no keyword bit to carry it, and no trigger for "an opponent loses life".
// NOT SUPPORTED: "Max speed — {T}: Target creature gains haste until end of turn" — Condition has no
// variant for "your speed is 4" (the closest, Condition::CountersOnSelf, reads counters off the
// source permanent and a player's speed is neither a counter nor on a permanent), and omitting the
// gate would be an ability the card does not print.

card!(
    index = index::AMONKHET_RACEWAY,
    oracle_id = "fe174586-d36b-40f6-babd-1f98e76eec22",
    scryfall_id = "4f312807-ea2a-4385-8774-4e23b4a5d4a6",
    faces = &[face!(name = "Amonkhet Raceway", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "start your engines! and the \"Max speed\" condition have no DSL vocabulary",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
