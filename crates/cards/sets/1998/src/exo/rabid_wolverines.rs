//! Rabid Wolverines — {3}{G}{G} — Creature — Wolverine
//! Oracle: Whenever this creature becomes blocked by a creature, this creature gets +1/+1 until end of turn.
//! Set: EXO #119 — Exodus | Scryfall ID: 99121a2b-c735-47be-b01e-cdf59809e7f3 | Oracle ID: ed076c39-9bfc-424a-8ce8-8572f7c0de73
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RABID_WOLVERINES,
    oracle_id = "ed076c39-9bfc-424a-8ce8-8572f7c0de73",
    scryfall_id = "99121a2b-c735-47be-b01e-cdf59809e7f3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rabid Wolverines",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLVERINE],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&Filter::CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(1, 1),
            duration: Duration::UntilEndOfTurn,
        }]
    ),],
);
