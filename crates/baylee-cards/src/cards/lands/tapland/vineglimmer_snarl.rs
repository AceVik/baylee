//! Vineglimmer Snarl — (no cost) — Land
//! Oracle: As this land enters, you may reveal a Forest or Island card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Set: SOC #420 — Secrets of Strixhaven Commander | Scryfall ID: 445bd162-b523-45f4-83d0-ffc702fc1fac | Oracle ID: 33f52df8-4b44-4422-8b0a-37fead9c894b
// IMPLEMENTED — {T} for {G} or {U}; the entry clause stays undecided (Partial).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VINEGLIMMER_SNARL,
    oracle_id = "33f52df8-4b44-4422-8b0a-37fead9c894b",
    scryfall_id = "445bd162-b523-45f4-83d0-ffc702fc1fac",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    // NOT SUPPORTED: "As this land enters, you may reveal a Forest or Island
    // card from your hand. If you don't, this land enters tapped." —
    // EnterModifier has no variant that reveals a card from hand, so the land
    // always enters untapped.
    faces = &[face!(name = "Vineglimmer Snarl", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the enters-tapped-unless-you-reveal-a-Forest-or-Island entry clause has no EnterModifier variant",
    ),
    abilities = &[mana_ability!(&[Effect::mana_choice(&[
        ManaColor::Green,
        ManaColor::Blue,
    ])])],
);
