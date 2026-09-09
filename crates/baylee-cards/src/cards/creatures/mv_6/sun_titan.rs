//! Sun Titan — {4}{W}{W} — Creature — Giant
//! Oracle: Vigilance
//! Oracle: Whenever this creature enters or attacks, you may return target permanent card with mana value 3 or less from your graveyard to the battlefield.
//! Set: SOC #178 — Secrets of Strixhaven Commander | Scryfall ID: 3d6eacf2-f6c7-4ede-b5a5-7463602699ae | Oracle ID: b2e950fb-cb7e-40a0-a311-5bbdd0477b29
// IMPLEMENTED — vigilance + reanimation on ETB and on attack.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static SMALL_PERMANENT: Filter = Filter::And(&[
    Filter::CmcAtMost(3),
    Filter::Not(&Filter::HasType(TypeSet::INSTANT)),
    Filter::Not(&Filter::HasType(TypeSet::SORCERY)),
]);

card! {
    index: 158,
    oracle_id: "b2e950fb-cb7e-40a0-a311-5bbdd0477b29",
    scryfall_id: "3d6eacf2-f6c7-4ede-b5a5-7463602699ae",
    faces: &[face! {
        name: "Sun Titan",
        mana_cost: baylee_core::mana!("{4}{W}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::GIANT],
        power: Some(6),
        toughness: Some(6),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::VIGILANCE,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&SMALL_PERMANENT, PlayerRel::You),
            }], targets: Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
                &SMALL_PERMANENT,
                PlayerRel::You,
            )))),
        triggered!(Trigger::Attacks(&Filter::This), &[Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&SMALL_PERMANENT, PlayerRel::You),
            }], targets: Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
                &SMALL_PERMANENT,
                PlayerRel::You,
            )))),
    ],
}
