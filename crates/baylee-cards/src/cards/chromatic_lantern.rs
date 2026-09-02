//! Chromatic Lantern — {3} — Artifact
//! Oracle: Lands you control have "{T}: Add one mana of any color."
//! Oracle: {T}: Add one mana of any color.
//! Set: MBC #73 — Mystery Booster Commander Edition | Scryfall ID: 9b29492a-8bdd-4806-8d1b-3058ed277cc1 | Oracle ID: 539f5396-d99a-417d-a84c-dff7930b5900
// IMPLEMENTED — its own any-color mana + the lands-you-control grant
// (GrantActivated static).

use baylee_cards_dsl::prelude::*;

card! {
    index: 19,
    oracle_id: "539f5396-d99a-417d-a84c-dff7930b5900",
    scryfall_id: "9b29492a-8bdd-4806-8d1b-3058ed277cc1",
    faces: &[face! {
        name: "Chromatic Lantern",
        mana_cost: baylee_core::mana!("{3}"),
        types: TypeSet::ARTIFACT,
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::And(&[Filter::LAND, Filter::ControlledByYou]),
            modifier: Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: ANY_COLOR_MANA,
                mana_ability: true,
            },
            cross_zone: false,
        }),
        mana_ability!(&[Effect::mana_choice(ALL_MANA_COLORS)]),
    ],
}
