//! Glacial Chasm — (no cost) — Land
//! Oracle: Cumulative upkeep—Pay 2 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When this land enters, sacrifice a land.
//! Oracle: Creatures you control can't attack.
//! Oracle: Prevent all damage that would be dealt to you.
//! Set: ME2 #229 — Masters Edition II | Scryfall ID: 0c008129-daba-46bc-829c-d2c0c13ecdd3 | Oracle ID: 73e7a2ad-d11c-4867-b97d-f971809da778

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLACIAL_CHASM,
    oracle_id = "73e7a2ad-d11c-4867-b97d-f971809da778",
    scryfall_id = "0c008129-daba-46bc-829c-d2c0c13ecdd3",
    faces = &[face!(name = "Glacial Chasm", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
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
                Effect::PlayerMayPayLifeOr {
                    player: PlayerRel::You,
                    life: Amount::Times {
                        base: &Amount::CountersOnSource(counters::AGE),
                        factor: 2
                    },
                    effect: &Effect::SacrificeSelf
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
        triggered!(
            Trigger::ETB,
            &[Effect::SacrificeFilter {
                who: PlayerRel::You,
                filter: &Filter::LAND
            }]
        ),
        static_ability!(Filter::YOUR_CREATURE, Modifier::CantAttack),
        static_ability!(Filter::This, Modifier::PreventDamageToYouFrom(&Filter::Any)),
    ],
);
