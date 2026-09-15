//! Orcish Bowmasters — {1}{B} — Creature — Orc Archer
//! Oracle: Flash
//! Oracle: When this creature enters and whenever an opponent draws a card except the first one they draw in each of their draw steps, this creature deals 1 damage to any target. Then amass Orcs 1.
//! Set: LTR #103 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 7c024bae-5631-4e20-ac69-df392ac9e109 | Oracle ID: ea5103f5-27e0-4eb1-902c-7f34652d6bf3
// IMPLEMENTED — flash + ping + amass on opponents' extra draws.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// The card prints one ability with two triggers, and the engine has no
/// "enters or draws" trigger, so it is written twice. The effects are named
/// once rather than typed twice: they were typed twice, and the second copy
/// lost the damage.
static PING_THEN_AMASS: &[Effect] = &[
    Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::AnyTarget,
    },
    Effect::Amass {
        token: &crate::tokens::ARMY_0_0_BLACK,
        subtype: creature::ORC,
        amount: 1,
    },
];

card!(
    index = 26056,
    oracle_id = "ea5103f5-27e0-4eb1-902c-7f34652d6bf3",
    scryfall_id = "7c024bae-5631-4e20-ac69-df392ac9e109",
    faces = &[face!(
        name = "Orcish Bowmasters",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::ORC, creature::ARCHER],
        power = Some(1),
        toughness = Some(1),
    )],
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::EntersBattlefield(&Filter::This),
            PING_THEN_AMASS,
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
        triggered!(
            Trigger::DrawsExceptFirst(PlayerRel::Opponent),
            PING_THEN_AMASS,
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
    ],
);
