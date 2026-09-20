//! Frostboil Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Island or Mountain card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Set: MSC #246 — Marvel Super Heroes Commander | Scryfall ID: ae705a26-5371-4236-904c-fe3e58b4721d | Oracle ID: 7137aae6-260d-41de-8b4e-42a8cf752697
// IMPLEMENTED — the mana ability; the reveal-as-it-enters clause is not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FROSTBOIL_SNARL,
    oracle_id = "7137aae6-260d-41de-8b4e-42a8cf752697",
    scryfall_id = "ae705a26-5371-4236-904c-fe3e58b4721d",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(name = "Frostboil Snarl", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no EnterModifier asks the controller to reveal a card from hand, so the land always enters untapped"
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Blue,
        ManaColor::Red,
    ])])],
);

// NOT SUPPORTED: "As this land enters, you may reveal an Island or Mountain
// card from your hand. If you don't, this land enters tapped." — the
// `FaceDef::enter_modifiers` vocabulary (Tapped, TappedUnless,
// TappedUnlessCount, TappedOrPayLife, …) has no variant that reveals a card
// from hand, so the arrival cannot be made conditional on it.
