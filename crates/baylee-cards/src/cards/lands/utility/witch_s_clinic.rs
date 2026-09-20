//! Witch's Clinic — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Target commander gains lifelink until end of turn.
//! Set: DSC #325 — Duskmourn: House of Horror Commander | Scryfall ID: d2422e3e-a923-4516-b87c-cac748354dca | Oracle ID: 05899372-9784-4bdb-9c28-504c71fed906
// PARTIAL — the mana ability is built; the lifelink activation cannot name its
// target, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WITCH_S_CLINIC,
    oracle_id = "05899372-9784-4bdb-9c28-504c71fed906",
    scryfall_id = "d2422e3e-a923-4516-b87c-cac748354dca",
    faces = &[face!(name = "Witch's Clinic", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "no Filter variant names a commander, so \"Target commander\" has no \
         TargetSpec to point at"
    ),
    // NOT SUPPORTED: "{2}, {T}: Target commander gains lifelink until end of
    // turn." The effect half is sayable — a continuous effect granting
    // KeywordSet::LIFELINK until end of turn — but the target half is not:
    // being a commander is a designation (CR 903.3), not a type, supertype or
    // subtype, and `Filter` has no predicate for it (MatchesChosenTypeOfSource,
    // SharesSubtypeWithCommander and the rest are about other things), so
    // TargetReq cannot be aimed at one. The ability comes off the card rather
    // than being offered as a pump that would accept any permanent put in front
    // of it.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
