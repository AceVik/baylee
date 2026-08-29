//! Volrath's Stronghold — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}: Put target creature card from your graveyard on top of your library.
//! Set: PD3 #352 — Premium Deck Series: Graveborn | Scryfall ID: f465ae5f-61f0-42c4-978f-841ba1226f56 | Oracle ID: 73b8cf90-3c71-4f8b-a29f-61894b7f27c9
// IMPLEMENTED.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_CARD: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(186),
    oracle_id: "73b8cf90-3c71-4f8b-a29f-61894b7f27c9",
    scryfall_id: "f465ae5f-61f0-42c4-978f-841ba1226f56",
    faces: &[FaceDef {
        name: "Volrath's Stronghold",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
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
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
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
            effects: &[Effect::GraveyardToTop {
                target: TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You),
            }],
            target: Some(TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
