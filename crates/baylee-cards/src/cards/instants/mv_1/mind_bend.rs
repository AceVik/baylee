//! Mind Bend — {U} — Instant
//! Oracle: Change the text of target permanent by replacing all instances of one color word with another or one basic land type with another. (For example, you may change "nonblack creature" to "nongreen creature" or "forestwalk" to "islandwalk." This effect lasts indefinitely.)
//! Set: 10E #93 — Tenth Edition | Scryfall ID: 692a5bfa-9a50-4610-acd7-19fc0a53f798 | Oracle ID: d8377869-f8f7-4b4b-93ad-3ad3c4826b1a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_BEND,
    oracle_id = "d8377869-f8f7-4b4b-93ad-3ad3c4826b1a",
    scryfall_id = "692a5bfa-9a50-4610-acd7-19fc0a53f798",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mind Bend",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ChooseOne {
            choices: &[
                Effect::ChangeTextWord {
                    kind: TextWordKind::Color,
                },
                Effect::ChangeTextWord {
                    kind: TextWordKind::BasicLandType,
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    )],
);
