//! Power Armor — {4} — Artifact
//! Oracle: Domain — {3}, {T}: Target creature gets +1/+1 until end of turn for each basic land type among lands you control.
//! Set: DDE #62 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 20bc8710-3d56-45fb-81e7-383a274a2374 | Oracle ID: 1423b8e6-9165-4a89-a6ed-18085f460bca
// IMPLEMENTED — {3}, {T}: +1/+1 until end of turn for each basic land type
// among lands you control (CR 305.6). Instant speed is the rules default
// (CR 117.1b) and the printing adds no timing restriction.

use baylee_cards_dsl::prelude::*;

/// The domain count: basic land types (CR 305.6) among lands you control.
static DOMAIN: Amount = Amount::BasicLandTypesAmong(&Filter::YOUR_LAND);

card!(
    index = index::POWER_ARMOR,
    oracle_id = "1423b8e6-9165-4a89-a6ed-18085f460bca",
    scryfall_id = "20bc8710-3d56-45fb-81e7-383a274a2374",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Power Armor",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::PumpTarget {
            power: DOMAIN,
            toughness: DOMAIN,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )],
);
