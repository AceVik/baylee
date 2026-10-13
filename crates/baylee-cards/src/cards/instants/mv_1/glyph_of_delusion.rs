//! Glyph of Delusion — {U} — Instant
//! Oracle: Put X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains "This creature doesn't untap during your untap step if it has a glyph counter on it" and "At the beginning of your upkeep, remove a glyph counter from this creature."
//! Set: LEG #60 — Legends | Scryfall ID: ee39da13-4b8a-4796-a7c2-aaa11992d573 | Oracle ID: e23ba169-4eb5-4083-a199-ac7e3026e67f
// IMPLEMENTED — two targets: a creature some Wall blocked this turn, then a
// Wall that blocked that creature this turn. The creature gets glyph counters
// equal to its power and, indefinitely, the untap restriction and the upkeep
// counter removal (both read from its own controller's turn).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Wall": a creature with the subtype Wall.
const WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

/// "Target creature that target Wall blocked this turn": blocked by some Wall.
static BLOCKED_BY_A_WALL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::BlockedThisTurnBy(&WALL)]);

/// "Target Wall [that] blocked [that creature] this turn": read with the
/// first target as the reference.
static WALL_THAT_BLOCKED_IT: Filter = Filter::And(&[WALL, Filter::BlockedReferenceThisTurn]);

/// "This creature doesn't untap during your untap step if it has a glyph
/// counter on it."
static NO_UNTAP_WITH_GLYPH: Filter =
    Filter::And(&[Filter::This, Filter::HasCounter(counters::GLYPH)]);

card!(
    index = index::GLYPH_OF_DELUSION,
    oracle_id = "e23ba169-4eb5-4083-a199-ac7e3026e67f",
    scryfall_id = "ee39da13-4b8a-4796-a7c2-aaa11992d573",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Glyph of Delusion",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            // The first target is the reference, so X is its power as it is.
            Effect::AddCounter {
                kind: counters::GLYPH,
                amount: Amount::TargetPower
            },
            // Bound to the creature as this resolves; no end (CR 611.2a).
            Effect::continuous(
                &Filter::Reference,
                Modifier::GrantStatic {
                    filter: &NO_UNTAP_WITH_GLYPH,
                    modifier: &Modifier::DoesNotUntap,
                },
                Duration::Indefinitely
            ),
            Effect::continuous(
                &Filter::Reference,
                Modifier::GrantTriggered {
                    trigger: Trigger::StepBegin {
                        step: StepKind::Upkeep,
                        whose: PlayerRel::You,
                    },
                    effects: &[Effect::RemoveCounterSelf {
                        kind: counters::GLYPH,
                        n: 1
                    }],
                    target: None,
                },
                Duration::Indefinitely
            ),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&BLOCKED_BY_A_WALL))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&WALL_THAT_BLOCKED_IT)))
    )],
);
