//! Caverns of Despair — {2}{R}{R} — World Enchantment
//! Oracle: No more than two creatures can attack each combat.
//! Oracle: No more than two creatures can block each combat.
//! Set: LEG #136 — Legends | Scryfall ID: 209f7479-b3a0-4c27-9602-78babb8d2e99 | Oracle ID: a1034a02-36cf-4586-a001-9dc3fb76e904
// PARTIAL — both limits are off the card; nothing in the DSL caps how many
// creatures may be declared as attackers or blockers (see the NOT SUPPORTED
// lines).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAVERNS_OF_DESPAIR,
    oracle_id = "a1034a02-36cf-4586-a001-9dc3fb76e904",
    scryfall_id = "209f7479-b3a0-4c27-9602-78babb8d2e99",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no modifier or replacement limits how many creatures can be declared \
         as attackers or as blockers in a combat"
    ),
    faces = &[face!(
        name = "Caverns of Despair",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
    // NOT SUPPORTED: "No more than two creatures can attack each combat." and
    // "No more than two creatures can block each combat." — the `Modifier`
    // list has per-creature combat modifiers (`CanBlockAdditional`,
    // `CanBlockAnyNumber`, `CantBeBlockedBy`, `AttacksEachCombat`), but none
    // caps a *count* of attackers or blockers, and no `Trigger` hears the
    // declaration of attackers or blockers to enforce one from the card.
    abilities = &[],
);
