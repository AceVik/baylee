//! Warrior en-Kor — {W}{W} — Creature — Kor Warrior Knight
//! Oracle: {0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.
//! Set: TPR #39 — Tempest Remastered | Scryfall ID: afcf1e51-5e9a-4c86-8a14-97d9e919c320 | Oracle ID: 1245b165-e26f-4839-8468-2e95148fc6f6
// PARTIAL: {0} activated ability redirecting the next 1 damage dealt to this creature to target creature you control is not supported; plays as a 2/2 Kor Warrior Knight.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARRIOR_EN_KOR,
    oracle_id = "1245b165-e26f-4839-8468-2e95148fc6f6",
    scryfall_id = "afcf1e51-5e9a-4c86-8a14-97d9e919c320",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{0} activated ability redirecting the next 1 damage dealt to this creature to target creature you control is not supported; plays as a 2/2 Kor Warrior Knight",
    ),
    faces = &[face!(
        name = "Warrior en-Kor",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::KOR,
            subtypes::creature::WARRIOR,
            subtypes::creature::KNIGHT
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
);
