//! Inner Sanctum — {1}{W}{W} — Enchantment
//! Oracle: Cumulative upkeep—Pay 2 life. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Prevent all damage that would be dealt to creatures you control.
//! Set: WTH #18 — Weatherlight | Scryfall ID: 2298faae-370e-4b87-bf32-d20c2282a928 | Oracle ID: 2ae36d31-08b7-481a-bfa1-a5c65921dee0
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INNER_SANCTUM,
    oracle_id = "2ae36d31-08b7-481a-bfa1-a5c65921dee0",
    scryfall_id = "2298faae-370e-4b87-bf32-d20c2282a928",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Inner Sanctum",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
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
                }
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            )))
        ),
        static_ability!(
            Filter::YOUR_CREATURE,
            Modifier::PreventDamageFrom(&Filter::Any)
        ),
    ],
);
