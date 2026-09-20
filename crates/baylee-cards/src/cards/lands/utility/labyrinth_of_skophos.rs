//! Labyrinth of Skophos — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Remove target attacking or blocking creature from combat.
//! Set: MKC #272 — Murders at Karlov Manor Commander | Scryfall ID: 388169df-4cce-452d-9215-e3f814ff4fdf | Oracle ID: 9ec5a487-d8ed-459a-8f58-56f6e9a2dfe8
// IMPLEMENTED — {T}: Add {C}; the remove-from-combat clause has no DSL variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LABYRINTH_OF_SKOPHOS,
    oracle_id = "9ec5a487-d8ed-459a-8f58-56f6e9a2dfe8",
    scryfall_id = "388169df-4cce-452d-9215-e3f814ff4fdf",
    faces = &[face!(name = "Labyrinth of Skophos", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("no Effect leaves combat: the {4}, {T} ability cannot be written"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{4}, {T}: Remove target attacking or blocking
        // creature from combat." — no `Effect` variant removes a permanent
        // from combat. `Effect::PhaseOut` is the nearest thing that stops
        // an attacker, and it is a different rule: it phases the creature
        // out whole (it comes back later, and it was never "removed from
        // combat"). The target is not sayable either — `Filter::Attacking`
        // exists, a blocking predicate does not, and `TargetSpec` has no
        // variant that spans the two.
    ],
);
