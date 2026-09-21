//! Thawing Glaciers — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {1}, {T}: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle. Return this land to its owner's hand at the beginning of the next cleanup step.
//! Set: VMA #318 — Vintage Masters | Scryfall ID: 397facec-f473-45a5-a4ce-02cb56e7bfab | Oracle ID: c6792d9f-8b74-43c4-814f-ba4adab2fdea
// IMPLEMENTED — enters tapped, and {1}, {T} searches up a basic land onto the
// battlefield tapped (the shuffle is derived from the search, not declared).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THAWING_GLACIERS,
    oracle_id = "c6792d9f-8b74-43c4-814f-ba4adab2fdea",
    scryfall_id = "397facec-f473-45a5-a4ce-02cb56e7bfab",
    faces = &[face!(
        name = "Thawing Glaciers",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "\"Return this land to its owner's hand at the beginning of the next \
         cleanup step\" is not expressible: no effect registers a delayed \
         trigger, StepKind has no cleanup step, and a StepBegin trigger would \
         fire on every one of them instead of once"
    ),
    abilities = &[
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            }]
        ),
        // NOT SUPPORTED: "Return this land to its owner's hand at the
        // beginning of the next cleanup step" — the delayed trigger that
        // would carry it does not exist (DelayedManaAtNextFirstMain and
        // PayCostOrLoseLater are the only two delayed effects there are).
    ],
);
