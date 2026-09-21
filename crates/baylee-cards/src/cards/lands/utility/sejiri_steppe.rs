//! Sejiri Steppe — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, target creature you control gains protection from the color of your choice until end of turn.
//! Oracle: {T}: Add {W}.
//! Set: DDG #36 — Duel Decks: Knights vs. Dragons | Scryfall ID: d45b0ed8-8692-4fa7-b32c-30d29028da3d | Oracle ID: 3dfbf95e-a91b-429c-96e2-95ac777e7027
// PARTIAL — enters tapped and {T}: Add {W} are built; protection from a color chosen on resolution has no DSL variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEJIRI_STEPPE,
    oracle_id = "3dfbf95e-a91b-429c-96e2-95ac777e7027",
    scryfall_id = "d45b0ed8-8692-4fa7-b32c-30d29028da3d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sejiri Steppe",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "granting protection from a color chosen on resolution is not expressible in the DSL"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        // NOT SUPPORTED: "When this land enters, target creature you control gains protection from the color of your choice until end of turn."
    ],
);
