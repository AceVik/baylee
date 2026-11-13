//! Palace Jailer — {2}{W}{W} — Creature — Human Soldier
//! Oracle: When this creature enters, you become the monarch.
//! Oracle: When this creature enters, exile target creature an opponent controls until an opponent becomes the monarch.
//! Set: MSC #140 — Marvel Super Heroes Commander | Scryfall ID: 3a8c2a84-e0f2-4611-af3d-42f4578ad4e3 | Oracle ID: 180eda7c-fca2-403b-85cd-8ffebaf9f408
// IMPLEMENTED — monarch designation (become monarch, monarch draw at end
// step) and the exile "until an opponent becomes the monarch"
// (Effect::exile_until, CR 610.3): an opponent of the player who controlled
// the trigger, whoever controls the Jailer by then.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::PALACE_JAILER,
    oracle_id = "180eda7c-fca2-403b-85cd-8ffebaf9f408",
    scryfall_id = "3a8c2a84-e0f2-4611-af3d-42f4578ad4e3",
    faces = &[face!(
        name = "Palace Jailer",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::HUMAN, creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::BecomeMonarch(PlayerRel::You)]),
        triggered!(
            Trigger::ETB,
            &[Effect::exile_until(
                TargetSpec::Object(&Filter::OPPONENT_CREATURE),
                ExileUntil::OpponentBecomesMonarch
            )],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &Filter::OPPONENT_CREATURE
            )))
        ),
    ],
);
