//! Pestilence Rats — {2}{B} — Creature — Rat
//! Oracle: Pestilence Rats's power is equal to the number of other Rats on the battlefield. (For example, as long as there are two other Rats on the battlefield, Pestilence Rats's power and toughness are 2/3.)
//! Set: ICE #157 — Ice Age | Scryfall ID: bff7f6a6-0e90-4eb4-b76e-d98454975fb6 | Oracle ID: bc6449c8-f0cd-4c07-8acf-436f97e8bbb2
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_RAT: Filter =
    Filter::And(&[Filter::Another, Filter::HasSubtype(subtypes::creature::RAT)]);

card!(
    index = index::PESTILENCE_RATS,
    oracle_id = "bc6449c8-f0cd-4c07-8acf-436f97e8bbb2",
    scryfall_id = "bff7f6a6-0e90-4eb4-b76e-d98454975fb6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pestilence Rats",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(0),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::OnBattlefield(&OTHER_RAT),
            toughness: PtCount::Fixed(3),
        }
    )],
);
