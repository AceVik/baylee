//! Radiant Essence — {1}{G}{W} — Creature — Spirit
//! Oracle: This creature gets +1/+2 as long as an opponent controls a black permanent.
//! Set: MIR #276 — Mirage | Scryfall ID: d9b1853c-d5b9-4361-8a2f-36946a62847b | Oracle ID: 48e2f2c3-9d99-4535-915f-551fbb59ade5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_BLACK_PERMANENT: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::RADIANT_ESSENCE,
    oracle_id = "48e2f2c3-9d99-4535-915f-551fbb59ade5",
    scryfall_id = "d9b1853c-d5b9-4361-8a2f-36946a62847b",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Radiant Essence",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ModifyPT(1, 2),
        condition = Some(Condition::OpponentControlCount(
            &OPPONENT_BLACK_PERMANENT,
            1,
        )),
    )],
);
