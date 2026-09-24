//! Grasping Shadows // Shadows' Lair — {3}{B} — Enchantment // Land — Cave
//! Oracle: Whenever a creature you control attacks alone, it gains deathtouch and lifelink until end of turn. Put a dread counter on this enchantment. Then if there are three or more dread counters on it, transform it.
//! Oracle: (Transforms from Grasping Shadows.)
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}, Remove a dread counter from this land: You draw a card and you lose 1 life.
//! Set: LCI #108 — The Lost Caverns of Ixalan | Scryfall ID: 81b8b9c9-725d-476d-a3cf-55e3dc3e433d | Oracle ID: 522a4b02-24c7-45d2-9097-2803cc9fffad
//! Face: Grasping Shadows — {3}{B} — Enchantment
//! Face: Shadows' Lair —  — Land — Cave
// PARTIAL — only the back face's {T}: Add {B} is written; the front face's
// one trigger is off, and the back face's dread-counter ability with it.

// NOT SUPPORTED: "Whenever a creature you control attacks alone, it gains
// deathtouch and lifelink until end of turn." — sayable on its own:
// Trigger::Attacks with Condition::ControlCountAtMost(&Filter::ATTACKING_CREATURE,
// 1). It stays off with the two clauses after it, because they are the same
// trigger's effects and neither can be written.
//
// NOT SUPPORTED: "Put a dread counter on this enchantment. Then if there are
// three or more dread counters on it, transform it." — "dread" is a word the
// rules have never heard of, so it would be a CounterKind::Custom id assigned
// in `baylee_cards_dsl::counters`, and no DREAD constant exists there (a card
// writes `counters::DREAD`, never a bare `CounterKind::Custom(…)`, which is
// the collision that module exists to prevent). The branch on three is
// sayable (Effect::IfCondition over Condition::CountersOnSelf); the transform
// it leads to is not (#206).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Shadows' Lair's abilities — the back face of a transforming card, so they
/// belong to that face and not to the enchantment the card is played as.
static BACK_ABILITIES: &[AbilityDef] = &[
    // NOT SUPPORTED: "{B}, {T}, Remove a dread counter from this land: You
    // draw a card and you lose 1 life." — CostPart::RemoveCounterSelf takes a
    // CounterKind, and there is no DREAD id to name, so the ability cannot be
    // written without inventing one.
    mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
];

card!(
    index = index::GRASPING_SHADOWS,
    oracle_id = "522a4b02-24c7-45d2-9097-2803cc9fffad",
    scryfall_id = "81b8b9c9-725d-476d-a3cf-55e3dc3e433d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Grasping Shadows",
            mana_cost = mana!("{3}{B}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Shadows' Lair",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "no counters::DREAD id, and no effect transforms a permanent in place (#206), so the attack trigger that puts the dread counters is off and Shadows' Lair is never reached",
    ),
);
