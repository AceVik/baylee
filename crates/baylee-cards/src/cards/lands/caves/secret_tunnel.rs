//! Secret Tunnel — (no cost) — Land — Cave
//! Oracle: This land can't be blocked.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn.
//! Set: TLA #278 — Avatar: The Last Airbender | Scryfall ID: 2d39a0e1-6484-409c-ab05-5b276925a949 | Oracle ID: 632e2979-d88a-482e-9bb8-57b683c5310f
// PARTIAL — {C} mana plus the printed unblockable clause; the {4}, {T} clause
// restricts its two targets to each other, which no Filter can say.

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
         can't be blocked — a two-target requirement is sayable, but no Filter \
         relates one target's creature types to the other's, and without that \
         restriction the ability would take any two creatures"
    ),
    abilities = &[
        // This land can't be blocked.
        static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{4}, {T}: Two target creatures you control that share
        // a creature type can't be blocked this turn." — two targets are
        // sayable (`TargetReq::exactly(…, 2)`), but no Filter relates one
        // target to the other, so it is off the card rather than offered for
        // any two creatures.
    ],
);
