//! Coral Reef — {U}{U} — Enchantment
//! Oracle: This enchantment enters with four polyp counters on it.
//! Oracle: Sacrifice an Island: Put two polyp counters on this enchantment.
//! Oracle: {U}, Tap an untapped blue creature you control, Remove a polyp counter from this enchantment: Put a +0/+1 counter on target creature.
//! Set: HML #24 — Homelands | Scryfall ID: 42fe2280-a996-4072-b5bf-f4fd56607a51 | Oracle ID: 37a40a53-2668-4521-a786-c5747eedeeb2
// PARTIAL — polyp counter type is not yet in the CounterKind::Custom registry; sacrifice an Island ability resolves with no effect, and {U}, tap an untapped blue creature you control ability puts a +0/+1 counter on target creature without removing a polyp counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
static BLUE_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::CORAL_REEF,
    oracle_id = "37a40a53-2668-4521-a786-c5747eedeeb2",
    scryfall_id = "42fe2280-a996-4072-b5bf-f4fd56607a51",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "polyp counter type not yet in CounterKind::Custom registry; sacrifice an Island ability resolves with no effect, and {U}, tap an untapped blue creature you control ability puts a +0/+1 counter on target creature without removing a polyp counter"
    ),
    faces = &[face!(
        name = "Coral Reef",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        activated!(cost!(Sacrifice(&ISLAND)), &[]),
        activated!(
            cost!("{U}", TapOther(&BLUE_CREATURE_YOU_CONTROL)),
            &[Effect::AddCounter {
                kind: CounterKind::Plus {
                    power: 0,
                    toughness: 1,
                },
                amount: Amount::Fixed(1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ],
);
