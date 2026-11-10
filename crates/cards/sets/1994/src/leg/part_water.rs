//! Part Water — {X}{X}{U} — Sorcery
//! Oracle: X target creatures gain islandwalk until end of turn. (They can't be blocked as long as defending player controls an Island.)
//! Set: LEG #66 — Legends | Scryfall ID: 4b659475-c8b7-493d-af63-04f34d8cc3b1 | Oracle ID: 8d51d448-14a6-4125-aa80-159ed6afbd03
// IMPLEMENTED — X target creatures gain islandwalk until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PART_WATER,
    oracle_id = "8d51d448-14a6-4125-aa80-159ed6afbd03",
    scryfall_id = "4b659475-c8b7-493d-af63-04f34d8cc3b1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Part Water",
        mana_cost = mana!("{X}{X}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::ISLANDWALK,
            duration: Duration::UntilEndOfTurn
        }],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
