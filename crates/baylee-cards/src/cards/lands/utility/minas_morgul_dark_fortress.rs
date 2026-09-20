//! Minas Morgul, Dark Fortress — (no cost) — Legendary Land
//! Oracle: Minas Morgul enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {3}{B}, {T}: Put a shadow counter on target creature. For as long as that creature has a shadow counter on it, it's a Wraith in addition to its other types. (A creature with shadow can block or be blocked by only creatures with shadow.)
//! Set: LTC #514 — Tales of Middle-earth Commander | Scryfall ID: 50f76652-15e7-4190-842e-db6db6d66e91 | Oracle ID: 867dbd5a-c3cf-41ce-980b-c9babc6f30f2
// PARTIAL — enters tapped and {T}: Add {B}; the shadow-counter ability is
// not built (see the NOT SUPPORTED line below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MINAS_MORGUL_DARK_FORTRESS,
    oracle_id = "867dbd5a-c3cf-41ce-980b-c9babc6f30f2",
    scryfall_id = "50f76652-15e7-4190-842e-db6db6d66e91",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Minas Morgul, Dark Fortress",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the {3}{B}, {T} ability is not built: a shadow counter has no `counters::` id assigned in `baylee_cards_dsl::counters`, no `Modifier` adds a subtype while a counter is on the object (`AddTypeIfCountersAtLeast` takes a TypeSet, not a SubtypeId), and shadow is a keyword no engine reader reads"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        // NOT SUPPORTED: {3}{B}, {T}: Put a shadow counter on target creature. For as long as that creature has a shadow counter on it, it's a Wraith in addition to its other types.
    ],
);
