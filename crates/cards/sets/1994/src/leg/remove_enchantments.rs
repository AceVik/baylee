//! Remove Enchantments — {W} — Instant
//! Oracle: Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control. Then destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control.
//! Set: LEG #33 — Legends | Scryfall ID: bf2e3a8a-b386-474d-b8e9-4c2d56a2b742 | Oracle ID: fa879d99-3d2b-4a9a-a17f-d5ac109f8f44
// IMPLEMENTED — returns your own enchantments and Auras on your permanents or on attacking opposing creatures, then destroys the other ones.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const AURA: Filter = Filter::HasSubtype(subtypes::enchantment::AURA);

/// "permanents you control"
const ON_YOUR_PERMANENT: Filter = Filter::AttachedTo(&Filter::ControlledByYou);

/// "attacking creatures your opponents control"
const ON_AN_OPPOSING_ATTACKER: Filter = Filter::AttachedTo(&Filter::And(&[
    Filter::ATTACKING_CREATURE,
    Filter::ControlledByOpponent,
]));

/// First sentence: "all enchantments you both own and control, all Auras you
/// own attached to permanents you control, and all Auras you own attached to
/// attacking creatures your opponents control."
const RETURNED: Filter = Filter::Or(&[
    Filter::And(&[
        Filter::ENCHANTMENT,
        Filter::OwnedByYou,
        Filter::ControlledByYou,
    ]),
    Filter::And(&[AURA, Filter::OwnedByYou, ON_YOUR_PERMANENT]),
    Filter::And(&[AURA, Filter::OwnedByYou, ON_AN_OPPOSING_ATTACKER]),
]);

/// Second sentence: "all other enchantments you control, all other Auras
/// attached to permanents you control, and all other Auras attached to
/// attacking creatures your opponents control." "Other" is whatever the first
/// sentence did not name.
const DESTROYED: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::And(&[Filter::ENCHANTMENT, Filter::ControlledByYou]),
        Filter::And(&[AURA, ON_YOUR_PERMANENT]),
        Filter::And(&[AURA, ON_AN_OPPOSING_ATTACKER]),
    ]),
    Filter::Not(&RETURNED),
]);

card!(
    index = index::REMOVE_ENCHANTMENTS,
    oracle_id = "fa879d99-3d2b-4a9a-a17f-d5ac109f8f44",
    scryfall_id = "bf2e3a8a-b386-474d-b8e9-4c2d56a2b742",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Remove Enchantments",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[
        Effect::ReturnAllToHand {
            filter: &RETURNED,
            opponents_only: false,
        },
        Effect::destroy_all(&DESTROYED),
    ])],
);
