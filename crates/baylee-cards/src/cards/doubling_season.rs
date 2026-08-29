//! Doubling Season — {4}{G} — Enchantment
//! Oracle: If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
//! Oracle: If an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.
//! Set: FDN #216 — Foundations | Scryfall ID: f2c4f80e-84a0-463b-82c3-5c6503809351 | Oracle ID: 01546b7d-a233-4176-8843-d732074dc5b6
// IMPLEMENTED — token creation and counter placement doubling (all counter
// kinds on permanents). NOT SUPPORTED yet: planeswalker ETB-loyalty
// doubling routes through this same hook once walkers land (M2.S7).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, PartnerKind,
    ReplacementRule,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOURS: Filter = Filter::ControlledByYou;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(35),
    oracle_id: "01546b7d-a233-4176-8843-d732074dc5b6",
    scryfall_id: "f2c4f80e-84a0-463b-82c3-5c6503809351",
    faces: &[FaceDef {
        name: "Doubling Season",
        mana_cost: baylee_core::mana!("{4}{G}"),
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
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("planeswalker ETB-loyalty doubling lands with walkers (M2.S7)"),
    abilities: &[
        AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation {
            controller_filter: &YOURS,
        }),
        AbilityDef::Replacement(ReplacementRule::DoubleCounterPlacement {
            object_filter: &YOURS,
        }),
    ],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage in baylee-engine s6 tests: Maskwood Nexus's
    // token ability creates two Shapeshifters with Doubling Season out.
}
