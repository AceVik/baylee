//! Fevered Convulsions — {B}{B} — Enchantment
//! Oracle: {2}{B}{B}: Put a -1/-1 counter on target creature.
//! Set: TMP #136 — Tempest | Scryfall ID: 3a790769-e76e-49e9-9d6d-05ce8e858243 | Oracle ID: 83f35293-39e9-4c78-9c9f-25a1fc905676
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FEVERED_CONVULSIONS,
    oracle_id = "83f35293-39e9-4c78-9c9f-25a1fc905676",
    scryfall_id = "3a790769-e76e-49e9-9d6d-05ce8e858243",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fevered Convulsions",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}{B}{B}"),
        &[Effect::AddCounter {
            kind: CounterKind::M1M1,
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
