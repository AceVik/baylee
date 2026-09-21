//! Tolaria — (no cost) — Legendary Land
//! Oracle: {T}: Add {U}.
//! Oracle: {T}: Target creature loses banding and all "bands with other" abilities until end of turn. Activate only during any upkeep step.
//! Set: LEG #308 — Legends | Scryfall ID: d43c01b7-443d-4061-a934-6863d230c9b8 | Oracle ID: 9879a4f3-3b9c-45cf-af03-7f2ae4c689b4
// PARTIAL — {T}: Add {U} is built; banding is not an enforced keyword and has no removal modifier.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOLARIA,
    oracle_id = "9879a4f3-3b9c-45cf-af03-7f2ae4c689b4",
    scryfall_id = "d43c01b7-443d-4061-a934-6863d230c9b8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Tolaria",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the second printed ability — removing banding and \"bands with other\" — is not expressible: banding carries no enforced keyword bit and no Modifier targets it"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        // NOT SUPPORTED: "{T}: Target creature loses banding and all \"bands with other\" abilities until end of turn. Activate only during any upkeep step."
    ],
);
