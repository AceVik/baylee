//! Word of Blasting — {1}{R} — Instant
//! Oracle: Destroy target Wall. It can't be regenerated. Word of Blasting deals damage equal to that Wall's mana value to the Wall's controller.
//! Set: MMQ #228 — Mercadian Masques | Scryfall ID: c5362ead-9162-4160-bfa9-432f7d0e222d | Oracle ID: 8b391f24-992f-4cc0-ac4d-31456983fc03

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WALL: Filter = Filter::HasSubtype(subtypes::creature::WALL);

card!(
    index = index::WORD_OF_BLASTING,
    oracle_id = "8b391f24-992f-4cc0-ac4d-31456983fc03",
    scryfall_id = "c5362ead-9162-4160-bfa9-432f7d0e222d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Word of Blasting",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy_no_regen(TargetSpec::Object(&WALL)),
            Effect::DealDamageToTargetController {
                amount: Amount::TargetCmc,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&WALL)))
    )],
);
