//! Benevolent Unicorn — {1}{W} — Creature — Unicorn
//! Oracle: If a spell would deal damage to a permanent or player, it deals that much damage minus 1 to that permanent or player instead.
//! Set: MIR #4 — Mirage | Scryfall ID: 2a243bd7-af98-4e44-af6e-3b0b71d4837b | Oracle ID: b437c963-def5-40d6-b567-ae9f1d2a0fa5
// PARTIAL — a vanilla 1/2: the damage-reducing replacement effect is not played.
// NOT SUPPORTED: "If a spell would deal damage to a permanent or player, it deals that much damage minus 1 to that permanent or player instead." The engine can prevent and redirect damage but no static ability reduces damage dealt by spells by 1.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BENEVOLENT_UNICORN,
    oracle_id = "b437c963-def5-40d6-b567-ae9f1d2a0fa5",
    scryfall_id = "2a243bd7-af98-4e44-af6e-3b0b71d4837b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "The replacement effect (a spell deals 1 less damage to a permanent or player) is not played: no static ability reduces damage dealt by spells; Benevolent Unicorn is a vanilla 1/2.",
    ),
    faces = &[face!(
        name = "Benevolent Unicorn",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(1),
        toughness = Some(2),
    ),],
);
