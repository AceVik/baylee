//! Forge of Heroes — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Choose target commander that entered this turn. Put a +1/+1 counter on it if it's a creature and a loyalty counter on it if it's a planeswalker.
//! Set: CMM #995 — Commander Masters | Scryfall ID: 5f11f053-5150-4a76-b2ca-9df3a0629911 | Oracle ID: 77807103-bcd5-479f-bedd-f5d97aa6d3d2
// PARTIAL — {T}: Add {C} is built; targeting a commander and choosing its counter by card type have no DSL shape.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORGE_OF_HEROES,
    oracle_id = "77807103-bcd5-479f-bedd-f5d97aa6d3d2",
    scryfall_id = "5f11f053-5150-4a76-b2ca-9df3a0629911",
    faces = &[face!(name = "Forge of Heroes", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Filter says \"is a commander\", so the target can't be named, and no Condition or effect branches on a target's card type, so the +1/+1-or-loyalty choice can't be said"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Choose target commander that entered this turn. Put a +1/+1 counter on it if it's a creature and a loyalty counter on it if it's a planeswalker."
    ],
);
