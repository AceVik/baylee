//! Infernal Medusa — {3}{B}{B} — Creature — Gorgon
//! Oracle: Whenever this creature blocks a creature, destroy that creature at end of combat.
//! Oracle: Whenever this creature becomes blocked by a non-Wall creature, destroy that creature at end of combat.
//! Set: LEG #108 — Legends | Scryfall ID: 26a5333f-2761-42b8-ae8b-1d360b109daf | Oracle ID: 2cf5ce1f-d5f6-44cd-96e5-87d990d7e770
// IMPLEMENTED — one block trigger per printed sentence: the pair's other
// creature is the event object, and its combat role picks the direction —
// `ATTACKING_CREATURE` for the creature this one blocks, a blocking non-Wall
// for the creature that blocks it, each destroyed at end of combat.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The blocker of the pair, when it is not a Wall: "a non-Wall creature".
static NON_WALL_BLOCKER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Blocking,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::INFERNAL_MEDUSA,
    oracle_id = "2cf5ce1f-d5f6-44cd-96e5-87d990d7e770",
    scryfall_id = "26a5333f-2761-42b8-ae8b-1d360b109daf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Infernal Medusa",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GORGON],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[
        // "Whenever this creature blocks a creature": the other creature of
        // the pair is the attacker this one blocked.
        triggered!(
            Trigger::BlocksOrBecomesBlockedBy(&Filter::ATTACKING_CREATURE),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)]
            }]
        ),
        // "Whenever this creature becomes blocked by a non-Wall creature":
        // the other creature is the blocker, which is what `Filter::Blocking`
        // picks out, while the attacker it blocked is never blocking.
        triggered!(
            Trigger::BlocksOrBecomesBlockedBy(&NON_WALL_BLOCKER),
            &[Effect::AtEndOfCombat {
                about: TargetSpec::EventObject,
                effects: &[Effect::destroy(TargetSpec::EventObject)]
            }]
        ),
    ],
);
