//! Auriok Steelshaper — {1}{W} — Creature — Human Soldier
//! Oracle: Equip costs you pay cost {1} less.
//! Oracle: As long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1.
//! Set: MRD #4 — Mirrodin | Scryfall ID: 4dd09c01-5a77-4992-96ae-c395a5966a92 | Oracle ID: e3ee8b0e-9bdc-4b9a-b4f9-59189dd6a80d
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_SOLDIER_OR_KNIGHT: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasType(TypeSet::CREATURE),
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::SOLDIER),
        Filter::HasSubtype(subtypes::creature::KNIGHT),
    ]),
]);

card!(
    index = index::AURIOK_STEELSHAPER,
    oracle_id = "e3ee8b0e-9bdc-4b9a-b4f9-59189dd6a80d",
    scryfall_id = "4dd09c01-5a77-4992-96ae-c395a5966a92",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Auriok Steelshaper",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::YOUR_ARTIFACT, Modifier::EquipCostsLess(1)),
        static_ability!(
            YOUR_SOLDIER_OR_KNIGHT,
            Modifier::ModifyPT(1, 1),
            condition = Some(Condition::SourceMatches(&Filter::Equipped)),
        ),
    ],
);
