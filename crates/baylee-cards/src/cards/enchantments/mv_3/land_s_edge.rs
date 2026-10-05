//! Land's Edge — {1}{R}{R} — World Enchantment
//! Oracle: Discard a card: If the discarded card was a land card, this enchantment deals 2 damage to target player or planeswalker. Any player may activate this ability.
//! Set: CHR #52 — Chronicles | Scryfall ID: 41798dd9-8ce8-4642-89c2-7356ea129d4e | Oracle ID: 82d58f2d-66a4-4154-9826-01ca8e8d32d0
// PARTIAL — the discard ability is off the card: neither "if the discarded
// card was a land card" nor "any player may activate this ability" has a DSL
// shape, and an ungated, controller-only version is a different card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_S_EDGE,
    oracle_id = "82d58f2d-66a4-4154-9826-01ca8e8d32d0",
    scryfall_id = "41798dd9-8ce8-4642-89c2-7356ea129d4e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no Condition or Amount reads the card a cost discarded, so \"if the \
         discarded card was a land card\" cannot gate the damage, and no \
         activation permission offers an ability to any player but its \
         controller"
    ),
    faces = &[face!(
        name = "Land's Edge",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    // NOT SUPPORTED: "Discard a card: If the discarded card was a land card,
    // this enchantment deals 2 damage to target player or planeswalker. Any
    // player may activate this ability." — the discard itself is
    // `CostPart::Discard`, but nothing reads which card was discarded:
    // `Amount` has no discarded-card value and `Condition` no discarded-card
    // predicate, so the damage cannot be gated on the card's type. "Any
    // player may activate" is the second gap — no `AbilityDef` field names
    // another activator (Ifh-Bíff Efreet). An ungated, controller-only
    // version would deal 2 for every discarded card, so the ability is left
    // off entirely.
    abilities = &[],
);
