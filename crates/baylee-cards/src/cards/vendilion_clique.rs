//! Vendilion Clique — {1}{U}{U} — Legendary Creature — Faerie Wizard
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When Vendilion Clique enters, look at target player's hand. You may choose a nonland card from it. If you do, that player reveals the chosen card, puts it on the bottom of their library, then draws a card.
//! Set: SLD #110 — Secret Lair Drop | Scryfall ID: cd702cf1-10ca-4448-9fb1-b6de635e839c | Oracle ID: 244d4807-0802-41bc-9460-55ac38a28a72
// PARTIAL — flash/flying + hand-attack implemented (choose a nonland card
// from the target player's hand, bottom it, draw). The look/reveal
// presentation is a protocol M3 item; heads-up the target is the opponent.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONLAND: Filter = Filter::LacksType(TypeSet::LAND);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(181),
    oracle_id: "244d4807-0802-41bc-9460-55ac38a28a72",
    scryfall_id: "cd702cf1-10ca-4448-9fb1-b6de635e839c",
    faces: &[FaceDef {
        name: "Vendilion Clique",
        mana_cost: baylee_core::mana!("{1}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::FAERIE, creature::WIZARD],
        power: Some(3),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("look/reveal presentation (M3); target player choice for MP (M3)"),
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[
            Effect::BottomCardFromHand {
                player: PlayerRel::Opponent,
                filter: &NONLAND,
            },
            Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::Opponent,
            },
        ],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
