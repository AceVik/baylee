//! Castle Embereth — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mountain.
//! Oracle: {T}: Add {R}.
//! Oracle: {1}{R}{R}, {T}: Creatures you control get +1/+0 until end of turn.
//! Set: TDC #347 — Tarkir: Dragonstorm Commander | Scryfall ID: 337f2d97-b317-4c10-b151-7acccf38fca8 | Oracle ID: 91fbb25b-8521-483f-88b0-77778d25f7fd
// IMPLEMENTED — checkland (enters tapped unless you control a Mountain); {T}: Add {R};
// {1}{R}{R}, {T}: all your creatures get +1/+0 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
]);

card! {
    index: 334,
    oracle_id: "91fbb25b-8521-483f-88b0-77778d25f7fd",
    scryfall_id: "337f2d97-b317-4c10-b151-7acccf38fca8",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Castle Embereth",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{1}{R}{R}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::PumpFilter {
                filter: &Filter::YOUR_CREATURE,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]
        ),
    ],
}
