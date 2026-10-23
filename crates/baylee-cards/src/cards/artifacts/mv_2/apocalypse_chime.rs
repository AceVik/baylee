//! Apocalypse Chime — {2} — Artifact
//! Oracle: {2}, {T}, Sacrifice this artifact: Destroy all nontoken permanents with a name originally printed in the Homelands expansion. They can't be regenerated.
//! Set: HML #101 — Homelands | Scryfall ID: cef20d8f-6e80-4fca-b6a7-541981f6a112 | Oracle ID: 995e2412-3f69-4aaf-a5d6-ae0ecbb95061
// PARTIAL — Filter::NameOriginallyPrintedIn takes an OriginalExpansion that has only ArabianNights and Antiquities (no Homelands), so the destruction is not said; the {2}, {T}, sacrifice ability is paid and resolves with no effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::APOCALYPSE_CHIME,
    oracle_id = "995e2412-3f69-4aaf-a5d6-ae0ecbb95061",
    scryfall_id = "cef20d8f-6e80-4fca-b6a7-541981f6a112",
    coverage = Coverage::Partial(
        "OriginalExpansion has no Homelands for Filter::NameOriginallyPrintedIn, so \"destroy all nontoken permanents with a name originally printed in the Homelands expansion\" is not said; the {2}, {T}, sacrifice ability is paid and resolves with no effect"
    ),
    faces = &[face!(
        name = "Apocalypse Chime",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(cost!("{2}", TapSelf, SacrificeSelf), &[])],
);
