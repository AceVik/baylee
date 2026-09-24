//! Springjack Pasture — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Create a 0/1 white Goat creature token.
//! Oracle: {T}, Sacrifice X Goats: Add X mana of any one color. You gain X life.
//! Set: C13 #326 — Commander 2013 | Scryfall ID: 035438b1-f794-41e5-9e2b-bc5136766cd5 | Oracle ID: 9eaadbbc-818b-4c21-9d4b-1bba48504d38
// PARTIAL — {T}: Add {C} and the Goat are built. The X-Goat sacrifice has no
// counted cost, so it carries a NOT SUPPORTED line where it would have been.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPRINGJACK_PASTURE,
    oracle_id = "9eaadbbc-818b-4c21-9d4b-1bba48504d38",
    scryfall_id = "035438b1-f794-41e5-9e2b-bc5136766cd5",
    faces = &[face!(name = "Springjack Pasture", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "'{T}, Sacrifice X Goats: …' needs a counted sacrifice cost and an \
         Amount that reads the number back for the mana and the life",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::GOAT_0_1_WHITE,
            }],
        ),
        // NOT SUPPORTED: "{T}, Sacrifice X Goats: Add X mana of any one color. You
        // gain X life." — CostPart::Sacrifice names one permanent and nothing in
        // the DSL counts a sacrifice, and no Amount reads that number back for the
        // mana or for the life.
    ],
);
