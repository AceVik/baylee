//! Urborg Justice — {B}{B} — Instant
//! Oracle: Target opponent sacrifices a creature of their choice for each creature put into your graveyard from the battlefield this turn.
//! Set: WTH #84 — Weatherlight | Scryfall ID: 39f322ff-0b04-41ce-90cd-9896f941e703 | Oracle ID: d8e2dea9-0e96-4ce4-a070-9893f44cf0cc
// PARTIAL — Target opponent sacrifices a creature of their choice if a creature died this turn; Effect::SacrificeFilter sacrifices one creature rather than scaling by the exact count of creatures put into your graveyard from the battlefield this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URBORG_JUSTICE,
    oracle_id = "d8e2dea9-0e96-4ce4-a070-9893f44cf0cc",
    scryfall_id = "39f322ff-0b04-41ce-90cd-9896f941e703",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Target opponent sacrifices a creature of their choice if a creature died this turn; Effect::SacrificeFilter sacrifices one creature rather than scaling by the exact count of creatures put into your graveyard from the battlefield this turn."
    ),
    faces = &[face!(
        name = "Urborg Justice",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::IfCondition {
            condition: Condition::CreaturesDiedThisTurnAtLeast(1),
            then: &[Effect::SacrificeFilter {
                who: PlayerRel::TargetPlayerOrController,
                filter: &Filter::CREATURE,
            }],
            otherwise: &[],
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
