//! Sokka, Tenacious Tactician — {1}{U}{R} — Legendary Creature — Human Warrior Ally
//! Oracle: Menace, prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
//! Oracle: Other Allies you control have menace and prowess.
//! Oracle: Whenever you cast a noncreature spell, create a 1/1 white Ally creature token.
//! Set: TLA #241 — Avatar: The Last Airbender | Scryfall ID: f0fa5897-1da7-488f-bb19-1632e969c050 | Oracle ID: 6b68acc2-b9d5-495b-8054-c04bae1349f1
// IMPLEMENTED — menace + prowess (engine-level keyword trigger) + Ally
// grants (layer 6) + token on noncreature spells.

use crate::filters::ANOTHER_ALLY;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static NONCREATURE_SPELL: Filter = Filter::And(&[Filter::ControlledByYou, Filter::NONCREATURE]);

use crate::tokens::ALLY_1_1_WHITE as ALLY_TOKEN;

card! {
    index: 149,
    oracle_id: "6b68acc2-b9d5-495b-8054-c04bae1349f1",
    scryfall_id: "f0fa5897-1da7-488f-bb19-1632e969c050",
    faces: &[face! {
        name: "Sokka, Tenacious Tactician",
        mana_cost: baylee_core::mana!("{1}{U}{R}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::WARRIOR, creature::ALLY],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Red]),
    keywords: KeywordSet::MENACE.union(KeywordSet::PROWESS),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: ANOTHER_ALLY,
            modifier: Modifier::AddKeyword(KeywordSet::MENACE.union(KeywordSet::PROWESS)),
            cross_zone: false,
        }),
        triggered!(Trigger::SpellCast(&NONCREATURE_SPELL), &[Effect::CreateTokenN {
                token: &ALLY_TOKEN,
                amount: Amount::Fixed(1),
            }]),
    ],
}
