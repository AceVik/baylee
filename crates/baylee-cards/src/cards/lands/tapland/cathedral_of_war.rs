//! Cathedral of War — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: Exalted (Whenever a creature you control attacks alone, that creature gets +1/+1 until end of turn.)
//! Oracle: {T}: Add {C}.
//! Set: M13 #221 — Magic 2013 | Scryfall ID: dd222c07-0b28-41cb-9237-ad7991ab078f | Oracle ID: 5ff647e4-730a-498f-8f2c-5bd64d5a9780
// IMPLEMENTED — enters tapped, and taps for {C}.
// NOT SUPPORTED: Exalted — "whenever a creature you control attacks alone":
// `Trigger::Attacks(&filter)` fires on any attacker and `Condition` can only
// count ("at least N"), so neither can state that the attacker is the *only*
// one. The +1/+1 has nowhere to hang.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CATHEDRAL_OF_WAR,
    oracle_id = "5ff647e4-730a-498f-8f2c-5bd64d5a9780",
    scryfall_id = "dd222c07-0b28-41cb-9237-ad7991ab078f",
    faces = &[face!(
        name = "Cathedral of War",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage =
        Coverage::Partial("Exalted: no Trigger or Condition variant expresses \"attacks alone\""),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
