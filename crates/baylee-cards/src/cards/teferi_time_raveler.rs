//! Teferi, Time Raveler — {1}{W}{U} — Legendary Planeswalker — Teferi
//! Oracle: Each opponent can cast spells only any time they could cast a sorcery.
//! Oracle: +1: Until your next turn, you may cast sorcery spells as though they had flash.
//! Oracle: −3: Return up to one target artifact, creature, or enchantment to its owner's hand. Draw a card.
//! Set: RVR #232 — Ravnica Remastered | Scryfall ID: 662fe50f-d75c-422c-8c6c-1f9b5c4ba21f | Oracle ID: ae7604bb-4818-45a3-960c-cf3d83f15964
// PARTIAL — timing lock + −3 implemented; +1 needs UntilYourNextTurn (M2+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    Layer, Modifier, PartnerKind, StaticAbility, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, planeswalker};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static BOUNCE_TARGET: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasType(TypeSet::CREATURE),
    Filter::HasType(TypeSet::ENCHANTMENT),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(166),
    oracle_id: "ae7604bb-4818-45a3-960c-cf3d83f15964",
    scryfall_id: "662fe50f-d75c-422c-8c6c-1f9b5c4ba21f",
    faces: &[FaceDef {
        name: "Teferi, Time Raveler",
        mana_cost: baylee_core::mana!("{1}{W}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::TEFERI],
        power: None,
        toughness: None,
        loyalty: Some(4),
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
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("+1 sorcery-flash until your next turn (UntilYourNextTurn, M2+)"),
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::OpponentsCastAsSorcery,
            cross_zone: false,
        }),
        AbilityDef::Loyalty {
            cost: -3,
            effects: &[
                Effect::ReturnToHand {
                    target: TargetSpec::Object(&BOUNCE_TARGET),
                },
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
            ],
            target: Some(TargetSpec::Object(&BOUNCE_TARGET)),
        },
    ],
};

#[cfg(test)]
mod tests {}
