//! Oakhollow Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {G}. Spend this mana only to cast a creature spell.
//! Oracle: {G}, {T}: Put a +1/+1 counter on each Frog, Rabbit, Raccoon, or Squirrel you control that entered the battlefield this turn.
//! Set: BLB #258 — Bloomburrow | Scryfall ID: 0d49b016-b02b-459f-85e9-c04f6bdcb94e | Oracle ID: 177b7fe4-8565-4631-b4d9-8b2b4282f3ac

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "each Frog, Rabbit, Raccoon, or Squirrel you control that entered the
/// battlefield this turn" — the tribe, the controller, and this turn's
/// arrivals, so the ones already on the battlefield are left alone.
static NEW_ARRIVALS: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(creature::FROG),
        Filter::HasSubtype(creature::RABBIT),
        Filter::HasSubtype(creature::RACCOON),
        Filter::HasSubtype(creature::SQUIRREL),
    ]),
    Filter::ControlledByYou,
    Filter::EnteredThisTurn,
]);

card!(
    index = index::OAKHOLLOW_VILLAGE,
    oracle_id = "177b7fe4-8565-4631-b4d9-8b2b4282f3ac",
    scryfall_id = "0d49b016-b02b-459f-85e9-c04f6bdcb94e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(name = "Oakhollow Village", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana(ManaColor::Green, 1).restricted(&Filter::CREATURE, SpendRider::None)]
        ),
        activated!(
            cost!("{G}", TapSelf),
            &[Effect::AddCounterFilter {
                filter: &NEW_ARRIVALS,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
    ],
);
