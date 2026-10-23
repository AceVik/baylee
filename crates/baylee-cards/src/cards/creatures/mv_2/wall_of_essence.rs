//! Wall of Essence — {1}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: Whenever this creature is dealt combat damage, you gain that much life.
//! Set: TPR #38 — Tempest Remastered | Scryfall ID: 94b05805-e2af-4048-b7c7-1dbd188ba0f6 | Oracle ID: 1374ac18-01bd-452f-af72-a5f96285ed69
// PARTIAL: restricting the trigger to combat damage and reading the exact amount of damage dealt to a permanent in Trigger::DealtDamage are not supported; has defender and gains 1 life whenever dealt damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_ESSENCE,
    oracle_id = "1374ac18-01bd-452f-af72-a5f96285ed69",
    scryfall_id = "94b05805-e2af-4048-b7c7-1dbd188ba0f6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "restricting the trigger to combat damage and reading the exact amount of damage dealt to a permanent in Trigger::DealtDamage are not supported; has defender and gains 1 life whenever dealt damage",
    ),
    keywords = KeywordSet::DEFENDER,
    faces = &[face!(
        name = "Wall of Essence",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::This),
        &[Effect::gain_life(1)],
    ),],
);
