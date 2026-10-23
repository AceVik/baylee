//! Dazzling Beauty — {2}{W} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: Target unblocked attacking creature becomes blocked. (This spell works on creatures that can't be blocked.)
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #8 — Mirage | Scryfall ID: ad0ece98-5506-4a18-b900-8d1a6cd87385 | Oracle ID: de96ef93-a2e0-409d-9d25-fcb2405039b5
// NOT SUPPORTED: making an unblocked attacking creature become blocked (CR 509.1h): it does not trigger "becomes blocked" abilities and a blocked trampler would still assign its damage to the player. The spell stops the creature's combat damage instead, so it cannot target a creature with trample (that would be stronger than printed).

use baylee_cards_dsl::prelude::*;

const UNBLOCKED_ATTACKING_NONTRAMPLER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Unblocked,
    Filter::Not(&Filter::HasKeyword(KeywordSet::TRAMPLE)),
]);

card!(
    index = index::DAZZLING_BEAUTY,
    oracle_id = "de96ef93-a2e0-409d-9d25-fcb2405039b5",
    scryfall_id = "ad0ece98-5506-4a18-b900-8d1a6cd87385",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "can be cast only during the declare blockers step; the unblocked attacking creature without trample assigns no combat damage this turn (as if blocked by nothing) and you draw a card at the beginning of the next turn's upkeep; making it actually become blocked is not supported, so \"becomes blocked\" abilities do not trigger and a creature with trample cannot be targeted",
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
            &UNBLOCKED_ATTACKING_NONTRAMPLER
        ))),
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers)),
    )],
);
