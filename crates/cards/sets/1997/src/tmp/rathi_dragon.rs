//! Rathi Dragon — {2}{R}{R} — Creature — Dragon
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: When this creature enters, sacrifice it unless you sacrifice two Mountains.
//! Set: TPR #150 — Tempest Remastered | Scryfall ID: 9ec3d43c-2b3e-4b79-a61d-03efd6a689d4 | Oracle ID: 6a180f07-7afa-4605-9c58-4521a5f3fcc1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MOUNTAIN: Filter = Filter::HasSubtype(subtypes::land::MOUNTAIN);

card!(
    index = index::RATHI_DRAGON,
    oracle_id = "6a180f07-7afa-4605-9c58-4521a5f3fcc1",
    scryfall_id = "9ec3d43c-2b3e-4b79-a61d-03efd6a689d4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rathi Dragon",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(5),
        toughness = Some(5),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PlayerMaySacrificeNOr {
            player: PlayerRel::You,
            filter: &MOUNTAIN,
            count: 2,
            effect: &Effect::SacrificeSelf,
        }],
    )],
);
