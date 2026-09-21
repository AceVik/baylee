//! Tomb of Urami — (no cost) — Legendary Land
//! Oracle: {T}: Add {B}. Tomb of Urami deals 1 damage to you if you don't control an Ogre.
//! Oracle: {2}{B}{B}, {T}, Sacrifice all lands you control: Create Urami, a legendary 5/5 black Demon Spirit creature token with flying.
//! Set: SOK #165 — Saviors of Kamigawa | Scryfall ID: 90fedf90-825c-4814-8f0d-170f537db44c | Oracle ID: f002be6a-e459-49c4-b765-062e30107439
// IMPLEMENTED — {T}: Add {B}. Partial: the printed damage rider and the
// Urami ability have no DSL spelling; both are NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

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
        "the {T} ability's \"deals 1 damage to you if you don't control an Ogre\" rider needs a condition the DSL cannot state, and the Urami ability needs a sacrifice-all-lands cost and a legendary Demon Spirit token"
    ),
    // NOT SUPPORTED: "Tomb of Urami deals 1 damage to you if you don't control
    // an Ogre." — Condition carries ControlCount(filter, at_least) and no
    // "you control no …" complement, and none of the effect-level branches
    // (IfControlGreatestCmc, IfNoCountersOnSelf, …) asks that question either,
    // so the mana ability is written without its drawback.
    //
    // NOT SUPPORTED: "{2}{B}{B}, {T}, Sacrifice all lands you control: Create
    // Urami, a legendary 5/5 black Demon Spirit creature token with flying."
    // — no CostPart takes *all* permanents a filter matches: SacrificeSelf is
    // only the source and Sacrifice(filter) buys one permanent, chosen, so the
    // printed cost (which includes this land) is not sayable. The token is a
    // second gap on the same sentence: CreateToken names a &TokenDef from the
    // registry and no printing of Urami is in it.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);
