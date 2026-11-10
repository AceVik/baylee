//! Mold Demon — {5}{B}{B} — Creature — Fungus Demon
//! Oracle: When this creature enters, sacrifice it unless you sacrifice two Swamps.
//! Set: LEG #112 — Legends | Scryfall ID: 649a33aa-7eac-4161-ae1a-fcbc758abccf | Oracle ID: 7ce9513e-50f3-4d36-889b-b180a8a16252

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOLD_DEMON,
    oracle_id = "7ce9513e-50f3-4d36-889b-b180a8a16252",
    scryfall_id = "649a33aa-7eac-4161-ae1a-fcbc758abccf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mold Demon",
        mana_cost = mana!("{5}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FUNGUS, subtypes::creature::DEMON],
        power = Some(6),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PlayerMaySacrificeNOr {
            player: PlayerRel::You,
            filter: &Filter::HasSubtype(subtypes::land::SWAMP),
            count: 2,
            effect: &Effect::SacrificeSelf,
        }]
    )],
);
