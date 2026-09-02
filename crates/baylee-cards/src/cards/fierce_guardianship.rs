//! Fierce Guardianship — {2}{U} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: Counter target noncreature spell.
//! Set: CMM #94 — Commander Masters | Scryfall ID: f7f3dd95-bd14-4e0f-a388-444f9cf1b0dc | Oracle ID: d09c9cba-fdd2-479b-ad5d-d05181c3e3f9
// IMPLEMENTED — commander-conditional free cast + noncreature counter.

static NONCREATURE_SPELL: Filter = Filter::Not(&Filter::CREATURE);

use baylee_cards_dsl::prelude::*;

card! {
    index: 50,
    oracle_id: "d09c9cba-fdd2-479b-ad5d-d05181c3e3f9",
    scryfall_id: "f7f3dd95-bd14-4e0f-a388-444f9cf1b0dc",
    faces: &[face! {
        name: "Fierce Guardianship",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::INSTANT,
        alternative_costs: &[AlternativeCost {
            cost: Cost::FREE,
            condition: AltCondition::CommanderControlled,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CounterTargetSpell], targets: Some(TargetReq::one(TargetSpec::Spell(&NONCREATURE_SPELL))))],
}
