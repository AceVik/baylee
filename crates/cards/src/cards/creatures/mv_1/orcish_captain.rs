//! Orcish Captain — {R} — Creature — Orc Warrior
//! Oracle: {1}: Flip a coin. If you win the flip, target Orc creature gets +2/+0 until end of turn. If you lose the flip, it gets -0/-2 until end of turn.
//! Set: ME2 #139 — Masters Edition II | Scryfall ID: 95f3eb71-f158-44d8-ae22-bdef48e2f13b | Oracle ID: 76c06de7-39ff-4b80-8e54-20910372cffd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ORC_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::ORC),
]);

card!(
    index = index::ORCISH_CAPTAIN,
    oracle_id = "76c06de7-39ff-4b80-8e54-20910372cffd",
    scryfall_id = "95f3eb71-f158-44d8-ae22-bdef48e2f13b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orcish Captain",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC, subtypes::creature::WARRIOR],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}"),
        &[Effect::FlipCoin {
            won: &[Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(2, 0),
                duration: Duration::UntilEndOfTurn,
            }],
            lost: &[Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(0, -2),
                duration: Duration::UntilEndOfTurn,
            }],
        }],
        target = Some(TargetSpec::Object(&ORC_CREATURE)),
    ),],
);
