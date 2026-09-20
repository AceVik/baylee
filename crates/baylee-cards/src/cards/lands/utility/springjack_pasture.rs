//! Springjack Pasture — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Create a 0/1 white Goat creature token.
//! Oracle: {T}, Sacrifice X Goats: Add X mana of any one color. You gain X life.
//! Set: C13 #326 — Commander 2013 | Scryfall ID: 035438b1-f794-41e5-9e2b-bc5136766cd5 | Oracle ID: 9eaadbbc-818b-4c21-9d4b-1bba48504d38
// PARTIAL — {T}: Add {C} only. The Goat token is not in the token registry and
// the X-Goat sacrifice has no counted cost, so each dropped clause carries a
// NOT SUPPORTED line where its ability would have been.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPRINGJACK_PASTURE,
    oracle_id = "9eaadbbc-818b-4c21-9d4b-1bba48504d38",
    scryfall_id = "035438b1-f794-41e5-9e2b-bc5136766cd5",
    faces = &[face!(name = "Springjack Pasture", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "'{4}, {T}: Create a 0/1 white Goat creature token' needs a Goat token the \
         registry does not have, and '{T}, Sacrifice X Goats: …' needs a counted \
         sacrifice cost and an amount read back from it",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{4}, {T}: Create a 0/1 white Goat creature token." —
        // Effect::CreateToken takes a &'static TokenDef, and the pool's registry
        // (crate::tokens / crate::generated_tokens) holds no Goat token, which a
        // card file may not declare for itself (no_card_file_defines_its_own_token).
        // NOT SUPPORTED: "{T}, Sacrifice X Goats: Add X mana of any one color. You
        // gain X life." — CostPart::Sacrifice names one permanent and nothing in
        // the DSL counts a sacrifice, and no Amount reads that number back for the
        // mana or for the life.
    ],
);
