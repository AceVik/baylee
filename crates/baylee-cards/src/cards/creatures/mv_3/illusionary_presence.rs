//! Illusionary Presence — {1}{U}{U} — Creature — Illusion
//! Oracle: Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your upkeep, choose a land type. This creature gains landwalk of the chosen type until end of turn. (It can't be blocked as long as defending player controls a land of that type.)
//! Set: ICE #76 — Ice Age | Scryfall ID: aa31efed-4a11-4f59-a623-bac45d20091d | Oracle ID: 8f319b31-efb0-4f48-a2a3-1f308e7b1dda
// NOT SUPPORTED: the upkeep trigger offers only the five basic land types rather than any land type (CR 205.3i).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ILLUSIONARY_PRESENCE,
    oracle_id = "8f319b31-efb0-4f48-a2a3-1f308e7b1dda",
    scryfall_id = "aa31efed-4a11-4f59-a623-bac45d20091d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the upkeep trigger offers only the five basic land types rather than any land type (CR 205.3i)",
    ),
    faces = &[face!(
        name = "Illusionary Presence",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{U}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::ChooseSourceLandwalk {
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
