//! Gavony Township — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
//! Set: MOC #406 — March of the Machine Commander | Scryfall ID: ce46c4e2-a515-41d7-8d70-d20cf4925996 | Oracle ID: 8a44e4e7-dfa2-427b-bbff-11c398fa60bb
// IMPLEMENTED — {T} for {C}, and a {2}{G}{W}, {T} activation that puts a
// +1/+1 counter on each creature you control.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAVONY_TOWNSHIP,
    oracle_id = "8a44e4e7-dfa2-427b-bbff-11c398fa60bb",
    scryfall_id = "ce46c4e2-a515-41d7-8d70-d20cf4925996",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Gavony Township", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}{G}{W}", TapSelf),
            &[Effect::AddCounterFilter {
                filter: &Filter::YOUR_CREATURE,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
    ],
);
