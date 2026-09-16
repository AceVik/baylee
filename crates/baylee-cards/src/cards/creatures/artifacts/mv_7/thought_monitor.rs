//! Thought Monitor — {6}{U} — Artifact Creature — Construct
//! Oracle: Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)
//! Oracle: Flying
//! Oracle: When this creature enters, draw two cards.
//! Set: EOC #79 — Edge of Eternities Commander | Scryfall ID: 18a6ea89-417c-4ee0-a410-8a0067b92967 | Oracle ID: 9deded8b-cec4-4ede-a50b-131404d456d4
// PARTIAL — a 2/2 flier that draws its controller two cards as it enters;
// the printed affinity never comes off the cost, so the spell is always
// paid at {6}{U}.
// NOT SUPPORTED: `Affinity for artifacts` — "costs {1} less to cast for each
// artifact you control". A printed reduction is `FaceDef::cost_reduction`,
// whose one variant is `CostReduction::NotStartingPlayer(n)`: a flat {n} on a
// yes/no condition, with nothing that counts permanents on the battlefield.
// The card therefore plays exactly as though the line were not printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THOUGHT_MONITOR,
    oracle_id = "9deded8b-cec4-4ede-a50b-131404d456d4",
    scryfall_id = "18a6ea89-417c-4ee0-a410-8a0067b92967",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thought Monitor",
        mana_cost = mana!("{6}{U}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(2),
        toughness = Some(2),
    ),],
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial("affinity for artifacts does not reduce the cost"),
    abilities = &[triggered!(Trigger::ETB, &[Effect::draw(2)])],
);

// Engine-level coverage: the ETB half is the shape Mulldrifter already plays
// in `s7_tests::mulldrifter_evoke_draws_then_sacrifices` — it enters, the
// trigger goes on the stack, its controller draws two. Affinity is untestable
// here by construction: nothing reads a reduction that counts artifacts.
