//! Elspeth, Storm Slayer — {3}{W}{W} — Legendary Planeswalker — Elspeth
//! Oracle: If one or more tokens would be created under your control, twice that many of those tokens are created instead.
//! Oracle: +1: Create a 1/1 white Soldier creature token.
//! Oracle: 0: Put a +1/+1 counter on each creature you control. Those creatures gain flying until your next turn.
//! Oracle: −3: Destroy target creature an opponent controls with mana value 3 or greater.
//! Set: TDM #11 — Tarkir: Dragonstorm | Scryfall ID: 73a065e3-b530-4e62-ab3c-4f6f908184ec | Oracle ID: f78af825-023a-42e9-8374-5c52303a1417
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(40),
    oracle_id: "f78af825-023a-42e9-8374-5c52303a1417",
    scryfall_id: "73a065e3-b530-4e62-ab3c-4f6f908184ec",
    faces: &[FaceDef {
        name: "Elspeth, Storm Slayer",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::planeswalker::ELSPETH],
        power: None,
        toughness: None,
        loyalty: Some(5),
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
