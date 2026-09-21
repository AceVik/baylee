//! Tomb Fortress — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {2}{B}{B}{B}, {T}, Exile this land: Mill four cards, then return a creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: 40K #168 — Warhammer 40,000 Commander | Scryfall ID: 5ffd1a43-5613-4058-9e51-fd3cc1585651 | Oracle ID: 986f510c-e2ec-423e-a443-51a169939558
// PARTIAL — enters tapped (`EnterModifier::Tapped`) and taps for {B}; the
// graveyard-return activation is off the card, see the note below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "{2}{B}{B}{B}, {T}, Exile this land: Mill four cards, then
// return a creature card from your graveyard to the battlefield. Activate
// only as a sorcery." — the printed sentence chooses the card as it resolves,
// and `Effect::GraveyardToBattlefield` acts on a `TargetSpec`, which is a
// target chosen when the ability is activated: it would have to be a creature
// card already in the graveyard *before* the mill, and an ability would fizzle
// whole if that card left. There is no untargeted chooser for this destination
// (the `Effect::ReturnChosenToHand` shape, which returns a permanent), so the
// ability stays off the card rather than being spelled as a targeted one.

card!(
    index = index::TOMB_FORTRESS,
    oracle_id = "986f510c-e2ec-423e-a443-51a169939558",
    scryfall_id = "5ffd1a43-5613-4058-9e51-fd3cc1585651",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tomb Fortress",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the activation's \"return a creature card from your graveyard to the battlefield\" is not a target, and the DSL has no untargeted chooser for that destination",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),],
);
