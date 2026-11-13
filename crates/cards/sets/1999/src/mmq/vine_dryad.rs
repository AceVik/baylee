//! Vine Dryad — {3}{G} — Creature — Dryad
//! Oracle: You may exile a green card from your hand rather than pay this spell's mana cost.
//! Oracle: Flash
//! Oracle: Forestwalk (This creature can't be blocked as long as defending player controls a Forest.)
//! Set: MMQ #284 — Mercadian Masques | Scryfall ID: fc9c9158-faed-42ae-9f6b-71dee49ff79f | Oracle ID: 990cd7de-66a2-4def-bf00-390b2682e995
// IMPLEMENTED — pitch alternative cost exiling a green card from your hand, Flash, and Forestwalk.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GREEN_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));

card!(
    index = index::VINE_DRYAD,
    oracle_id = "990cd7de-66a2-4def-bf00-390b2682e995",
    scryfall_id = "fc9c9158-faed-42ae-9f6b-71dee49ff79f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::FLASH.union(KeywordSet::FORESTWALK),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vine Dryad",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRYAD],
        power = Some(1),
        toughness = Some(3),
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&GREEN_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
);
