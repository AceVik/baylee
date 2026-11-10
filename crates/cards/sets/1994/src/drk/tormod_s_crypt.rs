//! Tormod's Crypt — {0} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Exile target player's graveyard.
//! Set: DMR #235 — Dominaria Remastered | Scryfall ID: 752bf493-5839-47e8-95f2-6d8201907428 | Oracle ID: 1573f7f9-672c-421a-b1ac-3d0d8aea59ca

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORMOD_S_CRYPT,
    oracle_id = "1573f7f9-672c-421a-b1ac-3d0d8aea59ca",
    scryfall_id = "752bf493-5839-47e8-95f2-6d8201907428",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tormod's Crypt",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("", TapSelf, SacrificeSelf),
        &[Effect::ExileGraveyard {
            player: PlayerRel::Chosen,
        }],
        target = Some(TargetSpec::AnyPlayer),
    )],
);
