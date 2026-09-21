//! Oakhollow Village — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add {G}. Spend this mana only to cast a creature spell.
//! Oracle: {G}, {T}: Put a +1/+1 counter on each Frog, Rabbit, Raccoon, or Squirrel you control that entered the battlefield this turn.
//! Set: BLB #258 — Bloomburrow | Scryfall ID: 0d49b016-b02b-459f-85e9-c04f6bdcb94e | Oracle ID: 177b7fe4-8565-4631-b4d9-8b2b4282f3ac
// PARTIAL — both mana abilities ({C}, and {G} restricted to creature spells);
// the {G},{T} counter ability needs a filter the DSL does not have.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OAKHOLLOW_VILLAGE,
    oracle_id = "177b7fe4-8565-4631-b4d9-8b2b4282f3ac",
    scryfall_id = "0d49b016-b02b-459f-85e9-c04f6bdcb94e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(name = "Oakhollow Village", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "{G}, {T}: Put a +1/+1 counter on each Frog, Rabbit, Raccoon, or Squirrel \
         you control that entered the battlefield this turn — no Filter variant \
         says \"entered the battlefield this turn\"",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana(ManaColor::Green, 1).restricted(&Filter::CREATURE, SpendRider::None)]
        ),
        // NOT SUPPORTED: "{G}, {T}: Put a +1/+1 counter on each Frog, Rabbit,
        // Raccoon, or Squirrel you control that entered the battlefield this
        // turn." — Effect::AddCounterFilter carries the tribe, but the
        // smallest filter the DSL has for it is the four subtypes plus
        // ControlledByYou, which would also hit every Frog/Rabbit/Raccoon/
        // Squirrel already on the battlefield. "Entered the battlefield this
        // turn" is not a Filter predicate, so the ability comes off rather
        // than countering the wrong permanents.
    ],
);
