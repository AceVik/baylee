//! Aura Flux — {2}{U} — Enchantment
//! Oracle: Other enchantments have "At the beginning of your upkeep, sacrifice this enchantment unless you pay {2}."
//! Set: ULG #27 — Urza's Legacy | Scryfall ID: a6be1542-70b8-4e97-a951-100966dc46ce | Oracle ID: 128304af-4555-4276-bf26-638017beb3ff
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const OTHER_ENCHANTMENT: Filter = Filter::And(&[Filter::ENCHANTMENT, Filter::Another]);

static AURA_FLUX_TAX: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::You,
    mana: Amount::Fixed(2),
    effect: &Effect::SacrificeSelf,
}];

card!(
    index = index::AURA_FLUX,
    oracle_id = "128304af-4555-4276-bf26-638017beb3ff",
    scryfall_id = "a6be1542-70b8-4e97-a951-100966dc46ce",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aura Flux",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        OTHER_ENCHANTMENT,
        Modifier::GrantTriggered {
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            effects: AURA_FLUX_TAX,
            target: None,
        },
    )],
);
