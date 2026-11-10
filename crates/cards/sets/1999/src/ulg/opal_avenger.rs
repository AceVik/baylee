//! Opal Avenger — {2}{W} — Enchantment
//! Oracle: When you have 10 or less life, if this permanent is an enchantment, it becomes a 3/5 Soldier creature.
//! Set: ULG #15 — Urza's Legacy | Scryfall ID: f9337bbe-e092-469d-8122-77f92e233306 | Oracle ID: 13a85c9f-f653-4d90-bea6-94022ee82527
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OPAL_AVENGER,
    oracle_id = "13a85c9f-f653-4d90-bea6-94022ee82527",
    scryfall_id = "f9337bbe-e092-469d-8122-77f92e233306",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Opal Avenger",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::State(&Condition::LifeAtMost(10)),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::SOLDIER,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(3, 5), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
