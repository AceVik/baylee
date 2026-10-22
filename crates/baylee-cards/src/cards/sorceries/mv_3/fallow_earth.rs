//! Fallow Earth — {2}{G} — Sorcery
//! Oracle: Put target land on top of its owner's library.
//! Set: 6ED #227 — Classic Sixth Edition | Scryfall ID: 44b02b12-415c-49f4-9f60-fa53beb4216f | Oracle ID: 05a4acfd-3bf3-4ec2-a28a-bc43ba7c6fbf
// PARTIAL — putting a target land unconditionally on top of its owner's library from the battlefield is not yet a dedicated Effect variant; uses OwnerPutsOnTopOrBottom.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALLOW_EARTH,
    oracle_id = "05a4acfd-3bf3-4ec2-a28a-bc43ba7c6fbf",
    scryfall_id = "44b02b12-415c-49f4-9f60-fa53beb4216f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "putting target land unconditionally on top of its owner's library from the battlefield is not yet a dedicated Effect variant; uses OwnerPutsOnTopOrBottom so its owner puts it on top or bottom of their library",
    ),
    faces = &[face!(
        name = "Fallow Earth",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::OwnerPutsOnTopOrBottom {
            target: TargetSpec::Object(&Filter::LAND),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    )],
);
