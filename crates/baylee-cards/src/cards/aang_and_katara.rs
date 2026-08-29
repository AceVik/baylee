//! Aang and Katara — {1}{G}{W}{U} — Legendary Creature — Human Avatar Ally
//! Oracle: Whenever Aang and Katara enter or attack, create X 1/1 white Ally creature tokens, where X is the number of tapped artifacts and/or creatures you control.
//! Set: TLA #192 — Avatar: The Last Airbender | Scryfall ID: f333ea01-124f-4125-87ab-609be40e774c | Oracle ID: 481c3e14-b670-4fab-aa9f-6ce5b514096d
// IMPLEMENTED — ETB/attack Ally token engine.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, TokenDef, Trigger, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static TAPPED_ARTIFACTS_CREATURES: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Tapped,
    Filter::Or(&[
        Filter::HasType(TypeSet::ARTIFACT),
        Filter::HasType(TypeSet::CREATURE),
    ]),
]);

static ALLY_TOKEN: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(0),
    oracle_id: "481c3e14-b670-4fab-aa9f-6ce5b514096d",
    scryfall_id: "f333ea01-124f-4125-87ab-609be40e774c",
    faces: &[FaceDef {
        name: "Aang and Katara",
        mana_cost: baylee_core::mana!("{1}{G}{W}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::AVATAR, creature::ALLY],
        power: Some(2),
        toughness: Some(3),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::CreateTokenN {
                token: &ALLY_TOKEN,
                amount: Amount::CountOf {
                    filter: &TAPPED_ARTIFACTS_CREATURES,
                    zone: ZoneSel::Battlefield,
                },
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::Attacks(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::CreateTokenN {
                token: &ALLY_TOKEN,
                amount: Amount::CountOf {
                    filter: &TAPPED_ARTIFACTS_CREATURES,
                    zone: ZoneSel::Battlefield,
                },
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
