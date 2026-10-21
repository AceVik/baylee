//! Hazduhr the Abbot — {3}{W}{W} — Legendary Creature — Human Cleric
//! Oracle: {X}, {T}: The next X damage that would be dealt this turn to target white creature you control is dealt to Hazduhr instead.
//! Set: HML #8 — Homelands | Scryfall ID: adfd416a-dddf-40e4-acf0-84057edb7a58 | Oracle ID: 35844d8b-8f68-4248-b921-e766757a9e26
// PARTIAL — redirecting the prevented X damage to Hazduhr the Abbot itself is not in Effect; {X}, {T} prevents the next X damage that would be dealt this turn to target white creature you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_WHITE_CREATURE: Filter = Filter::And(&[
    Filter::YOUR_CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::HAZDUHR_THE_ABBOT,
    oracle_id = "35844d8b-8f68-4248-b921-e766757a9e26",
    scryfall_id = "adfd416a-dddf-40e4-acf0-84057edb7a58",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "redirecting the prevented X damage to Hazduhr the Abbot itself is not in Effect; {X}, {T} prevents the next X damage that would be dealt this turn to target white creature you control"
    ),
    faces = &[face!(
        name = "Hazduhr the Abbot",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(5),
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&YOUR_WHITE_CREATURE),
            amount: Amount::X,
        }],
        target = Some(TargetSpec::Object(&YOUR_WHITE_CREATURE)),
    )],
);
