//! Nivix, Aerie of the Firemind — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}{U}{R}, {T}: Exile the top card of your library. Until your next turn, you may cast it if it's an instant or sorcery spell.
//! Set: DDJ #36 — Duel Decks: Izzet vs. Golgari | Scryfall ID: a025a42d-cbcc-4bd2-9331-5c7dc677da20 | Oracle ID: 9c482f1d-08b4-4882-918c-448a556d3fbe

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NIVIX_AERIE_OF_THE_FIREMIND,
    oracle_id = "9c482f1d-08b4-4882-918c-448a556d3fbe",
    scryfall_id = "a025a42d-cbcc-4bd2-9331-5c7dc677da20",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Nivix, Aerie of the Firemind",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}{U}{R}", TapSelf),
            &[Effect::ExileTopToPlay {
                who: PlayerRel::You,
                play: ExilePlay {
                    count: 1,
                    window: PlayWindow::UntilYourNextTurn,
                    free: false,
                    mana_any_type: false,
                    spell_types: Some(TypeSet::INSTANT.union(TypeSet::SORCERY)),
                }
            },]
        ),
    ],
);
