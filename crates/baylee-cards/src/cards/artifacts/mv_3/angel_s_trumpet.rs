//! Angel's Trumpet — {3} — Artifact
//! Oracle: All creatures have vigilance.
//! Oracle: At the beginning of each player's end step, tap all untapped creatures that player controls that didn't attack this turn. This artifact deals damage to the player equal to the number of creatures tapped this way.
//! Set: ULG #121 — Urza's Legacy | Scryfall ID: 0c7b248a-3c74-4592-b357-47989568298c | Oracle ID: f187f766-3696-419e-9865-58023a46446b
// PARTIAL: dealing damage equal to the number of creatures tapped this way is not in ThisWay; grants all creatures vigilance and taps all untapped creatures the active player controls that didn't attack this turn at the beginning of each player's end step.

use baylee_cards_dsl::prelude::*;

const IDLE_UNTAPPED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Untapped,
    Filter::Not(&Filter::AttackedThisTurn),
]);

card!(
    index = index::ANGEL_S_TRUMPET,
    oracle_id = "f187f766-3696-419e-9865-58023a46446b",
    scryfall_id = "0c7b248a-3c74-4592-b357-47989568298c",
    coverage = Coverage::Partial(
        "dealing damage equal to the number of creatures tapped this way is not in ThisWay; grants all creatures vigilance and taps all untapped creatures the active player controls that didn't attack this turn at the beginning of each player's end step",
    ),
    faces = &[face!(
        name = "Angel's Trumpet",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(
            Filter::CREATURE,
            Modifier::AddKeyword(KeywordSet::VIGILANCE)
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::TapAllOf {
                who: PlayerRel::ActivePlayer,
                filter: &IDLE_UNTAPPED_CREATURE,
            }],
        ),
    ],
);
