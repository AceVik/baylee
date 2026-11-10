//! Fire Covenant — {1}{B}{R} — Instant
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: Fire Covenant deals X damage divided as you choose among any number of target creatures.
//! Set: ECC #54 — Lorwyn Eclipsed Commander | Scryfall ID: 1a13437b-e0c7-4ca9-a0e2-b8cb1b5813ec | Oracle ID: 025939a0-424a-41bf-8fc9-2ef9ea5485f7
// IMPLEMENTED — life paid at casting, then the announced X is divided among creatures.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRE_COVENANT,
    oracle_id = "025939a0-424a-41bf-8fc9-2ef9ea5485f7",
    scryfall_id = "1a13437b-e0c7-4ca9-a0e2-b8cb1b5813ec",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fire Covenant",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::PayLifeX],
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageDivided {
            amount: DivisionAmount::XDoubleFrom(u32::MAX),
        }],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::CREATURE),
            u8::MAX,
        )),
    )],
);
