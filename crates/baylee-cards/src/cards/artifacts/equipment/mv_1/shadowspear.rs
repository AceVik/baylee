//! Shadowspear — {1} — Legendary Artifact — Equipment
//! Oracle: Equipped creature gets +1/+1 and has trample and lifelink.
//! Oracle: {1}: Permanents your opponents control lose hexproof and indestructible until end of turn.
//! Oracle: Equip {2}
//! Set: THB #236 — Theros Beyond Death | Scryfall ID: 939c6e19-4b27-4023-bb9c-ae440f91e21c | Oracle ID: 8b27326f-e7b8-4a4d-b589-df459246d19a
// IMPLEMENTED — equipped creature +1/+1, trample, lifelink; {1} strips hexproof and indestructible from opponents' permanents until EOT; equip {2}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHADOWSPEAR,
    oracle_id = "8b27326f-e7b8-4a4d-b589-df459246d19a",
    scryfall_id = "939c6e19-4b27-4023-bb9c-ae440f91e21c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shadowspear",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::TRAMPLE.union(KeywordSet::LIFELINK)),
        ),
        activated!(
            cost!("{1}"),
            &[Effect::continuous(
                &Filter::ControlledByOpponent,
                Modifier::RemoveKeyword(KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE)),
                Duration::UntilEndOfTurn,
            )],
        ),
        equip!(cost!("{2}")),
    ],
);
