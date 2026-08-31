//! Ancestral Vision — (no cost) — Sorcery
//! Oracle: Suspend 4—{U} (Rather than cast this card from your hand, pay {U} and exile it with four time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)
//! Oracle: Target player draws three cards.
//! Set: TDC #144 — Tarkir: Dragonstorm Commander | Scryfall ID: 9ec075ba-db56-4dcf-b1dc-fe6270b7ab36 | Oracle ID: 9728dec9-d482-4c7a-8cdc-44d010dc878d
// IMPLEMENTED — suspend 4 with countdown and free cast at zero.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
    PlayerRel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(4),
    oracle_id: "9728dec9-d482-4c7a-8cdc-44d010dc878d",
    scryfall_id: "9ec075ba-db56-4dcf-b1dc-fe6270b7ab36",
    faces: &[FaceDef {
        name: "Ancestral Vision",
        types: TypeSet::SORCERY,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Suspend {
            counters: 4,
            cost: baylee_core::mana!("{U}"),
        },
        AbilityDef::Spell {
            effects: &[Effect::DrawCardsFor {
                amount: Amount::Fixed(3),
                who: PlayerRel::Chosen,
            }],
            targets: Some(baylee_cards_dsl::TargetReq::one(
                baylee_cards_dsl::TargetSpec::AnyPlayer,
            )),
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
