//! Hagra Mauling // Hagra Broodpit — {2}{B}{B} — Instant // Land
//! Oracle: This spell costs {1} less to cast if an opponent controls no basic lands.
//! Oracle: Destroy target creature.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #106 — Zendikar Rising | Scryfall ID: 7c04c734-354d-4925-8161-7052110951df | Oracle ID: 37783ce6-af58-4ef6-8ab4-587079970307
//! Face: Hagra Mauling — {2}{B}{B} — Instant
//! Face: Hagra Broodpit —  — Land

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static BROODPIT_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::HAGRA_MAULING,
    oracle_id = "37783ce6-af58-4ef6-8ab4-587079970307",
    scryfall_id = "7c04c734-354d-4925-8161-7052110951df",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Hagra Mauling",
            mana_cost = mana!("{2}{B}{B}"),
            types = TypeSet::INSTANT,
            cost_reduction = Some(CostReduction::If {
                condition: Condition::OpponentControlCountAtMost(&Filter::BASIC_LAND, 0),
                amount: 1,
            }),
        ),
        face!(
            name = "Hagra Broodpit",
            types = TypeSet::LAND,
            abilities = BROODPIT_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
