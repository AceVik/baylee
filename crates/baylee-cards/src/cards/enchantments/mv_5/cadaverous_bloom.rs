//! Cadaverous Bloom — {3}{B}{G} — Enchantment
//! Oracle: Exile a card from your hand: Add {B}{B} or {G}{G}.
//! Set: MIR #258 — Mirage | Scryfall ID: c9bef70b-61c7-4df5-b4df-09cd6ab2015c | Oracle ID: fbb0f73b-5e30-4632-99c1-e49582e41f8d
// NOT SUPPORTED: Activated mana abilities with an exile-from-hand cost (`CostPart::ExileFromHand`) are not yet supported by `pay_cost` (`paid_by_the_casting_wizard`).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CADAVEROUS_BLOOM,
    oracle_id = "fbb0f73b-5e30-4632-99c1-e49582e41f8d",
    scryfall_id = "c9bef70b-61c7-4df5-b4df-09cd6ab2015c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage = Coverage::Partial(
        "Activated mana abilities with an exile-from-hand cost (`Exile a card from your hand: Add {B}{B} or {G}{G}`) are not yet supported by `pay_cost` (`paid_by_the_casting_wizard`); enters as a `{3}{B}{G}` enchantment with no abilities.",
    ),
    faces = &[face!(
        name = "Cadaverous Bloom",
        mana_cost = mana!("{3}{B}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
