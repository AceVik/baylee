//! Arcum's Weathervane — {2} — Artifact
//! Oracle: {2}, {T}: Target snow land is no longer snow.
//! Oracle: {2}, {T}: Target nonsnow basic land becomes snow.
//! Set: ICE #310 — Ice Age | Scryfall ID: 9e142435-6930-4596-bc3b-60abde1229df | Oracle ID: b4e57fba-5fa3-4fd5-8f0a-7b61593ade72

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARCUM_S_WEATHERVANE,
    oracle_id = "b4e57fba-5fa3-4fd5-8f0a-7b61593ade72",
    scryfall_id = "9e142435-6930-4596-bc3b-60abde1229df",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Arcum's Weathervane",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::RemoveSupertype(SupertypeSet::SNOW),
                Duration::Indefinitely,
            )],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::LAND,
                Filter::HasSupertype(SupertypeSet::SNOW),
            ]))),
        ),
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::AddSupertype(SupertypeSet::SNOW),
                Duration::Indefinitely,
            )],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::LAND,
                Filter::HasSupertype(SupertypeSet::BASIC),
                Filter::Not(&Filter::HasSupertype(SupertypeSet::SNOW)),
            ]))),
        ),
    ],
);
