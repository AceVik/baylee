//! Mosswort Bridge — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {G}, {T}: You may play the exiled card without paying its mana cost if creatures you control have total power 10 or greater.
//! Set: TDC #379 — Tarkir: Dragonstorm Commander | Scryfall ID: 1490c0dc-06d6-45ee-af3c-2935b0ab1233 | Oracle ID: 7cb9e29f-835f-4155-a2a5-4b778866c773
// PARTIAL — "enters tapped" and "{T}: Add {G}" are built; hideaway and the
// free-play ability are clauses the DSL cannot say, so the card is not
// playable as printed.
// NOT SUPPORTED: "Hideaway 4" — nothing exiles one of the top four cards of a
// library face down and puts the rest on the bottom in a random order; the
// nearest variant, Effect::LookAtTopPick, puts the kept card into your hand,
// and no effect ever reads a face-down exiled card back.
// NOT SUPPORTED: "{G}, {T}: You may play the exiled card without paying its
// mana cost if creatures you control have total power 10 or greater" — no
// effect plays a card from exile for free (Effect::GrantsFlashback casts from
// a graveyard for its own cost), and no Condition reads the total power of a
// set of creatures (Condition::ControlCount counts permanents, not power).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOSSWORT_BRIDGE,
    oracle_id = "7cb9e29f-835f-4155-a2a5-4b778866c773",
    scryfall_id = "1490c0dc-06d6-45ee-af3c-2935b0ab1233",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Mosswort Bridge",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "hideaway 4 (exile one of the top four face down, rest on the bottom) and the {G},{T} free-play ability have no DSL variant"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
