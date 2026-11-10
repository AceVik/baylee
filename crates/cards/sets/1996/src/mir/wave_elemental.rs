//! Wave Elemental — {2}{U}{U} — Creature — Elemental
//! Oracle: {U}, {T}, Sacrifice this creature: Tap up to three target creatures without flying.
//! Set: MIR #102 — Mirage | Scryfall ID: 111ee762-72cb-43e3-88a1-6f1f0b9ee66e | Oracle ID: d8eed5f7-8724-45c7-b6fc-dcfcc400f09a
// IMPLEMENTED — {U}, {T}, Sacrifice: tap up to three target creatures without flying.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::WAVE_ELEMENTAL,
    oracle_id = "d8eed5f7-8724-45c7-b6fc-dcfcc400f09a",
    scryfall_id = "111ee762-72cb-43e3-88a1-6f1f0b9ee66e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wave Elemental",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{U}", TapSelf, SacrificeSelf),
        &[Effect::TapTarget],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&CREATURE_WITHOUT_FLYING),
            3,
        )),
    )],
);
