//! Prophecy — {W} — Sorcery
//! Oracle: Reveal the top card of target opponent's library. If it's a land, you gain 1 life. Then that player shuffles.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: HML #11 — Homelands | Scryfall ID: f514eb63-3a4e-4410-ba3d-487cf81f7063 | Oracle ID: 8dcf15fd-f000-4424-b74c-22fc2642b8c3
// PARTIAL — revealing the top card of target opponent's library to gain 1 life if it is a land is not in Effect; target opponent shuffles their library and you draw a card at the beginning of the next turn's upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROPHECY,
    oracle_id = "8dcf15fd-f000-4424-b74c-22fc2642b8c3",
    scryfall_id = "f514eb63-3a4e-4410-ba3d-487cf81f7063",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "revealing the top card of target opponent's library to gain 1 life if it is a land is not in Effect; target opponent shuffles their library and you draw a card at the beginning of the next turn's upkeep"
    ),
    faces = &[face!(
        name = "Prophecy",
        mana_cost = mana!("{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::ShuffleLibrary {
                who: PlayerRel::Chosen,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )],
);
