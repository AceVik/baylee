//! Alchemist's Refuge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {G}{U}, {T}: You may cast spells this turn as though they had flash.
//! Set: SOC #360 — Secrets of Strixhaven Commander | Scryfall ID: 36a11c11-1ea9-48e2-b1bc-49cba1390620 | Oracle ID: 357ed28b-899f-404b-94ff-6fb2ef81d87b
// PARTIAL — {T}: Add {C} written; the flash grant has no Modifier, see below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALCHEMIST_S_REFUGE,
    oracle_id = "357ed28b-899f-404b-94ff-6fb2ef81d87b",
    scryfall_id = "36a11c11-1ea9-48e2-b1bc-49cba1390620",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[face!(name = "Alchemist's Refuge", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "{G}{U}, {T}: You may cast spells this turn as though they had flash — no Modifier reaches every spell; Modifier::SorceriesHaveFlash is sorcery-scoped (Teferi, Time Raveler)"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{G}{U}, {T}: You may cast spells this turn as though they
// had flash." The vocabulary's only flash-granting modifier is
// Modifier::SorceriesHaveFlash, which lifts the timing restriction on
// sorceries alone — this clause covers every spell, and casting a creature or
// any other permanent at instant speed is the reason the card sees play. The
// ability is left off rather than shipped narrower than the card prints it.
