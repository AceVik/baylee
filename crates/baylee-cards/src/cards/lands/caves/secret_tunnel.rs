//! Secret Tunnel — (no cost) — Land — Cave
//! Oracle: This land can't be blocked.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn.
//! Set: TLA #278 — Avatar: The Last Airbender | Scryfall ID: 2d39a0e1-6484-409c-ab05-5b276925a949 | Oracle ID: 632e2979-d88a-482e-9bb8-57b683c5310f
// PARTIAL — {C} mana plus the printed unblockable clause; the {4}, {T} clause
// asks for two targets, and an activated ability can state only one.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SECRET_TUNNEL,
    oracle_id = "632e2979-d88a-482e-9bb8-57b683c5310f",
    scryfall_id = "2d39a0e1-6484-409c-ab05-5b276925a949",
    faces = &[face!(
        name = "Secret Tunnel",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Partial(
        "{4}, {T}: two target creatures you control that share a creature type \
         can't be blocked — AbilityDef::Activated states one TargetSpec and no \
         target count, and no Filter can say \"shares a creature type with the \
         other target\""
    ),
    abilities = &[
        // This land can't be blocked.
        static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{4}, {T}: Two target creatures you control that share
        // a creature type can't be blocked this turn." — the ability needs a
        // target count of two (`AbilityDef::Activated` carries a bare
        // `TargetSpec`) and a target restriction no Filter can phrase, so it is
        // off the card rather than offered as a one-creature ability.
    ],
);
