//! Access Tunnel — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Target creature with power 3 or less can't be blocked this turn.
//! Set: TDC #337 — Tarkir: Dragonstorm Commander | Scryfall ID: 4bef5957-71a4-4fe0-b2ce-dff8e8690bd9 | Oracle ID: ed9cc560-f30b-4b60-a094-ccf93ed656a7
// PARTIAL — {T}: Add {C} is built. The {3} activation would have to target
// any creature, because `Filter` cannot compare power; it stays off the card
// and is named in the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ACCESS_TUNNEL,
    oracle_id = "ed9cc560-f30b-4b60-a094-ccf93ed656a7",
    scryfall_id = "4bef5957-71a4-4fe0-b2ce-dff8e8690bd9",
    faces = &[face!(name = "Access Tunnel", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "no Filter variant compares power, so \"target creature with power 3 or less\" has no target spec",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {3}, {T}: Target creature with power 3 or less can't
        // be blocked this turn. — the printed target restriction is a power
        // comparison, and `Filter` has only `ToughnessAtMost` and `CmcAtMost`
        // beside it, so the restriction cannot be stated. Granting the
        // `unblockable` keyword to *any* creature (or to the wrong one) would
        // be a different card, so the ability is not offered at all.
    ],
);
