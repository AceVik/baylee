//! Monumental Henge — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Plains.
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}{W}, {T}: Look at the top five cards of your library. You may reveal a historic card from among them and put it into your hand. Put the rest on the bottom of your library in a random order. (Artifacts, legendaries, and Sagas are historic.)
//! Set: MH3 #222 — Modern Horizons 3 | Scryfall ID: 62907e7b-e531-4f51-9a69-7e60ae525775 | Oracle ID: c48df45c-3513-4d56-aed6-30c2f3a759cd

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

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
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{2}{W}{W}", TapSelf),
            &[Effect::LookAtTopFind {
                found: SearchDest::Hand,
                count: 5,
                filter: &Filter::Or(&[
                    Filter::ARTIFACT,
                    Filter::HasSupertype(SupertypeSet::LEGENDARY),
                    Filter::HasSubtype(baylee_core::generated::subtypes::enchantment::SAGA)
                ]),
                random: true
            }]
        ),
    ],
);
