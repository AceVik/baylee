//! Escape Tunnel — (no cost) — Land
//! Oracle: {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Oracle: {T}, Sacrifice this land: Target creature with power 2 or less can't be blocked this turn.
//! Set: TMT #184 — Teenage Mutant Ninja Turtles | Scryfall ID: 5df90940-15ea-418c-8547-6c75d69ec6d3 | Oracle ID: 0056fc91-4398-471c-b561-7ff99750ac8a
// IMPLEMENTED — both halves: the fetch, and the evasion behind the power
// restriction its target prints.

use baylee_cards_dsl::prelude::*;

/// Two or less, not three: the neighbouring land in this pool (Access
/// Tunnel) prints the same sentence one point wider, which is the whole
/// difference between the two cards and the reason the bound is a parameter
/// rather than a named predicate.
static SMALL_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::PowerAtMost(2)]);

card!(
    index = index::ESCAPE_TUNNEL,
    oracle_id = "0056fc91-4398-471c-b561-7ff99750ac8a",
    scryfall_id = "5df90940-15ea-418c-8547-6c75d69ec6d3",
    faces = &[face!(name = "Escape Tunnel", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            }]
        ),
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::UNBLOCKABLE,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&SMALL_CREATURE)),
        ),
    ],
);
