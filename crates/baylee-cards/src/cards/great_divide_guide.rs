//! Great Divide Guide — {1}{G} — Creature — Human Scout Ally
//! Oracle: Each land and Ally you control has "{T}: Add one mana of any color."
//! Set: TLA #181 — Avatar: The Last Airbender | Scryfall ID: cc3063ec-5ea6-46c1-8331-c740cbaf6c76 | Oracle ID: 79e69a91-d580-47fb-be76-1e32c50d2fa0
// IMPLEMENTED — the land/Ally any-color mana grant (GrantActivated
// static, same machinery as Chromatic Lantern).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    ALL_MANA_COLORS, ANY_COLOR_MANA, AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef,
    CommanderRule, Cost, Coverage, Effect, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(62),
    oracle_id: "79e69a91-d580-47fb-be76-1e32c50d2fa0",
    scryfall_id: "cc3063ec-5ea6-46c1-8331-c740cbaf6c76",
    faces: &[FaceDef {
        name: "Great Divide Guide",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::SCOUT, creature::ALLY],
        power: Some(1),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
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
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
