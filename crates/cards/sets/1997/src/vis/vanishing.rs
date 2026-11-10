//! Vanishing — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {U}{U}: Enchanted creature phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: VIS #48 — Visions | Scryfall ID: 8d1fb805-1382-458c-b98d-4491f13833b6 | Oracle ID: cd2791e8-0b61-49b8-8b72-ff2737df1049
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::VANISHING,
    oracle_id = "cd2791e8-0b61-49b8-8b72-ff2737df1049",
    scryfall_id = "8d1fb805-1382-458c-b98d-4491f13833b6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Vanishing",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        activated!(
            cost!("{U}{U}"),
            &[Effect::PhaseOutFilter(&ENCHANTED_CREATURE)],
        ),
    ],
);
