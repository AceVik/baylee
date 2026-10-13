//! Glyph of Reincarnation — {G} — Instant
//! Oracle: Cast this spell only after combat.
//! Oracle: Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control.
//! Set: LEG #189 — Legends | Scryfall ID: a67e8214-a192-4143-9d5e-d0e254e1bf6e | Oracle ID: 93d9a1c6-9f6f-487f-a46c-bfd80946ccdf
// IMPLEMENTED — cast only after combat; destroys every creature the target
// Wall blocked this turn (no regeneration), then for each one that died puts
// a creature card from the graveyard of the player who controlled it as it
// was last blocked by that Wall onto the battlefield under its owner's control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Target Wall creature."
static WALL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::WALL),
]);

/// "All creatures that were blocked by target Wall this turn."
static BLOCKED_BY_IT: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockedByReferenceThisTurn]);

card!(
    index = index::GLYPH_OF_REINCARNATION,
    oracle_id = "93d9a1c6-9f6f-487f-a46c-bfd80946ccdf",
    scryfall_id = "a67e8214-a192-4143-9d5e-d0e254e1bf6e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Glyph of Reincarnation",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::destroy_all_no_regen(&BLOCKED_BY_IT),
            Effect::ReturnPerBlockedDeath {
                filter: &Filter::CREATURE
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&WALL))),
        condition = Some(Condition::AfterCombat)
    )],
);
