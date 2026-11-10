//! Uktabi Efreet — {2}{G}{G} — Creature — Efreet
//! Oracle: Cumulative upkeep {G} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Set: WTH #143 — Weatherlight | Scryfall ID: 3678a224-d314-4108-8a39-de0c1b635b5c | Oracle ID: 2fd9a37f-151a-4501-9d8e-bc67cc902592
// IMPLEMENTED — Cumulative upkeep {G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UKTABI_EFREET,
    oracle_id = "2fd9a37f-151a-4501-9d8e-bc67cc902592",
    scryfall_id = "3678a224-d314-4108-8a39-de0c1b635b5c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Uktabi Efreet",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
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
                each: mana!("{G}"),
                times: Amount::CountersOnSource(counters::AGE),
                paid: &[],
                unpaid: &[Effect::SacrificeSelf]
            }
        ],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
