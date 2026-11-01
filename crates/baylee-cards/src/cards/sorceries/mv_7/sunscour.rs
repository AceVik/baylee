//! Sunscour — {5}{W}{W} — Sorcery
//! Oracle: You may exile two white cards from your hand rather than pay this spell's mana cost.
//! Oracle: Destroy all creatures.
//! Set: CSP #19 — Coldsnap | Scryfall ID: 44c726db-a30a-4e76-9fbf-ec6d5cd7a1ba | Oracle ID: 155f14dd-1516-4b9f-ae39-0a6172c69a66
// IMPLEMENTED — two-card pitch cost and a complete creature sweeper.

use baylee_cards_dsl::prelude::*;

static WHITE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

card!(
    index = index::SUNSCOUR,
    oracle_id = "155f14dd-1516-4b9f-ae39-0a6172c69a66",
    scryfall_id = "44c726db-a30a-4e76-9fbf-ec6d5cd7a1ba",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sunscour",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::SORCERY,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&WHITE_CARD), ExileFromHand(&WHITE_CARD)),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(&[Effect::DestroyAll {
        filter: &Filter::CREATURE,
        no_regen: false,
    }])],
);
