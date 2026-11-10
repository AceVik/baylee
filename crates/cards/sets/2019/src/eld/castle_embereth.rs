//! Castle Embereth — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Mountain.
//! Oracle: {T}: Add {R}.
//! Oracle: {1}{R}{R}, {T}: Creatures you control get +1/+0 until end of turn.
//! Set: TDC #347 — Tarkir: Dragonstorm Commander | Scryfall ID: 337f2d97-b317-4c10-b151-7acccf38fca8 | Oracle ID: 91fbb25b-8521-483f-88b0-77778d25f7fd
// IMPLEMENTED — enters tapped unless you control a Mountain, taps for {R},
// and pays {1}{R}{R} plus itself to give your creatures +1/+0 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static MOUNTAIN_YOU_CONTROL: Filter = f!(your Filter::HasSubtype(land::MOUNTAIN));

card!(
    index = index::CASTLE_EMBERETH,
    oracle_id = "91fbb25b-8521-483f-88b0-77778d25f7fd",
    scryfall_id = "337f2d97-b317-4c10-b151-7acccf38fca8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Castle Embereth",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&MOUNTAIN_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
        activated!(
            cost!("{1}{R}{R}", TapSelf),
            &[Effect::PumpFilter {
                filter: &Filter::YOUR_CREATURE,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
