//! Land Tax — {W} — Enchantment
//! Oracle: At the beginning of your upkeep, if an opponent controls more lands than you, you may search your library for up to three basic land cards, reveal them, put them into your hand, then shuffle.
//! Set: SOC #153 — Secrets of Strixhaven Commander | Scryfall ID: c70d3706-9d28-49ac-8718-4a516413e727 | Oracle ID: d2d9ecea-7925-420e-98b9-2f87f41f387c
// PARTIAL — the intervening "if an opponent controls more lands than you" has
// no Condition, so the search is off the card rather than firing ungated.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_TAX,
    oracle_id = "d2d9ecea-7925-420e-98b9-2f87f41f387c",
    scryfall_id = "c70d3706-9d28-49ac-8718-4a516413e727",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "no Condition compares an opponent's land count to yours, so the \
         intervening if cannot be written; an ungated trigger would search \
         every upkeep, strictly stronger than the printed card"
    ),
    faces = &[face!(
        name = "Land Tax",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// NOT SUPPORTED: "At the beginning of your upkeep, if an opponent controls
// more lands than you, you may search your library for up to three basic land
// cards, reveal them, put them into your hand, then shuffle." — no Condition
// compares an opponent's count to yours (`Condition::OpponentControlCount`
// takes a fixed threshold), and the search is left off rather than written
// without its gate.
