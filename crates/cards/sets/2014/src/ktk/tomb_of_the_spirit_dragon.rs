//! Tomb of the Spirit Dragon — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: You gain 1 life for each colorless creature you control.
//! Set: CMM #1048 — Commander Masters | Scryfall ID: 824bf3cc-fb35-4bc1-998c-6758b1eed2ca | Oracle ID: 22f6391e-2634-440f-af1b-9581d1bff818
// IMPLEMENTED — {T}: Add {C}; {2}, {T}: gain 1 life for each colorless
// creature you control (Amount::CountOf over the battlefield).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOMB_OF_THE_SPIRIT_DRAGON,
    oracle_id = "22f6391e-2634-440f-af1b-9581d1bff818",
    scryfall_id = "824bf3cc-fb35-4bc1-998c-6758b1eed2ca",
    faces = &[face!(
        name = "Tomb of the Spirit Dragon",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::GainLife {
                amount: Amount::CountOf {
                    filter: &f!(your colorless CREATURE),
                    zone: ZoneSel::Battlefield,
                },
            }]
        ),
    ],
);
