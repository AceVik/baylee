//! Lightning Dart — {1}{R} — Instant
//! Oracle: Lightning Dart deals 1 damage to target creature. If that creature is white or blue, Lightning Dart deals 4 damage to it instead.
//! Set: INV #152 — Invasion | Scryfall ID: 54d05157-d154-4203-bf3e-add110cb1cee | Oracle ID: 7bc53740-86e7-4e9d-aab6-b5ae62a42133
// IMPLEMENTED — deals 1 damage to target creature, or 4 if white or blue.

use baylee_cards_dsl::prelude::*;

static WHITE_OR_BLUE: Filter = Filter::Or(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::LIGHTNING_DART,
    oracle_id = "7bc53740-86e7-4e9d-aab6-b5ae62a42133",
    scryfall_id = "54d05157-d154-4203-bf3e-add110cb1cee",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lightning Dart",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::IfCondition {
                condition: &Condition::ObjectMatches {
                    what: &TargetSpec::Object(&Filter::CREATURE),
                    filter: &WHITE_OR_BLUE,
                },
                then: &Amount::Fixed(4),
                otherwise: &Amount::Fixed(1),
            },
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
