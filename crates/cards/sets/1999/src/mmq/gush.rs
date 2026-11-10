//! Gush — {4}{U} — Instant
//! Oracle: You may return two Islands you control to their owner's hand rather than pay this spell's mana cost.
//! Oracle: Draw two cards.
//! Set: JVC #27 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: d4a3a921-3b7f-474c-b8c3-67a1a6ba5cc1 | Oracle ID: 16d8ee99-8ec8-429f-9ba7-818a74b6f910
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUSH,
    oracle_id = "16d8ee99-8ec8-429f-9ba7-818a74b6f910",
    scryfall_id = "d4a3a921-3b7f-474c-b8c3-67a1a6ba5cc1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gush",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[
                    CostPart::ReturnToHand(&Filter::And(&[
                        Filter::ControlledByYou,
                        Filter::HasSubtype(subtypes::land::ISLAND),
                    ])),
                    CostPart::ReturnToHand(&Filter::And(&[
                        Filter::ControlledByYou,
                        Filter::HasSubtype(subtypes::land::ISLAND),
                    ])),
                ],
            },
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[spell!(&[Effect::draw(2)])],
);
