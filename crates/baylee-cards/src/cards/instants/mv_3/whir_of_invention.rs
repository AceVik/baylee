//! Whir of Invention — {X}{U}{U}{U} — Instant
//! Oracle: Improvise (Your artifacts can help cast this spell. Each artifact you tap after you're done activating mana abilities pays for {1}.)
//! Oracle: Search your library for an artifact card with mana value X or less, put it onto the battlefield, then shuffle.
//! Set: AER #49 — Aether Revolt | Scryfall ID: 0279fd3c-9252-4958-9d7a-5f33aa25907e | Oracle ID: 152b91c9-cc07-4ca8-944f-9bc2242a2283
// PARTIAL — the tutor is built (an artifact card with mana value X or less, via CmcAtMostX off the announced X); improvise is not expressible.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Improvise (Your artifacts can help cast this spell. Each artifact you tap after you're done activating mana abilities pays for {1}.) — no CostPart or AlternativeCost taps an artifact to pay a spell's generic mana, and improvise is not one of the keyword bits the engine reads.

card!(
    index = index::WHIR_OF_INVENTION,
    oracle_id = "152b91c9-cc07-4ca8-944f-9bc2242a2283",
    scryfall_id = "0279fd3c-9252-4958-9d7a-5f33aa25907e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Whir of Invention",
        mana_cost = mana!("{X}{U}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "improvise: nothing in the DSL taps an artifact to pay {1} of a spell's cost"
    ),
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::And(&[Filter::ARTIFACT, Filter::CmcAtMostX]),
        finds: &[Find::BATTLEFIELD],
        optional: false,
    }])],
);
