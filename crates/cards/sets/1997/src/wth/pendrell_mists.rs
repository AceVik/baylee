//! Pendrell Mists — {3}{U} — Enchantment
//! Oracle: All creatures have "At the beginning of your upkeep, sacrifice this creature unless you pay {1}."
//! Set: WTH #47 — Weatherlight | Scryfall ID: b902b972-3a93-4e4e-aa77-02ada81e6b95 | Oracle ID: b18b8415-a0b1-459d-8c6b-896381184935
// IMPLEMENTED — Grants every creature "At the beginning of your upkeep, sacrifice this creature unless you pay {1}."

use baylee_cards_dsl::prelude::*;

static PENDRELL_TAX: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effect: &Effect::SacrificeSelf,
}];

card!(
    index = index::PENDRELL_MISTS,
    oracle_id = "b18b8415-a0b1-459d-8c6b-896381184935",
    scryfall_id = "b902b972-3a93-4e4e-aa77-02ada81e6b95",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pendrell Mists",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::GrantTriggered {
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            effects: PENDRELL_TAX,
            target: None,
        },
    ),],
);
