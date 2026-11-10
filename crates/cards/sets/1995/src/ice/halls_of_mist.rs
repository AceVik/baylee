//! Halls of Mist — (no cost) — Land
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Creatures that attacked during their controller's last turn can't attack.
//! Set: ICE #354 — Ice Age | Scryfall ID: b926a189-90b6-47bb-b5d6-b033e57007b4 | Oracle ID: 57d7deaf-ee08-49ab-b239-1c77a1f65225
use baylee_cards_dsl::prelude::*;

static ATTACKED_LAST_TURN: Filter =
    Filter::And(&[Filter::CREATURE, Filter::AttackedDuringControllersLastTurn]);

card!(
    index = index::HALLS_OF_MIST,
    oracle_id = "57d7deaf-ee08-49ab-b239-1c77a1f65225",
    scryfall_id = "b926a189-90b6-47bb-b5d6-b033e57007b4",
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Halls of Mist", types = TypeSet::LAND,)],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1)
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{1}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf]
                }
            ]
        ),
        static_ability!(ATTACKED_LAST_TURN, Modifier::CantAttack),
    ],
);
