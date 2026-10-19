//! Ball Lightning — {R}{R}{R} — Creature — Elemental
//! Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)
//! Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.)
//! Oracle: At the beginning of the end step, sacrifice this creature.
//! Set: FDN #618 — Foundations | Scryfall ID: 5f27dbf0-6818-40ea-832d-10686b4c2900 | Oracle ID: 7485cf25-eb41-4397-be13-7f0b8c10c70a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BALL_LIGHTNING,
    oracle_id = "7485cf25-eb41-4397-be13-7f0b8c10c70a",
    scryfall_id = "5f27dbf0-6818-40ea-832d-10686b4c2900",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ball Lightning",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(6),
        toughness = Some(1),
        keywords = KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
        abilities = &[triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeSelf],
        )],
    ),],
);
