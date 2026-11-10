//! Maddening Imp — {2}{B} — Creature — Imp
//! Oracle: Flying
//! Oracle: {T}: Non-Wall creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy each of those creatures that didn't attack this turn. Activate only during an opponent's turn and only before combat.
//! Set: TMP #143 — Tempest | Scryfall ID: dda08eb5-c75c-4c21-bfd1-1f04a3575241 | Oracle ID: 97dea59e-2090-4653-9ed9-86820a20605f
// PARTIAL — Forces the active opponent's non-Wall creatures to attack this turn; delayed destruction of the original affected creatures that did not attack cannot be bound to an iterated event object.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ACTIVE_NON_WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
    Filter::ControlledByActivePlayer,
]);

card!(
    index = index::MADDENING_IMP,
    oracle_id = "97dea59e-2090-4653-9ed9-86820a20605f",
    scryfall_id = "dda08eb5-c75c-4c21-bfd1-1f04a3575241",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Forces the active opponent's non-Wall creatures to attack this turn; delayed destruction of the original affected creatures that did not attack cannot be bound to an iterated event object."
    ),
    faces = &[face!(
        name = "Maddening Imp",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::IMP],
        power = Some(1),
        toughness = Some(1),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[
            Effect::continuous(
                &ACTIVE_NON_WALL,
                Modifier::AttacksEachCombat,
                Duration::UntilEndOfTurn,
            ),
            // NOT SUPPORTED: remembering the original affected non-Walls for next-end-step destruction of those that did not attack.
        ],
        condition = Some(Condition::All(&[
            Condition::OpponentsTurn,
            Condition::BeforeStep(StepKind::CombatBegin),
        ])),
    )],
);
