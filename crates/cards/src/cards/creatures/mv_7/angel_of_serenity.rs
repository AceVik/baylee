//! Angel of Serenity — {4}{W}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: When this creature enters, you may exile up to three other target creatures from the battlefield and/or creature cards from graveyards.
//! Oracle: When this creature leaves the battlefield, return the exiled cards to their owners' hands.
//! Set: C21 #83 — Commander 2021 | Scryfall ID: 60301fbc-1bd4-4c37-ad19-660d625a090a | Oracle ID: f4ce6078-8c7b-4f68-b324-13130c63a983
// IMPLEMENTED — mixed battlefield/graveyard linked exile and return to hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_CREATURE_OR_CARD: Filter = Filter::And(&[Filter::Another, Filter::CREATURE]);

card!(
    index = index::ANGEL_OF_SERENITY,
    oracle_id = "f4ce6078-8c7b-4f68-b324-13130c63a983",
    scryfall_id = "60301fbc-1bd4-4c37-ad19-660d625a090a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angel of Serenity",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(5),
        toughness = Some(6),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::ExileTargetsWithSource],
            }],
            targets = Some(TargetReq::up_to(
                TargetSpec::ObjectOrCardInAnyGraveyard(&OTHER_CREATURE_OR_CARD),
                3,
            )),
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedExile { to: ZoneRef::Hand }],
        ),
    ],
);
