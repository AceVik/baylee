//! Great Divide Guide — {1}{G} — Creature — Human Scout Ally
//! Oracle: Each land and Ally you control has "{T}: Add one mana of any color."
//! Set: TLA #181 — Avatar: The Last Airbender | Scryfall ID: cc3063ec-5ea6-46c1-8331-c740cbaf6c76 | Oracle ID: 79e69a91-d580-47fb-be76-1e32c50d2fa0
// IMPLEMENTED — the land/Ally any-color mana grant (GrantActivated
// static, same machinery as Chromatic Lantern).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_COLOR: &[ManaColor] = &[
    ManaColor::White,
    ManaColor::Blue,
    ManaColor::Black,
    ManaColor::Red,
    ManaColor::Green,
];
static ANY_COLOR_MANA: &[Effect] = &[Effect::AddManaChoice {
    colors: ANY_COLOR,
    amount: Amount::Fixed(1),
    combination: false,
}];
pub static CARD: CardDef = CardDef {
    index: CardIndex::new(62),
    oracle_id: "79e69a91-d580-47fb-be76-1e32c50d2fa0",
    scryfall_id: "cc3063ec-5ea6-46c1-8331-c740cbaf6c76",
    faces: &[FaceDef {
        name: "Great Divide Guide",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::SCOUT, creature::ALLY],
        power: Some(1),
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
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Static(StaticAbility {
        layer: Layer::Ability,
        filter: Filter::And(&[
            Filter::Or(&[
                Filter::HasType(TypeSet::LAND),
                Filter::HasSubtype(creature::ALLY),
            ]),
            Filter::ControlledByYou,
        ]),
        modifier: Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: ANY_COLOR_MANA,
            mana_ability: true,
        },
        cross_zone: false,
    })],
};

#[cfg(test)]
mod tests {}
