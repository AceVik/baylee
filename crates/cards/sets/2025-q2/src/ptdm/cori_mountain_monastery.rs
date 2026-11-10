//! Cori Mountain Monastery — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains or an Island.
//! Oracle: {T}: Add {R}.
//! Oracle: {3}{R}, {T}: Exile the top card of your library. Until the end of your next turn, you may play that card.
//! Set: TDM #252 — Tarkir: Dragonstorm | Scryfall ID: 9312821a-2059-4f44-9b20-c9522b827e38 | Oracle ID: 35c60b66-8c85-432e-90fe-99c19d21ed15

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

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
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cori Mountain Monastery",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&PLAINS_OR_ISLAND_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{3}{R}", TapSelf),
            &[Effect::ExileTopToPlay {
                who: PlayerRel::You,
                play: ExilePlay {
                    count: 1,
                    window: PlayWindow::ThroughYourNextTurn,
                    free: false,
                    mana_any_type: false,
                    spell_types: None,
                }
            },]
        ),
    ],
);
