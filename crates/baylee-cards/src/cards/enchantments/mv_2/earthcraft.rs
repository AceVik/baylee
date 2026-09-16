//! Earthcraft — {1}{G} — Enchantment
//! Oracle: Tap an untapped creature you control: Untap target basic land.
//! Set: TMP #222 — Tempest | Scryfall ID: 9dda7531-82a1-4f49-8858-601ddbc6e2bc | Oracle ID: 50aa7aff-1f01-4224-9a83-01f74d703ec2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARTHCRAFT,
    oracle_id = "50aa7aff-1f01-4224-9a83-01f74d703ec2",
    scryfall_id = "9dda7531-82a1-4f49-8858-601ddbc6e2bc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Earthcraft",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
