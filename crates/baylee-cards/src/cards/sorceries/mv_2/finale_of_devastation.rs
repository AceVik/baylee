//! Finale of Devastation — {X}{G}{G} — Sorcery
//! Oracle: Search your library and/or graveyard for a creature card with mana value X or less and put it onto the battlefield. If you search your library this way, shuffle. If X is 10 or more, creatures you control get +X/+X and gain haste until end of turn.
//! Set: CMM #289 — Commander Masters | Scryfall ID: b10d99bf-b2ce-4443-b924-ff0eb8be1033 | Oracle ID: 69872a9a-fe54-4e58-940c-89395af71acd
// PARTIAL — the library half of the search, as one find straight onto the
// battlefield; the graveyard half and the "X is 10 or more" pump are
// NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FINALE_OF_DEVASTATION,
    oracle_id = "69872a9a-fe54-4e58-940c-89395af71acd",
    scryfall_id = "b10d99bf-b2ce-4443-b924-ff0eb8be1033",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Finale of Devastation",
        mana_cost = mana!("{X}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "library-only search: no graveyard zone, and no branch on the announced X",
    ),
    abilities = &[spell!(&[Effect::SearchLibrary {
        // "a creature card with mana value X or less" — Filter::CmcAtMostX
        // reads the announced X off the spell, so the bound is not a constant.
        filter: &Filter::And(&[Filter::CREATURE, Filter::CmcAtMostX]),
        finds: &[Find::BATTLEFIELD],
        optional: false,
    }])],
);

// NOT SUPPORTED: "Search your library and/or graveyard for a creature card
// with mana value X or less and put it onto the battlefield." — the
// graveyard half. Effect::SearchLibrary searches the library and carries no
// zone, and the printed card is *one* find chosen from either zone;
// Effect::GraveyardToBattlefield is a target picked as the spell is cast and
// not a search, so a Sequence of the two would return a second card instead
// of the same one. ("If you search your library this way, shuffle" is
// derived: every printed search shuffles.)
// NOT SUPPORTED: "If X is 10 or more, creatures you control get +X/+X and
// gain haste until end of turn." — no effect branches on the announced X
// (IfKicked and IfEventPowerAtLeast read something else), so the pump would
// either fire at every X or at none of them.
