//! Tithe — {W} — Instant
//! Oracle: Search your library for a Plains card. If target opponent controls more lands than you, you may search your library for an additional Plains card. Reveal those cards, put them into your hand, then shuffle.
//! Set: VIS #23 — Visions | Scryfall ID: aae08938-e563-4322-b2eb-db81913ea730 | Oracle ID: 318304f5-0929-4cd3-97e6-08ac1ac48aa6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PLAINS_CARD: Filter = Filter::HasSubtype(subtypes::land::PLAINS);

card!(
    index = index::TITHE,
    oracle_id = "318304f5-0929-4cd3-97e6-08ac1ac48aa6",
    scryfall_id = "aae08938-e563-4322-b2eb-db81913ea730",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tithe",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::IfCondition {
            // "target opponent controls more lands than you": the targeted opponent's lands, not
            // "an opponent's", so at a table of three the other opponent does not count.
            condition: Condition::AmountAtLeast(
                Amount::Difference {
                    left: &Amount::ControlledCount {
                        who: PlayerRel::Chosen,
                        filter: &Filter::And(&[Filter::LAND, Filter::ControlledByOpponent]),
                    },
                    right: &Amount::ControlledCount {
                        who: PlayerRel::You,
                        filter: &Filter::LAND,
                    },
                },
                1,
            ),
            then: &[Effect::SearchLibrary {
                filter: &PLAINS_CARD,
                finds: &[Find::HAND, Find::HAND],
                optional: true,
            }],
            otherwise: &[Effect::SearchLibrary {
                filter: &PLAINS_CARD,
                finds: &[Find::HAND],
                optional: true,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
