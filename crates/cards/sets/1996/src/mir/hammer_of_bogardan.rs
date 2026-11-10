//! Hammer of Bogardan — {1}{R}{R} — Sorcery
//! Oracle: Hammer of Bogardan deals 3 damage to any target.
//! Oracle: {2}{R}{R}{R}: Return this card from your graveyard to your hand. Activate only during your upkeep.
//! Set: 8ED #193 — Eighth Edition | Scryfall ID: abf6e6c0-ddea-4d15-ab1a-5c7d9fa69d51 | Oracle ID: 3089bdd7-87e6-4d0e-ad98-c32e6f861c30
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAMMER_OF_BOGARDAN,
    oracle_id = "3089bdd7-87e6-4d0e-ad98-c32e6f861c30",
    scryfall_id = "abf6e6c0-ddea-4d15-ab1a-5c7d9fa69d51",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hammer of Bogardan",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        activated!(
            cost!("{2}{R}{R}{R}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
            zone = ActivationZone::Graveyard,
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
