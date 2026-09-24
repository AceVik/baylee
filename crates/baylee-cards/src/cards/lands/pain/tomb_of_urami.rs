//! Tomb of Urami — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}. Tomb of Urami deals 1 damage to you if you don't control an Ogre.
//! Oracle: {2}{B}{B}, {T}, Sacrifice all lands you control: Create Urami, a legendary 5/5 black Demon Spirit creature token with flying.
//! Set: SOK #165 — Saviors of Kamigawa | Scryfall ID: 90fedf90-825c-4814-8f0d-170f537db44c | Oracle ID: f002be6a-e459-49c4-b765-062e30107439
// PARTIAL — {T}: Add {B} with its Ogre-less damage rider. The Urami ability
// has no DSL spelling; it is NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TOMB_OF_URAMI,
    oracle_id = "f002be6a-e459-49c4-b765-062e30107439",
    scryfall_id = "90fedf90-825c-4814-8f0d-170f537db44c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tomb of Urami",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the Urami ability needs a \"sacrifice all lands you control\" cost, and no CostPart sacrifices every permanent a filter matches (Sacrifice(filter) buys one)"
    ),
    // NOT SUPPORTED: "{2}{B}{B}, {T}, Sacrifice all lands you control: Create
    // Urami, a legendary 5/5 black Demon Spirit creature token with flying."
    // — no CostPart takes *all* permanents a filter matches: SacrificeSelf is
    // only the source and Sacrifice(filter) buys one permanent, chosen, so the
    // printed cost (which includes this land) is not sayable. The token
    // exists (URAMI_LEGENDARY_5_5_BLACK_FLYING); the cost is the whole gap.
    abilities = &[mana_ability!(&[
        Effect::mana(ManaColor::Black, 1),
        Effect::IfCondition {
            condition: Condition::ControlCountAtMost(
                &Filter::HasSubtype(subtypes::creature::OGRE),
                0,
            ),
            then: &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            }],
            otherwise: &[],
        },
    ])],
);
