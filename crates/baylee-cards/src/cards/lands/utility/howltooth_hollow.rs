//! Howltooth Hollow — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}: You may play the exiled card without paying its mana cost if each player has no cards in hand.
//! Set: LRW #269 — Lorwyn | Scryfall ID: 5c678643-6640-49e1-a5b0-2954123bcabc | Oracle ID: 463fc699-f4fc-4112-a6b3-6dcb642203e6
// PARTIAL — enters tapped and {T}: Add {B}; Hideaway 4 and the
// play-the-exiled-card ability are not expressible in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HOWLTOOTH_HOLLOW,
    oracle_id = "463fc699-f4fc-4112-a6b3-6dcb642203e6",
    scryfall_id = "5c678643-6640-49e1-a5b0-2954123bcabc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Hideaway 4 (look at the top four, exile one face down, rest on the bottom in a random order) and playing the exiled card without paying its mana cost are not expressible"
    ),
    faces = &[face!(
        name = "Howltooth Hollow",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        // NOT SUPPORTED: "Hideaway 4 (When this land enters, look at the top
        // four cards of your library, exile one face down, then put the rest
        // on the bottom in a random order.)" — `Effect::LookAtTopPick` looks
        // and keeps to hand, and nothing exiles a chosen card face down or
        // randomizes a bottom order.
        // NOT SUPPORTED: "{B}, {T}: You may play the exiled card without
        // paying its mana cost if each player has no cards in hand." — no
        // permission to play a card linked out of exile, and no `Condition`
        // asking whether a player's hand is empty.
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
    ],
);
