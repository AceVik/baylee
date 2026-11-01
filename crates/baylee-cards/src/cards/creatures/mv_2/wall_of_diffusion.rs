//! Wall of Diffusion — {1}{R} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: This creature can block creatures with shadow as though it had shadow.
//! Set: TPR #164 — Tempest Remastered | Scryfall ID: 4c8cb08b-e820-4054-b5c3-7b737c97c9a5 | Oracle ID: ae42f803-f4ac-45f3-9ce7-227a59096f19
// PARTIAL — Defender; blocking creatures with shadow as though it had shadow is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALL_OF_DIFFUSION,
    oracle_id = "ae42f803-f4ac-45f3-9ce7-227a59096f19",
    scryfall_id = "4c8cb08b-e820-4054-b5c3-7b737c97c9a5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "blocking creatures with shadow as though it had shadow is not in the DSL; enters as a 0/5 Wall with defender"
    ),
    faces = &[face!(
        name = "Wall of Diffusion",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        keywords = KeywordSet::DEFENDER,
        power = Some(0),
        toughness = Some(5),
    ),],
);
