//! Restoration Angel — {3}{W} — Creature — Angel
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When this creature enters, you may exile target non-Angel creature you control, then return that card to the battlefield under your control.
//! Set: INR #38 — Innistrad Remastered | Scryfall ID: f17f85d3-58e5-4128-90c5-98b524256af8 | Oracle ID: dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa
// IMPLEMENTED — flash flying + immediate blink of a non-Angel creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static NON_ANGEL_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(creature::ANGEL)),
    Filter::ControlledByYou,
]);

card! {
    index: 131,
    oracle_id: "dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa",
    scryfall_id: "f17f85d3-58e5-4128-90c5-98b524256af8",
    faces: &[face! {
        name: "Restoration Angel",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ANGEL],
        power: Some(3),
        toughness: Some(4),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::Blink {
            target: TargetSpec::Object(&NON_ANGEL_CREATURE_YOU_CONTROL),
        }], targets: Some(TargetReq {
            spec: TargetSpec::Object(&NON_ANGEL_CREATURE_YOU_CONTROL),
            min: 0,
            max: 1,
            count_is_x: false,
        }))],
}
