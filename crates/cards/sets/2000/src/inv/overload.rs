//! Overload — {R} — Instant
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead.
//! Set: INV #157 — Invasion | Scryfall ID: c91fca91-7296-422e-b251-d571b710ff71 | Oracle ID: 07159efc-c69f-4164-a8ca-9da641dbf702
// IMPLEMENTED — kicker {2} expands the mana value threshold from 2 to 5 on resolution.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERLOAD,
    oracle_id = "07159efc-c69f-4164-a8ca-9da641dbf702",
    scryfall_id = "c91fca91-7296-422e-b251-d571b710ff71",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Overload",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
        additional_costs = &[cost!("{2}")],
    ),],
    abilities = &[spell!(
        &[Effect::IfKicked {
            then: &[Effect::IfTargetMatches {
                filter: &Filter::CmcAtMost(5),
                then: &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
            }],
            otherwise: &[Effect::IfTargetMatches {
                filter: &Filter::CmcAtMost(2),
                then: &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
    ),],
);
