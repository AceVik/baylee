//! Mtenda Griffin — {3}{W} — Creature — Griffin
//! Oracle: Flying
//! Oracle: {W}, {T}: Return this creature to its owner's hand and return target Griffin card from your graveyard to your hand. Activate only during your upkeep.
//! Set: MIR #28 — Mirage | Scryfall ID: 162dd988-0beb-48e4-9eaa-a08ddb835648 | Oracle ID: 273f0353-4936-491b-82da-6d4d7f3259a6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const GRIFFIN_CARD: Filter = Filter::HasSubtype(subtypes::creature::GRIFFIN);

card!(
    index = index::MTENDA_GRIFFIN,
    oracle_id = "273f0353-4936-491b-82da-6d4d7f3259a6",
    scryfall_id = "162dd988-0beb-48e4-9eaa-a08ddb835648",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mtenda Griffin",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GRIFFIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{W}", TapSelf),
        &[
            Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            },
            Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&GRIFFIN_CARD, PlayerRel::You),
            },
        ],
        target = Some(TargetSpec::CardInGraveyard(&GRIFFIN_CARD, PlayerRel::You)),
        condition = Some(Condition::All(&[
            Condition::YourTurn,
            Condition::DuringStep(StepKind::Upkeep),
        ])),
    )],
);
