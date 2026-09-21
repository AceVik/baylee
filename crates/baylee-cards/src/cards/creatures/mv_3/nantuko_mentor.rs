//! Nantuko Mentor — {2}{G} — Creature — Insect Druid
//! Oracle: {2}{G}, {T}: Target creature gets +X/+X until end of turn, where X is that creature's power.
//! Set: ODY #255 — Odyssey | Scryfall ID: 48cb5283-d384-490e-a0a5-2d10c1acc8cc | Oracle ID: b79378e7-99db-403f-8f63-4d71ebdb3f6c
// IMPLEMENTED — {2}{G}, {T}: target creature gets +X/+X until end of turn,
// where X is that creature's power (Amount::TargetPower).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NANTUKO_MENTOR,
    oracle_id = "b79378e7-99db-403f-8f63-4d71ebdb3f6c",
    scryfall_id = "48cb5283-d384-490e-a0a5-2d10c1acc8cc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nantuko Mentor",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{2}{G}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::TargetPower,
            toughness: Amount::TargetPower,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
