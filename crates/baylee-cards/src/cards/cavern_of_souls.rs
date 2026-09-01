//! Cavern of Souls — (no cost) — Land
//! Oracle: As this land enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type, and that spell can't be countered.
//! Set: LCI #269 — The Lost Caverns of Ixalan | Scryfall ID: 3aad15a2-8a1b-4460-9b06-e85863081878 | Oracle ID: 89ca686a-7c72-4d8f-9290-e89635624a83
// IMPLEMENTED — choose-a-type, {C}, and the restricted any-color mana:
// it pays only for creature spells of the chosen type and makes them
// uncounterable (mana provenance + Uncounterable rider).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    ALL_MANA_COLORS, AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule,
    Cost, Coverage, Effect, EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static CHOSEN_TYPE_CREATURE_SPELL: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::MatchesChosenTypeOfSource,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(17),
    oracle_id: "89ca686a-7c72-4d8f-9290-e89635624a83",
    scryfall_id: "3aad15a2-8a1b-4460-9b06-e85863081878",
    faces: &[FaceDef {
        name: "Cavern of Souls",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::ChooseSubtype],
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Colorless,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddManaRestricted {
                colors: ALL_MANA_COLORS,
                amount: 1,
                filter: &CHOSEN_TYPE_CREATURE_SPELL,
                rider: baylee_cards_dsl::SpendRider::Uncounterable,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
