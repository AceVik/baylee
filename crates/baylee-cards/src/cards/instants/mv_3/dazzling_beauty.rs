//! Dazzling Beauty — {2}{W} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: Target unblocked attacking creature becomes blocked. (This spell works on creatures that can't be blocked.)
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #8 — Mirage | Scryfall ID: ad0ece98-5506-4a18-b900-8d1a6cd87385 | Oracle ID: de96ef93-a2e0-409d-9d25-fcb2405039b5
// NOT SUPPORTED: Making an unblocked attacking creature become blocked without a blocker (CR 509.1h, triggering "becomes blocked" abilities and changing its blocked status for filters) is not yet a dedicated `Effect` variant.

use baylee_cards_dsl::prelude::*;

const UNBLOCKED_ATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Attacking, Filter::Unblocked]);

card!(
    index = index::DAZZLING_BEAUTY,
    oracle_id = "de96ef93-a2e0-409d-9d25-fcb2405039b5",
    scryfall_id = "ad0ece98-5506-4a18-b900-8d1a6cd87385",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Making an unblocked attacking creature become blocked without a blocker (CR 509.1h, triggering 'becomes blocked' abilities and changing its blocked status for filters) is not yet a dedicated `Effect` variant; in play, Dazzling Beauty can only be cast during the declare blockers step targeting an unblocked attacking creature, causes that creature to assign no combat damage this turn (`Modifier::AssignsNoCombatDamage`, matching CR 510.1c for a blocked creature with no blockers), and draws a card at the beginning of the next turn's upkeep.",
    ),
    faces = &[face!(
        name = "Dazzling Beauty",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AssignsNoCombatDamage,
                Duration::UntilEndOfTurn,
            ),
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &UNBLOCKED_ATTACKING_CREATURE
        ))),
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers)),
    )],
);
