//! Witch's Clinic — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Target commander gains lifelink until end of turn.
//! Set: DSC #325 — Duskmourn: House of Horror Commander | Scryfall ID: d2422e3e-a923-4516-b87c-cac748354dca | Oracle ID: 05899372-9784-4bdb-9c28-504c71fed906

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WITCH_S_CLINIC,
    oracle_id = "05899372-9784-4bdb-9c28-504c71fed906",
    scryfall_id = "d2422e3e-a923-4516-b87c-cac748354dca",
    faces = &[face!(name = "Witch's Clinic", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::LIFELINK),
                duration: Duration::UntilEndOfTurn
            }],
            target = Some(TargetSpec::Object(&Filter::IsCommander))
        ),
    ],
);
