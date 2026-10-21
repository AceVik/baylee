//! Faerie Noble — {2}{G} — Creature — Faerie Noble
//! Oracle: Flying
//! Oracle: Other Faerie creatures you control get +0/+1.
//! Oracle: {T}: Other Faerie creatures you control get +1/+0 until end of turn.
//! Set: ME3 #117 — Masters Edition III | Scryfall ID: 4d0687f4-f2d0-4915-a797-3797377195b7 | Oracle ID: 6172e5b9-2a01-429c-a140-e9b22fbab56e
// IMPLEMENTED — flying, other Faerie creatures you control get +0/+1, and {T} to give other Faerie creatures you control +1/+0 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_FAERIES_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Another,
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::creature::FAERIE),
]);

card!(
    index = index::FAERIE_NOBLE,
    oracle_id = "6172e5b9-2a01-429c-a140-e9b22fbab56e",
    scryfall_id = "4d0687f4-f2d0-4915-a797-3797377195b7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Faerie Noble",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE, subtypes::creature::NOBLE],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(OTHER_FAERIES_YOU_CONTROL, Modifier::ModifyPT(0, 1)),
        activated!(
            cost!(TapSelf),
            &[Effect::PumpFilter {
                filter: &OTHER_FAERIES_YOU_CONTROL,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }]
        ),
    ],
);
