//! Monumental Henge — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains.
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}{W}, {T}: Look at the top five cards of your library. You may reveal a historic card from among them and put it into your hand. Put the rest on the bottom of your library in a random order. (Artifacts, legendaries, and Sagas are historic.)
//! Set: MH3 #222 — Modern Horizons 3 | Scryfall ID: 62907e7b-e531-4f51-9a69-7e60ae525775 | Oracle ID: c48df45c-3513-4d56-aed6-30c2f3a759cd
// PARTIAL — the enter-tapped condition (unless you control a Plains) and
// {T}: Add {W} are implemented; the {2}{W}{W} historic look-and-pick is
// NOT SUPPORTED below, which is what keeps this card Partial.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::MONUMENTAL_HENGE,
    oracle_id = "c48df45c-3513-4d56-aed6-30c2f3a759cd",
    scryfall_id = "62907e7b-e531-4f51-9a69-7e60ae525775",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Monumental Henge",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&f!(
            your Filter::HasSubtype(land::PLAINS)
        ))],
    )],
    coverage = Coverage::Partial(
        "the {2}{W}{W} ability: Effect::LookAtTopPick carries no filter and no \"may\", so the \
         printed \"You may reveal a historic card from among them and put it into your hand\" \
         cannot be said — the ability is left off rather than widened to any of the top five",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: "{2}{W}{W}, {T}: Look at the top five cards of your library. You may
        // reveal a historic card from among them and put it into your hand. Put the rest on the
        // bottom of your library in a random order." — the historic restriction (artifact,
        // legendary, or Saga) and the optional reveal have no expression on LookAtTopPick.
    ],
);
