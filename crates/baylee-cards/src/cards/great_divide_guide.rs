//! Great Divide Guide — {1}{G} — Creature — Human Scout Ally
//! Oracle: Each land and Ally you control has "{T}: Add one mana of any color."
//! Set: TLA #181 — Avatar: The Last Airbender | Scryfall ID: cc3063ec-5ea6-46c1-8331-c740cbaf6c76 | Oracle ID: 79e69a91-d580-47fb-be76-1e32c50d2fa0
// PARTIAL — the mana-ability GRANT to lands and Allies needs layer-6
// ability grants (not keyword modifiers; tracked for M2.S7+). The card is
// otherwise playable (an Ally itself).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
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
    coverage: Coverage::Partial(
        "mana-ability grant to lands/Allies (layer-6 ability grants, M2.S7+)",
    ),
    abilities: &[],
};

#[cfg(test)]
mod tests {}
