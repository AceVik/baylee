//! Shelldock Isle — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it.
//! Set: LRW #272 — Lorwyn | Scryfall ID: 4216656e-90e8-45fc-a0f6-0d0d79d0a021 | Oracle ID: f748b2fb-6c2a-400a-8e96-fa4e4a1dfe80
// PARTIAL — enters tapped and taps for {U}; the two clauses below have no
// vocabulary and are named with `// NOT SUPPORTED:` lines.
// NOT SUPPORTED: Hideaway 4 — no effect looks at the top N cards of a
// library, exiles one of them face down and puts the rest on the bottom in a
// random order. `Effect::LookAtTopPick` comes closest and is a different
// sentence: its pick goes to hand and is revealed, and its rest go to the
// bottom in the chooser's order.
// NOT SUPPORTED: "{U}, {T}: You may play the exiled card without paying its
// mana cost if a library has twenty or fewer cards in it" — no effect grants
// playing a card from exile (`Modifier::PlayLandsFromGraveyard` and
// `Modifier::GrantsFlashback` are the graveyard's own two sentences), and no
// `Condition` counts a library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHELLDOCK_ISLE,
    oracle_id = "f748b2fb-6c2a-400a-8e96-fa4e4a1dfe80",
    scryfall_id = "4216656e-90e8-45fc-a0f6-0d0d79d0a021",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shelldock Isle",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "Hideaway 4 is not expressible, and the play-from-exile ability needs a permission to play a card from exile plus a condition on a library's size"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
