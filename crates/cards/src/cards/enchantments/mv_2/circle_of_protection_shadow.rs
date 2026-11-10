//! Circle of Protection: Shadow — {1}{W} — Enchantment
//! Oracle: {1}: The next time a creature of your choice with shadow would deal damage to you this turn, prevent that damage.
//! Set: TMP #12 — Tempest | Scryfall ID: 49f29a3b-7136-496c-bc29-8808bfff0f82 | Oracle ID: 07c42f46-800e-4d2a-b844-c125ab93190d
// IMPLEMENTED — Pays {1} to prevent the next damage a chosen creature with shadow would deal to you this turn.

use baylee_cards_dsl::prelude::*;

const CREATURE_WITH_SHADOW: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SHADOW)]);

card!(
    index = index::CIRCLE_OF_PROTECTION_SHADOW,
    oracle_id = "07c42f46-800e-4d2a-b844-c125ab93190d",
    scryfall_id = "49f29a3b-7136-496c-bc29-8808bfff0f82",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Circle of Protection: Shadow",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{1}"),
        &[Effect::PreventNextFromChosenSource {
            sources: &CREATURE_WITH_SHADOW,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }]
    )],
);
