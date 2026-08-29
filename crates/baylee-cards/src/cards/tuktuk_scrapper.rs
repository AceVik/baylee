//! Tuktuk Scrapper — {3}{R} — Creature — Goblin Artificer Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may destroy target artifact. If that artifact is put into a graveyard this way, this creature deals damage to that artifact's controller equal to the number of Allies you control.
//! Set: WWK #94 — Worldwake | Scryfall ID: d3a84a2a-6384-497a-8ee2-de0fa74fcc80 | Oracle ID: 85cf2403-b419-4364-8ac9-67dd1ceddf9e
// IMPLEMENTED — rally artifact destruction + damage per Ally (damage hits
// the destroyed artifact's controller).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, TargetReq, TargetSpec, Trigger, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLY_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[Filter::This, Filter::HasSubtype(creature::ALLY)]),
]);
static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);
static ARTIFACT: Filter = Filter::HasType(TypeSet::ARTIFACT);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(174),
    oracle_id: "85cf2403-b419-4364-8ac9-67dd1ceddf9e",
    scryfall_id: "d3a84a2a-6384-497a-8ee2-de0fa74fcc80",
    faces: &[FaceDef {
        name: "Tuktuk Scrapper",
        mana_cost: baylee_core::mana!("{3}{R}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::GOBLIN, creature::ARTIFICER, creature::ALLY],
        power: Some(2),
        toughness: Some(2),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&ALLY_ETB),
        once_per_turn: false,
        effects: &[
            Effect::Destroy {
                target: TargetSpec::Object(&ARTIFACT),
            },
            Effect::DealDamageToTargetController {
                amount: Amount::CountOf {
                    filter: &ALLIES_YOU,
                    zone: ZoneSel::Battlefield,
                },
            },
        ],
        targets: Some(TargetReq::up_to_one(TargetSpec::Object(&ARTIFACT))),
    }],
};

#[cfg(test)]
mod tests {}
