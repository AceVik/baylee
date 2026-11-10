//! Underdark Rift — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {5}, {T}, Exile this land: Roll a d10. Put target artifact, creature, or planeswalker into its owner's library just beneath the top X cards of that library, where X is the result. Activate only as a sorcery.
//! Set: AFC #62 — Forgotten Realms Commander | Scryfall ID: da7b5c5b-fba1-4993-9779-bef96bcb0064 | Oracle ID: 199c7604-6b4b-4cec-b6c2-b6b2918b11c8

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE_OR_PLANESWALKER: Filter = Filter::HasType(
    TypeSet::ARTIFACT
        .union(TypeSet::CREATURE)
        .union(TypeSet::PLANESWALKER),
);

card!(
    index = index::UNDERDARK_RIFT,
    oracle_id = "199c7604-6b4b-4cec-b6c2-b6b2918b11c8",
    scryfall_id = "da7b5c5b-fba1-4993-9779-bef96bcb0064",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Underdark Rift", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{5}", TapSelf, ExileSelf),
            &[Effect::RollDiePutTargetIntoLibraryBeneathTop { sides: 10 }],
            timing = ActivationTiming::SorcerySpeed,
            target = Some(TargetSpec::Object(&ARTIFACT_CREATURE_OR_PLANESWALKER)),
        ),
    ],
);
