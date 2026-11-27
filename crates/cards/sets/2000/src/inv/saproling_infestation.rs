//! Saproling Infestation — {1}{G} — Enchantment
//! Oracle: Whenever a player kicks a spell, you create a 1/1 green Saproling creature token.
//! Set: INV #208 — Invasion | Scryfall ID: 8642e530-914c-4149-944a-c4966ee27299 | Oracle ID: dd257eb3-4112-41e5-9c7d-00d1f683dbdb
// IMPLEMENTED — creates a 1/1 green Saproling whenever any player casts a kicked spell.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAPROLING_INFESTATION,
    oracle_id = "dd257eb3-4112-41e5-9c7d-00d1f683dbdb",
    scryfall_id = "8642e530-914c-4149-944a-c4966ee27299",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Saproling Infestation",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::Kicked),
        &[Effect::CreateToken {
            token: &generated_tokens::SAPROLING_1_1_GREEN,
        }]
    ),],
);
