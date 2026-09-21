//! Great Hall of Starnheim — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {W}{W}{B}, {T}, Sacrifice this land and a creature you control: Create a 4/4 white Angel Warrior creature token with flying and vigilance. Activate only as a sorcery.
//! Set: KHM #259 — Kaldheim | Scryfall ID: a23c757e-5944-47ce-b06f-27b4c403044c | Oracle ID: 75d8ef26-b745-448b-94b9-3c64668ce171
// PARTIAL — the land enters tapped and taps for {B}; the token-making
// ability is off the card, see the NOT SUPPORTED note below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREAT_HALL_OF_STARNHEIM,
    oracle_id = "75d8ef26-b745-448b-94b9-3c64668ce171",
    scryfall_id = "a23c757e-5944-47ce-b06f-27b4c403044c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Great Hall of Starnheim",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "no TokenDef for a 4/4 white Angel Warrior token with flying and vigilance, and `no_card_file_defines_its_own_token` forbids writing one here"
    ),
    // NOT SUPPORTED: "{W}{W}{B}, {T}, Sacrifice this land and a creature you
    // control: Create a 4/4 white Angel Warrior creature token with flying and
    // vigilance. Activate only as a sorcery." — `Effect::CreateToken` needs a
    // `&'static TokenDef`, and the vocabulary carries no Angel Warrior: the
    // nearest entry, `tokens::ANGEL_4_4_WHITE_FLYING`, is 4/4 white with
    // flying and no vigilance, which is a different token (and not an Angel
    // Warrior), and a token is registered in `crate::tokens` rather than in a
    // card file. The cost itself is sayable — `cost!("{W}{W}{B}", TapSelf,
    // SacrificeSelf, Sacrifice(&Filter::YOUR_CREATURE))` at sorcery speed —
    // so the ability returns the moment the token exists.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);
