//! Pursuit of Knowledge — {3}{W} — Enchantment
//! Oracle: If you would draw a card, you may put a study counter on this enchantment instead.
//! Oracle: Remove three study counters from this enchantment, Sacrifice this enchantment: Draw seven cards.
//! Set: STH #10 — Stronghold | Scryfall ID: eaa577b1-2604-4065-a973-166521682a86 | Oracle ID: 7f5124a4-9ffd-475a-8b77-3ff59a9871a1
// PARTIAL: replacing card draws with putting a study counter on this enchantment is not supported; implements the activated ability to remove three study counters and sacrifice this enchantment to draw seven cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PURSUIT_OF_KNOWLEDGE,
    oracle_id = "7f5124a4-9ffd-475a-8b77-3ff59a9871a1",
    scryfall_id = "eaa577b1-2604-4065-a973-166521682a86",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "replacing card draws with putting a study counter on this enchantment is not supported; implements the activated ability to remove three study counters and sacrifice this enchantment to draw seven cards",
    ),
    faces = &[face!(
        name = "Pursuit of Knowledge",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(
            RemoveCounterSelf {
                kind: counters::STUDY,
                n: 3
            },
            SacrificeSelf
        ),
        &[Effect::draw(7)],
    ),],
);
