//! Courser of Kruphix — {1}{G}{G} — Enchantment Creature — Centaur
//! Oracle: Play with the top card of your library revealed.
//! Oracle: You may play lands from the top of your library.
//! Oracle: Landfall — Whenever a land you control enters, you gain 1 life.
//! Set: CMM #888 — Commander Masters | Scryfall ID: dc63d2ea-a980-466e-9ebb-f28008f84c3d | Oracle ID: 46779609-4fa7-4fd2-b5b4-7d4d749339e6
// IMPLEMENTED — public library top, ordinary land-play permission, and landfall.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COURSER_OF_KRUPHIX,
    oracle_id = "46779609-4fa7-4fd2-b5b4-7d4d749339e6",
    scryfall_id = "dc63d2ea-a980-466e-9ebb-f28008f84c3d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Courser of Kruphix",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CENTAUR],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[
        static_ability!(Filter::Any, Modifier::RevealLibraryTop),
        static_ability!(Filter::Any, Modifier::PlayLandsFromLibraryTop),
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::gain_life(1)]
        ),
    ],
);
