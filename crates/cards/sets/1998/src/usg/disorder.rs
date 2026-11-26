//! Disorder — {1}{R} — Sorcery
//! Oracle: Disorder deals 2 damage to each white creature and each player who controls a white creature.
//! Set: 7ED #179 — Seventh Edition | Scryfall ID: b6d11422-60a9-4386-8e7f-dd7dcdac58d8 | Oracle ID: 586da84b-1384-4da2-983d-340c22b73eb8
// IMPLEMENTED — deals 2 damage to each white creature and each player who controls a white creature.

use baylee_cards_dsl::prelude::*;

static WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::DISORDER,
    oracle_id = "586da84b-1384-4da2-983d-340c22b73eb8",
    scryfall_id = "b6d11422-60a9-4386-8e7f-dd7dcdac58d8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Disorder",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::damage_each(2, &WHITE_CREATURE),
        Effect::ForEachPlayer {
            players: PlayerRel::EachPlayer,
            effects: &[Effect::IfCondition {
                condition: Condition::AmountAtLeast(
                    Amount::ControlledCount {
                        who: PlayerRel::EventPlayer,
                        filter: &WHITE_CREATURE,
                    },
                    1,
                ),
                then: &[Effect::DealDamage {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::Player(PlayerRel::EventPlayer),
                }],
                otherwise: &[],
            }],
        },
    ])],
);
