//! Malevolent Hermit // Benevolent Geist — {1}{U} — Creature — Human Wizard // Creature — Spirit Wizard
//! Oracle: {U}, Sacrifice this creature: Counter target noncreature spell unless its controller pays {3}.
//! Oracle: Disturb {2}{U} (You may cast this card from your graveyard transformed for its disturb cost.)
//! Oracle: Flying
//! Oracle: Noncreature spells you control can't be countered.
//! Oracle: If Benevolent Geist would be put into a graveyard from anywhere, exile it instead.
//! Set: MID #61 — Innistrad: Midnight Hunt | Scryfall ID: e79269af-63eb-43d2-afee-c38fa14a0c5b | Oracle ID: 51233ade-70cd-4539-9f41-5ffab761da54
//! Face: Malevolent Hermit — {1}{U} — Creature — Human Wizard
//! Face: Benevolent Geist —  — Creature — Spirit Wizard
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MALEVOLENT_HERMIT,
    oracle_id = "51233ade-70cd-4539-9f41-5ffab761da54",
    scryfall_id = "e79269af-63eb-43d2-afee-c38fa14a0c5b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Malevolent Hermit",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
            power = Some(2),
            toughness = Some(1),
        ),
        face!(
            name = "Benevolent Geist",
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::SPIRIT, subtypes::creature::WIZARD],
            power = Some(2),
            toughness = Some(2),
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
