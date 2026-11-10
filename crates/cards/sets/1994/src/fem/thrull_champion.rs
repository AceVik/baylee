//! Thrull Champion — {4}{B} — Creature — Thrull
//! Oracle: Thrull creatures get +1/+1.
//! Oracle: {T}: Gain control of target Thrull for as long as you control this creature.
//! Set: ME1 #83 — Masters Edition | Scryfall ID: 1c77c217-088c-41ab-b56b-2cb703d06991 | Oracle ID: 728f0a3f-722d-4936-ade8-31883499f56d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const THRULL: Filter = Filter::HasSubtype(subtypes::creature::THRULL);
const THRULL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::THRULL),
]);

card!(
    index = index::THRULL_CHAMPION,
    oracle_id = "728f0a3f-722d-4936-ade8-31883499f56d",
    scryfall_id = "1c77c217-088c-41ab-b56b-2cb703d06991",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thrull Champion",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(THRULL_CREATURE, Modifier::ModifyPT(1, 1)),
        activated!(
            Cost::TAP,
            &[Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::WhileYouControlSource,
            }],
            target = Some(TargetSpec::Object(&THRULL)),
        ),
    ],
);
