//! Orcish Settlers — {1}{R} — Creature — Orc
//! Oracle: {X}{X}{R}, {T}, Sacrifice this creature: Destroy X target lands.
//! Set: WTH #112 — Weatherlight | Scryfall ID: d54764f6-6f65-405c-ba30-1e485ce3fe21 | Oracle ID: 8b8d4ae0-0d18-4f06-a883-0edff494d06b
// IMPLEMENTED — Pays {X}{X}{R}, taps, and sacrifices itself to destroy X target lands.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORCISH_SETTLERS,
    oracle_id = "8b8d4ae0-0d18-4f06-a883-0edff494d06b",
    scryfall_id = "d54764f6-6f65-405c-ba30-1e485ce3fe21",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orcish Settlers",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{X}{X}{R}", TapSelf, SacrificeSelf),
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::LAND),
            no_regen: false,
        }],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::LAND))),
    ),],
);
