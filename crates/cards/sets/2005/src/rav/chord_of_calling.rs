//! Chord of Calling — {X}{G}{G}{G} — Instant
//! Oracle: Convoke (Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.)
//! Oracle: Search your library for a creature card with mana value X or less, put it onto the battlefield, then shuffle.
//! Set: RVR #134 — Ravnica Remastered | Scryfall ID: b18fe7e0-8344-40cc-b242-83f01c6be7a6 | Oracle ID: 6789a170-f2c5-4fc0-8a45-2b2361e67410
// IMPLEMENTED — convoke is a face flag (CR 702.51) and the search is bounded
// by the announced X.

use baylee_cards_dsl::prelude::*;

/// "A creature card with mana value X or less" — X being the value announced
/// for this spell, which is why the bound is `CmcAtMostX` and not a number.
static CREATURE_WITHIN_X: Filter = Filter::And(&[Filter::CREATURE, Filter::CmcAtMostX]);

card!(
    index = index::CHORD_OF_CALLING,
    oracle_id = "6789a170-f2c5-4fc0-8a45-2b2361e67410",
    scryfall_id = "b18fe7e0-8344-40cc-b242-83f01c6be7a6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chord of Calling",
        mana_cost = mana!("{X}{G}{G}{G}"),
        types = TypeSet::INSTANT,
        convoke = true,
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &CREATURE_WITHIN_X,
        finds: &[Find::BATTLEFIELD],
        optional: false,
    }])],
);
