//! Oath of Scholars — {3}{U} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who has more cards in hand than they do and is their opponent. The first player may discard their hand and draw three cards.
//! Set: EXO #42 — Exodus | Scryfall ID: d61376ad-21c8-4d34-b37d-ed60877f5d4a | Oracle ID: da4a5129-b005-4fc6-b175-0416627881db
// PARTIAL: Triggers on your upkeep targeting any opponent to let you discard your hand and draw three cards; triggering for each player and restricting target opponent to one who has more cards in hand are not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_SCHOLARS,
    oracle_id = "da4a5129-b005-4fc6-b175-0416627881db",
    scryfall_id = "d61376ad-21c8-4d34-b37d-ed60877f5d4a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Triggers on your upkeep targeting any opponent to let you discard your hand and draw three cards; triggering for each player and restricting target opponent to one who has more cards in hand are not supported.",
    ),
    faces = &[face!(
        name = "Oath of Scholars",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[
                Effect::DiscardHand {
                    who: PlayerRel::You,
                },
                Effect::draw(3),
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    ),],
);
