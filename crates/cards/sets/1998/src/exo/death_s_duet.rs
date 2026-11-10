//! Death's Duet — {2}{B} — Sorcery
//! Oracle: Return two target creature cards from your graveyard to your hand.
//! Set: TPR #101 — Tempest Remastered | Scryfall ID: 666647e9-afc3-4b04-bad9-b4ce11438ee4 | Oracle ID: 28d24add-da3a-4697-8fd2-5106c9225c0e
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_S_DUET,
    oracle_id = "28d24add-da3a-4697-8fd2-5106c9225c0e",
    scryfall_id = "666647e9-afc3-4b04-bad9-b4ce11438ee4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Death's Duet",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        targets = Some(TargetReq::exactly(
            TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            2,
        )),
    )],
);
