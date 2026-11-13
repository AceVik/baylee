//! The Lonely Mountain — (no cost) — Land — Mountain
//! Oracle: ({T}: Add {R}.)
//! Oracle: This land enters tapped unless you control an Equipment.
//! Oracle: {4}{R}, {T}: Create a 2/2 red Dwarf creature token. This ability costs {1} less to activate for each Equipment you control. Activate only as a sorcery.
//! Set: HOB #187 — The Hobbit | Scryfall ID: b39ebc4d-a01a-4401-ab3a-bf6142c93b47 | Oracle ID: 3678c06f-8a33-4a6d-bf20-5b92d5c05a95

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_LONELY_MOUNTAIN,
    oracle_id = "3678c06f-8a33-4a6d-bf20-5b92d5c05a95",
    scryfall_id = "b39ebc4d-a01a-4401-ab3a-bf6142c93b47",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "The Lonely Mountain",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::MOUNTAIN],
        enter_modifiers = &[EnterModifier::TappedUnless(&f!(
            your Filter::HasSubtype(subtypes::artifact::EQUIPMENT)
        ))],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::intrinsic_mana()]),
        activated!(
            cost!("{4}{R}", TapSelf),
            &[Effect::CreateToken {
                token: &crate::generated_tokens::DWARF_2_2_RED
            }],
            timing = ActivationTiming::SorcerySpeed,
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountOf {
                    filter: &Filter::And(&[
                        Filter::HasSubtype(subtypes::artifact::EQUIPMENT),
                        Filter::ControlledByYou
                    ]),
                    zone: ZoneSel::Battlefield
                },
                each: 1
            }),
        ),
    ],
);
