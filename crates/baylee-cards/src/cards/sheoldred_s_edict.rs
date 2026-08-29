//! Sheoldred's Edict — {1}{B} — Instant
//! Oracle: Choose one —
//! Oracle: • Each opponent sacrifices a nontoken creature of their choice.
//! Oracle: • Each opponent sacrifices a creature token of their choice.
//! Oracle: • Each opponent sacrifices a planeswalker of their choice.
//! Set: ONE #108 — Phyrexia: All Will Be One | Scryfall ID: a9225cc3-90f0-448f-a8d9-7c6c2796d077 | Oracle ID: 217062f5-96f1-454c-9507-17f34ef37070
// IMPLEMENTED — all three edict modes (per-opponent sacrifice choice).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    PlayerRel, SpellMode,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONTOKEN_CREATURE: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::Not(&Filter::IsToken),
]);
static CREATURE_TOKEN: Filter = Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::IsToken]);
static WALKER: Filter = Filter::HasType(TypeSet::PLANESWALKER);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(144),
    oracle_id: "217062f5-96f1-454c-9507-17f34ef37070",
    scryfall_id: "a9225cc3-90f0-448f-a8d9-7c6c2796d077",
    faces: &[FaceDef {
        name: "Sheoldred's Edict",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::INSTANT,
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
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            SpellMode {
                effects: &[Effect::SacrificeFilter {
                    who: PlayerRel::EachOpponent,
                    filter: &NONTOKEN_CREATURE,
                }],
                target: None,
                cost_override: None,
            },
            SpellMode {
                effects: &[Effect::SacrificeFilter {
                    who: PlayerRel::EachOpponent,
                    filter: &CREATURE_TOKEN,
                }],
                target: None,
                cost_override: None,
            },
            SpellMode {
                effects: &[Effect::SacrificeFilter {
                    who: PlayerRel::EachOpponent,
                    filter: &WALKER,
                }],
                target: None,
                cost_override: None,
            },
        ],
    }],
};

#[cfg(test)]
mod tests {}
