//! Nomads en-Kor — {W} — Creature — Kor Nomad Soldier
//! Oracle: {0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.
//! Set: TPR #20 — Tempest Remastered | Scryfall ID: 48ed076a-909f-4ef0-b85e-a1935b1c2827 | Oracle ID: b8d395a3-0bfe-45c3-bb3a-820d4f235b88
// PARTIAL: {0} activated ability redirecting the next 1 damage dealt to this creature to target creature you control is not supported; plays as a 1/1 Kor Nomad Soldier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOMADS_EN_KOR,
    oracle_id = "b8d395a3-0bfe-45c3-bb3a-820d4f235b88",
    scryfall_id = "48ed076a-909f-4ef0-b85e-a1935b1c2827",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{0} activated ability redirecting the next 1 damage dealt to this creature to target creature you control is not supported; plays as a 1/1 Kor Nomad Soldier",
    ),
    faces = &[face!(
        name = "Nomads en-Kor",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::KOR,
            subtypes::creature::NOMAD,
            subtypes::creature::SOLDIER
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);
