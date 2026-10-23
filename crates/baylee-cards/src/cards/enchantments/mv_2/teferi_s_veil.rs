//! Teferi's Veil — {1}{U} — Enchantment
//! Oracle: Whenever a creature you control attacks, it phases out at end of combat. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: WTH #53 — Weatherlight | Scryfall ID: cbf39b80-d972-4f79-902f-cc613c32e446 | Oracle ID: c6db2ea4-954c-443b-9603-c41596555c31
// PARTIAL — Grants 'Whenever this creature attacks, it phases out at end of combat' to creatures you control so attacking creatures phase out at end of combat; direct trigger on Teferi's Veil phasing out the attacking creature without granting an ability is not supported.

use baylee_cards_dsl::prelude::*;

static PHASE_OUT_AT_END_OF_COMBAT: &[Effect] = &[Effect::AtEndOfCombat {
    about: TargetSpec::ThisObject,
    effects: &[Effect::PhaseOut { target: None }],
}];

card!(
    index = index::TEFERI_S_VEIL,
    oracle_id = "c6db2ea4-954c-443b-9603-c41596555c31",
    scryfall_id = "cbf39b80-d972-4f79-902f-cc613c32e446",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Grants 'Whenever this creature attacks, it phases out at end of combat' to creatures you control so attacking creatures phase out at end of combat; direct trigger on Teferi's Veil phasing out the attacking creature without granting an ability is not supported.",
    ),
    faces = &[face!(
        name = "Teferi's Veil",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
        Modifier::GrantTriggered {
            trigger: Trigger::Attacks(&Filter::This),
            effects: PHASE_OUT_AT_END_OF_COMBAT,
            target: None,
        },
    ),],
);
