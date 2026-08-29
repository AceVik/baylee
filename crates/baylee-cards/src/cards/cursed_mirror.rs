//! Cursed Mirror — {2}{R} — Artifact
//! Oracle: {T}: Add {R}.
//! Oracle: As this artifact enters, you may have it become a copy of any creature on the battlefield until end of turn, except it has haste.
//! Set: SOC #242 — Secrets of Strixhaven Commander | Scryfall ID: 077392b3-6b06-46c8-8737-51e85f690448 | Oracle ID: 4d67e2a7-4aa7-44cc-853b-500d7aac046d
// IMPLEMENTED — {R} mana + until-EOT clone with haste (layer-1 copy
// effect with UntilEndOfTurn duration).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, CopyMod, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(28),
    oracle_id: "4d67e2a7-4aa7-44cc-853b-500d7aac046d",
    scryfall_id: "077392b3-6b06-46c8-8737-51e85f690448",
    faces: &[FaceDef {
        name: "Cursed Mirror",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Red,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::CopyOnEnterUntilEot {
            target: TargetSpec::Object(&ANY_CREATURE),
            mods: &[CopyMod::AddKeyword(KeywordSet::HASTE)],
        },
    ],
};

#[cfg(test)]
mod tests {}
