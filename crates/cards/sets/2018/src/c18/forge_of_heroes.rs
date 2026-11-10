//! Forge of Heroes — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Choose target commander that entered this turn. Put a +1/+1 counter on it if it's a creature and a loyalty counter on it if it's a planeswalker.
//! Set: CMM #995 — Commander Masters | Scryfall ID: 5f11f053-5150-4a76-b2ca-9df3a0629911 | Oracle ID: 77807103-bcd5-479f-bedd-f5d97aa6d3d2

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORGE_OF_HEROES,
    oracle_id = "77807103-bcd5-479f-bedd-f5d97aa6d3d2",
    scryfall_id = "5f11f053-5150-4a76-b2ca-9df3a0629911",
    faces = &[face!(name = "Forge of Heroes", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(TapSelf),
            &[
                Effect::IfTargetMatches {
                    filter: &Filter::CREATURE,
                    then: &[Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(1)
                    }]
                },
                Effect::IfTargetMatches {
                    filter: &Filter::PLANESWALKER,
                    then: &[Effect::AddCounter {
                        kind: CounterKind::Loyalty,
                        amount: Amount::Fixed(1)
                    }]
                },
            ],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::IsCommander,
                Filter::EnteredThisTurn
            ])))
        ),
    ],
);
