//! Cori Mountain Monastery — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains or an Island.
//! Oracle: {T}: Add {R}.
//! Oracle: {3}{R}, {T}: Exile the top card of your library. Until the end of your next turn, you may play that card.
//! Set: TDM #252 — Tarkir: Dragonstorm | Scryfall ID: 9312821a-2059-4f44-9b20-c9522b827e38 | Oracle ID: 35c60b66-8c85-432e-90fe-99c19d21ed15
// PARTIAL — the check-land entry (tapped unless a Plains or an Island) and
// {T}: Add {R}; the impulse-draw ability has no construct and is dropped.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

/// "a Plains or an Island you control" — control clause included, because
/// the entry check walks the whole battlefield and scopes nothing itself.
static PLAINS_OR_ISLAND_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::CORI_MOUNTAIN_MONASTERY,
    oracle_id = "35c60b66-8c85-432e-90fe-99c19d21ed15",
    scryfall_id = "9312821a-2059-4f44-9b20-c9522b827e38",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "the {3}{R}, {T} ability: no Effect exiles the top card of a library, \
         and no Modifier grants permission to play a card from exile"
    ),
    faces = &[face!(
        name = "Cori Mountain Monastery",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&PLAINS_OR_ISLAND_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        // NOT SUPPORTED: {3}{R}, {T}: Exile the top card of your library.
        // Until the end of your next turn, you may play that card. No
        // TargetSpec reaches the top card of a library, and the permission
        // to play it from exile has no Modifier.
    ],
);
