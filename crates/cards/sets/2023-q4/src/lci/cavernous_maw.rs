//! Cavernous Maw — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {2}: This land becomes a 3/3 Elemental creature until end of turn. It's still a Cave land. Activate only if the number of other Caves you control plus the number of Cave cards in your graveyard is three or greater.
//! Set: LCI #270 — The Lost Caverns of Ixalan | Scryfall ID: 2a51ebf6-a465-42e2-82b7-d2cb928ca632 | Oracle ID: 952ab8fe-f7d3-4673-89de-8c6d3f8a081f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CAVERNOUS_MAW,
    oracle_id = "952ab8fe-f7d3-4673-89de-8c6d3f8a081f",
    scryfall_id = "2a51ebf6-a465-42e2-82b7-d2cb928ca632",
    faces = &[face!(
        name = "Cavernous Maw",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::ELEMENTAL),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 3),
                    Duration::UntilEndOfTurn,
                ),
            ],
            condition = Some(Condition::AmountAtLeast(
                Amount::Sum {
                    left: &Amount::CountOf {
                        filter: &Filter::And(&[
                            Filter::HasSubtype(subtypes::land::CAVE),
                            Filter::ControlledByYou,
                            Filter::Another
                        ]),
                        zone: ZoneSel::Battlefield
                    },
                    right: &Amount::CountOf {
                        filter: &Filter::HasSubtype(subtypes::land::CAVE),
                        zone: ZoneSel::GraveyardYou
                    },
                },
                3
            )),
        ),
    ],
);
