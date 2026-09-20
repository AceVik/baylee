//! Windbrisk Heights — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: {W}, {T}: You may play the exiled card without paying its mana cost if you attacked with three or more creatures this turn.
//! Set: TDC #411 — Tarkir: Dragonstorm Commander | Scryfall ID: df441bae-3b9c-46a9-8287-870c00d58a64 | Oracle ID: 3589bcfc-42b0-414a-adce-bc690dc631c8
// PARTIAL — enters tapped plus {T}: Add {W}; hideaway and the play-the-exiled-card clause have no vocabulary.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDBRISK_HEIGHTS,
    oracle_id = "3589bcfc-42b0-414a-adce-bc690dc631c8",
    scryfall_id = "df441bae-3b9c-46a9-8287-870c00d58a64",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Windbrisk Heights",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "hideaway 4 (exile one of the top four cards face down, the rest to the bottom in a random order) and \"{W}, {T}: You may play the exiled card without paying its mana cost if you attacked with three or more creatures this turn\" are not expressible"
    ),
    // NOT SUPPORTED: Hideaway 4 — no effect looks at the top of the library and
    // exiles one card face down, no object records the exiled card, and nothing
    // randomises the order of the cards put on the bottom.
    // NOT SUPPORTED: "{W}, {T}: You may play the exiled card without paying its
    // mana cost if you attacked with three or more creatures this turn." — no
    // Effect plays a card from exile, with or without its mana cost, and
    // Condition has no variant for having attacked with three or more creatures.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
);
