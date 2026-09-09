//! Thought Vessel — {2} — Artifact
//! Oracle: You have no maximum hand size.
//! Oracle: {T}: Add {C}.
//! Set: MBC #78 — Mystery Booster Commander Edition | Scryfall ID: ad077996-6b5e-4eb8-bb6e-93d43c5efa8f | Oracle ID: 9965d9c5-2ebf-4a6c-930e-55c5890979be
// IMPLEMENTED — Reliquary Tower's static on an artifact. The modifier is
// read per *player* (the cleanup step asks whether any effect its controller
// owns says so), which is why the filter is `Any` rather than `This`.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1358,
    oracle_id: "9965d9c5-2ebf-4a6c-930e-55c5890979be",
    scryfall_id: "ad077996-6b5e-4eb8-bb6e-93d43c5efa8f",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Thought Vessel",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
    },
    ],
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::NoMaxHandSize,
            cross_zone: false,
        }),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
}
