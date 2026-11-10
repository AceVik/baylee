//! Krosan Verge — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Search your library for a Forest card and a Plains card, put them onto the battlefield tapped, then shuffle.
//! Set: OTC #304 — Outlaws of Thunder Junction Commander | Scryfall ID: 19fc5bec-f877-430c-8e83-e6c5fe97f3c4 | Oracle ID: d9a10971-f32b-4978-952d-fed0a5bc9e36
// IMPLEMENTED — enters tapped; {T}: Add {C}; {2}, {T}, Sacrifice this land:
// search for a Forest card and a Plains card, both onto the battlefield
// tapped, then shuffle.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::KROSAN_VERGE,
    oracle_id = "d9a10971-f32b-4978-952d-fed0a5bc9e36",
    scryfall_id = "19fc5bec-f877-430c-8e83-e6c5fe97f3c4",
    faces = &[face!(
        name = "Krosan Verge",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // `SearchLibrary` shares one `filter` across every `Find` in it, and
        // this clause names two different qualities — a Forest card *and* a
        // Plains card — so the one printed search that may find one of each
        // is two searches here, one per quality. Nothing is dropped: a
        // filter of "Forest or Plains" over two finds would also allow two
        // Forests, which the card does not print.
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[
                Effect::SearchLibrary {
                    filter: &Filter::HasSubtype(land::FOREST),
                    finds: &[Find::BATTLEFIELD_TAPPED],
                    optional: false,
                },
                Effect::SearchLibrary {
                    filter: &Filter::HasSubtype(land::PLAINS),
                    finds: &[Find::BATTLEFIELD_TAPPED],
                    optional: false,
                },
            ]
        ),
    ],
);
