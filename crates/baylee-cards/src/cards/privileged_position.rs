//! Privileged Position — {2}{G/W}{G/W}{G/W} — Enchantment
//! Oracle: ({G/W} can be paid with either {G} or {W}.)
//! Oracle: Other permanents you control have hexproof. (They can't be the targets of spells or abilities your opponents control.)
//! Set: 2X2 #263 — Double Masters 2022 | Scryfall ID: 9655bbe4-062f-4278-ad05-a326a64c5b69 | Oracle ID: abd62af0-c17d-4f62-af15-9ea83037b990
// IMPLEMENTED — hexproof grant to your other permanents (layer 6).

static OTHER_YOURS: Filter = Filter::And(&[Filter::ControlledByYou, Filter::Another]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 119,
    oracle_id: "abd62af0-c17d-4f62-af15-9ea83037b990",
    scryfall_id: "9655bbe4-062f-4278-ad05-a326a64c5b69",
    faces: &[face! {
        name: "Privileged Position",
        mana_cost: baylee_core::mana!("{2}{G/W}{G/W}{G/W}"),
        types: TypeSet::ENCHANTMENT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Static(StaticAbility {
        layer: Layer::Ability,
        filter: OTHER_YOURS,
        modifier: Modifier::AddKeyword(KeywordSet::HEXPROOF),
        cross_zone: false,
    })],
}
