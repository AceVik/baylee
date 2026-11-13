//! Glyph of Doom — {B} — Instant
//! Oracle: Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn.
//! Set: LEG #100 — Legends | Scryfall ID: 332bfce9-052d-42e9-a407-4a1dd59e0f2a | Oracle ID: a3b3e02f-c75d-4cd6-8c57-d85ce00975fb
// IMPLEMENTED — the delayed destroy at this turn's next end of combat, about
// the chosen Wall: every creature that Wall blocked this turn (cast after
// combat it waits for nothing and does nothing).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Target Wall creature."
static WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

/// "All creatures that were blocked by that creature this turn": the delayed
/// trigger's reference is the Wall it is about.
static BLOCKED_BY_IT: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockedByReferenceThisTurn]);

card!(
    index = index::GLYPH_OF_DOOM,
    oracle_id = "a3b3e02f-c75d-4cd6-8c57-d85ce00975fb",
    scryfall_id = "332bfce9-052d-42e9-a407-4a1dd59e0f2a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Glyph of Doom",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::AtEndOfCombatThisTurn {
            about: TargetSpec::Object(&WALL),
            effects: &[Effect::destroy_all(&BLOCKED_BY_IT)],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&WALL)))
    )],
);
