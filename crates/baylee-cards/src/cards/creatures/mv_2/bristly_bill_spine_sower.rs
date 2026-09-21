//! Bristly Bill, Spine Sower — {1}{G} — Legendary Creature — Plant Druid
//! Oracle: Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature.
//! Oracle: {3}{G}{G}: Double the number of +1/+1 counters on each creature you control.
//! Set: OTJ #157 — Outlaws of Thunder Junction | Scryfall ID: 52eef0d6-24b7-40b7-8403-e8e863d0cd55 | Oracle ID: d3b2d8a2-d3bc-448c-9cf6-6bead6010c28
// PARTIAL — the landfall trigger is an ordinary enter-trigger with a +1/+1
// counter on a target creature; the doubling activation has no variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRISTLY_BILL_SPINE_SOWER,
    oracle_id = "d3b2d8a2-d3bc-448c-9cf6-6bead6010c28",
    scryfall_id = "52eef0d6-24b7-40b7-8403-e8e863d0cd55",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the {3}{G}{G} activated ability (double the +1/+1 counters on each creature you control) is missing"
    ),
    faces = &[face!(
        name = "Bristly Bill, Spine Sower",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::PLANT, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::YOUR_LAND),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
    // NOT SUPPORTED: "{3}{G}{G}: Double the number of +1/+1 counters on each
    // creature you control." — no `Effect` and no `Modifier` doubles counters
    // already on a permanent. `ReplacementRule::DoubleCounterPlacement` is
    // the other sentence (it doubles what a later effect would *place*), and
    // `Modifier::ModifyPTPerCount` changes size rather than counters.
);
