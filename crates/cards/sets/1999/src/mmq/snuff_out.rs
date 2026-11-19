//! Snuff Out — {3}{B} — Instant
//! Oracle: If you control a Swamp, you may pay 4 life rather than pay this spell's mana cost.
//! Oracle: Destroy target nonblack creature. It can't be regenerated.
//! Set: GVL #53 — Duel Decks Anthology: Garruk vs. Liliana | Scryfall ID: 75bbe89f-09af-494e-b58e-271f64bde4b5 | Oracle ID: 324824cb-f938-401c-b9b5-d8908b431ef0
// IMPLEMENTED — Swamp-conditional 4-life alternative cost; destroy target nonblack creature without regeneration.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SNUFF_OUT,
    oracle_id = "324824cb-f938-401c-b9b5-d8908b431ef0",
    scryfall_id = "75bbe89f-09af-494e-b58e-271f64bde4b5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Snuff Out",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(PayLife(4)),
            condition: AltCondition::If(&Condition::ControlCount(
                &Filter::HasSubtype(subtypes::land::SWAMP),
                1,
            )),
        }],
    ),],
    abilities = &[spell!(
        &[Effect::destroy_no_regen(TargetSpec::Object(&Filter::And(
            &[
                Filter::CREATURE,
                Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
            ]
        )))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
        ]))))
    ),],
);
