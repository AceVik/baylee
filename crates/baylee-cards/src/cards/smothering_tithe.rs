//! Smothering Tithe — {3}{W} — Enchantment
//! Oracle: Whenever an opponent draws a card, that player may pay {2}. If they don't, you create a Treasure token. (It's an artifact with "{T}, Sacrifice this artifact: Add one mana of any color.")
//! Set: 2X2 #32 — Double Masters 2022 | Scryfall ID: 861b5889-0183-4bee-afeb-a4b2aa700a8e | Oracle ID: 153376c9-dffd-458c-8ce3-a4c8269bc4e9
// IMPLEMENTED — opponent-choice {2} tax → Treasure tokens.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
    PlayerRel, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, artifact};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static TREASURE: TokenDef = TokenDef {
    name: "Treasure",
    colors: ColorSet::EMPTY,
    types: TypeSet::ARTIFACT,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[artifact::TREASURE],
    power: None,
    toughness: None,
    keywords: KeywordSet::EMPTY,
};
static MAKE_TREASURE: Effect = Effect::CreateToken { token: &TREASURE };

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(147),
    oracle_id: "153376c9-dffd-458c-8ce3-a4c8269bc4e9",
    scryfall_id: "861b5889-0183-4bee-afeb-a4b2aa700a8e",
    faces: &[FaceDef {
        name: "Smothering Tithe",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::Draws(PlayerRel::Opponent),
        effects: &[Effect::PlayerMayPayOr {
            player: PlayerRel::Opponent,
            mana: 2,
            effect: &MAKE_TREASURE,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
