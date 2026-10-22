//! Tithe — {W} — Instant
//! Oracle: Search your library for a Plains card. If target opponent controls more lands than you, you may search your library for an additional Plains card. Reveal those cards, put them into your hand, then shuffle.
//! Set: VIS #23 — Visions | Scryfall ID: aae08938-e563-4322-b2eb-db81913ea730 | Oracle ID: 318304f5-0929-4cd3-97e6-08ac1ac48aa6
// PARTIAL: targets an opponent and searches your library for a Plains card (or up to two Plains cards if an opponent controls more lands than you), puts them into your hand, and shuffles; checking specifically the targeted opponent's land count in multiplayer is not yet modeled in Condition::OpponentControlsMore.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PLAINS_CARD: Filter = Filter::HasSubtype(subtypes::land::PLAINS);

card!(
    index = index::TITHE,
    oracle_id = "318304f5-0929-4cd3-97e6-08ac1ac48aa6",
    scryfall_id = "aae08938-e563-4322-b2eb-db81913ea730",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "targets an opponent and searches your library for a Plains card (or up to two Plains cards if an opponent controls more lands than you), puts them into your hand, and shuffles; checking specifically the targeted opponent's land count in multiplayer is not yet modeled in Condition::OpponentControlsMore",
    ),
    faces = &[face!(
        name = "Tithe",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::IfCondition {
            condition: Condition::OpponentControlsMore(&Filter::LAND),
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
