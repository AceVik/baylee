//! Great Divide Guide — {1}{G} — Creature — Human Scout Ally
//! Oracle: Each land and Ally you control has "{T}: Add one mana of any color."
//! Set: TLA #181 — Avatar: The Last Airbender | Scryfall ID: cc3063ec-5ea6-46c1-8331-c740cbaf6c76 | Oracle ID: 79e69a91-d580-47fb-be76-1e32c50d2fa0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(62),
    oracle_id: "79e69a91-d580-47fb-be76-1e32c50d2fa0",
    scryfall_id: "cc3063ec-5ea6-46c1-8331-c740cbaf6c76",
    faces: &[FaceDef {
        name: "Great Divide Guide",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::creature::HUMAN,
            subtypes::creature::SCOUT,
            subtypes::creature::ALLY,
        ],
        power: Some(2),
        toughness: Some(3),
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
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
