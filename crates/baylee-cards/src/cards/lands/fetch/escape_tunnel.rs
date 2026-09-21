//! Escape Tunnel — (no cost) — Land
//! Oracle: {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Oracle: {T}, Sacrifice this land: Target creature with power 2 or less can't be blocked this turn.
//! Set: TMT #184 — Teenage Mutant Ninja Turtles | Scryfall ID: 5df90940-15ea-418c-8547-6c75d69ec6d3 | Oracle ID: 0056fc91-4398-471c-b561-7ff99750ac8a
// PARTIAL — the fetch half is built; the second ability's target needs a
// power comparison the DSL does not have, so that half is dropped below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ESCAPE_TUNNEL,
    oracle_id = "0056fc91-4398-471c-b561-7ff99750ac8a",
    scryfall_id = "5df90940-15ea-418c-8547-6c75d69ec6d3",
    faces = &[face!(name = "Escape Tunnel", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the second ability targets a creature with power 2 or less, and Filter \
         has no power comparison (`ToughnessAtMost` exists, its power twin does not)"
    ),
    abilities = &[
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            }]
        ),
        // NOT SUPPORTED: "{T}, Sacrifice this land: Target creature with
        // power 2 or less can't be blocked this turn." — the target filter is
        // the blocker (`Filter` can compare toughness and mana value, not
        // power), so the ability comes off the card rather than being
        // approximated with a wider filter. The "can't be blocked" half alone
        // would be `PumpTarget` with `KeywordSet::UNBLOCKABLE` and
        // `Duration::UntilEndOfTurn`.
    ],
);
