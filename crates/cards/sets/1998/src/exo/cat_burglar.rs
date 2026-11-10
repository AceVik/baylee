//! Cat Burglar — {3}{B} — Creature — Kor Rogue Minion
//! Oracle: {2}{B}, {T}: Target player discards a card. Activate only as a sorcery.
//! Set: EXO #54 — Exodus | Scryfall ID: 947109f9-7035-4a2a-bbc2-a2958f8c5d01 | Oracle ID: 16aea5b3-9f43-4852-b8b8-80e87909b794
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CAT_BURGLAR,
    oracle_id = "16aea5b3-9f43-4852-b8b8-80e87909b794",
    scryfall_id = "947109f9-7035-4a2a-bbc2-a2958f8c5d01",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cat Burglar",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::KOR,
            subtypes::creature::ROGUE,
            subtypes::creature::MINION,
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{2}{B}", TapSelf),
        &[Effect::DiscardCount {
            who: PlayerRel::Chosen,
            count: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        timing = ActivationTiming::SorcerySpeed,
    )],
);
