//! Reflections of Littjara — {4}{U} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Whenever you cast a spell of the chosen type, copy that spell. (A copy of a permanent spell becomes a token.)
//! Set: TDC #164 — Tarkir: Dragonstorm Commander | Scryfall ID: 578a1846-8c1a-4013-b669-1d3f4ddbbaa3 | Oracle ID: c3fdfb94-2d10-4743-864c-a59fdd57d8b7
// IMPLEMENTED — choose-a-type + cast-triggered spell copy. Copies are
// card-less objects (tokens) already, matching the "a copy of a
// permanent spell becomes a token" rule.

static YOUR_SPELL_OF_CHOSEN_TYPE: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::MatchesChosenTypeOfSource]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 129,
    oracle_id: "c3fdfb94-2d10-4743-864c-a59fdd57d8b7",
    scryfall_id: "578a1846-8c1a-4013-b669-1d3f4ddbbaa3",
    faces: &[face! {
        name: "Reflections of Littjara",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::ENCHANTMENT,
        enter_modifiers: &[EnterModifier::ChooseSubtype],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::SpellCast(&YOUR_SPELL_OF_CHOSEN_TYPE), &[Effect::CopyTargetSpell { mods: &[] }], targets: Some(TargetReq::one(TargetSpec::EventObject)))],
}
