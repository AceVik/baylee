//! Glasspool Mimic // Glasspool Shore — {2}{U} — Creature — Shapeshifter Rogue // Land
//! Oracle: You may have Glasspool Mimic enter the battlefield as a copy of any creature on the battlefield, except it's a Shapeshifter Rogue in addition to its other types. // {T}: Add {U}.
//! Set: ZNR #60 — Zendikar Rising | Scryfall ID: 5adcb500-8c77-4925-8e2c-1243502827d1 | Oracle ID: c178953c-3888-4edd-9d0c-265bd82b1d24
// IMPLEMENTED — clone-with-extra-subtypes front (CopyOnEnter) + MDFC
// land back playable via the face-choice land play.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static SHORE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card! {
    index: 60,
    oracle_id: "c178953c-3888-4edd-9d0c-265bd82b1d24",
    scryfall_id: "5adcb500-8c77-4925-8e2c-1243502827d1",
    faces: &[
        face! {
            name: "Glasspool Mimic",
            mana_cost: baylee_core::mana!("{2}{U}"),
            types: TypeSet::CREATURE,
            subtypes: &[creature::SHAPESHIFTER, creature::ROGUE],
            power: Some(0),
            toughness: Some(0),
        },
        face! {
            name: "Glasspool Shore",
            types: TypeSet::LAND,
            abilities: SHORE_MANA,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[
            CopyMod::AddSubtype(creature::SHAPESHIFTER),
            CopyMod::AddSubtype(creature::ROGUE),
        ],
    }],
}
