//! Extraplanar Lens — {3} — Artifact
//! Oracle: Imprint — When this artifact enters, you may exile target land you control.
//! Oracle: Whenever a land with the same name as the exiled card is tapped for mana, its controller adds one mana of any type that land produced.
//! Set: CMM #381 — Commander Masters | Scryfall ID: f050a675-c6a2-4d0d-bac6-1d8c51ecf824 | Oracle ID: e48f9fc7-72cf-4c3a-b3b3-d237edc01310
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static SAME_NAME_LAND: Filter = Filter::And(&[Filter::LAND, Filter::SameNameAsLinkedExile]);

card!(
    index = index::EXTRAPLANAR_LENS,
    oracle_id = "e48f9fc7-72cf-4c3a-b3b3-d237edc01310",
    scryfall_id = "f050a675-c6a2-4d0d-bac6-1d8c51ecf824",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Extraplanar Lens",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::ExileTargetsWithSource],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND))),
        ),
        triggered!(
            Trigger::TappedForMana {
                by: PlayerRel::EachPlayer,
                filter: &SAME_NAME_LAND,
            },
            &[Effect::AddManaLikeEvent { amount: 1 }],
        ),
    ],
);
