//! Siren's Call — {U} — Instant
//! Oracle: Cast this spell only during an opponent's turn, before attackers are declared.
//! Oracle: Creatures the active player controls attack this turn if able.
//! Oracle: At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn.
//! Set: 4ED #101 — Fourth Edition | Scryfall ID: 51832cfb-0a2e-4674-bb36-38027a71ac6d | Oracle ID: 269fc857-a052-4f0a-9759-467ccf42bebb

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SIREN_S_CALL,
    oracle_id = "269fc857-a052-4f0a-9759-467ccf42bebb",
    scryfall_id = "51832cfb-0a2e-4674-bb36-38027a71ac6d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Siren's Call",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            // A rule and not a characteristic, so it reaches a creature the
            // active player comes to control after this resolves (CR 611.2c).
            Effect::continuous(
                &Filter::And(&[Filter::CREATURE, Filter::ControlledByActivePlayer]),
                Modifier::AttacksEachCombat,
                Duration::UntilEndOfTurn
            ),
            Effect::AtNextEndStep {
                effects: &[Effect::destroy_all(&Filter::And(&[
                    Filter::CREATURE,
                    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
                    Filter::ControlledByActivePlayer,
                    Filter::ControlledSinceTurnBegan,
                    Filter::Not(&Filter::AttackedThisTurn),
                ]))],
            },
        ],
        condition = Some(Condition::All(&[
            Condition::OpponentsTurn,
            Condition::BeforeStep(StepKind::DeclareAttackers),
        ]))
    )],
);
