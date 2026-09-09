//! Tuktuk Scrapper — {3}{R} — Creature — Goblin Artificer Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may destroy target artifact. If that artifact is put into a graveyard this way, this creature deals damage to that artifact's controller equal to the number of Allies you control.
//! Set: WWK #94 — Worldwake | Scryfall ID: d3a84a2a-6384-497a-8ee2-de0fa74fcc80 | Oracle ID: 85cf2403-b419-4364-8ac9-67dd1ceddf9e
// IMPLEMENTED — rally artifact destruction + damage per Ally (damage hits
// the destroyed artifact's controller).

use crate::filters::YOUR_ALLIES;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

card! {
    index: 174,
    oracle_id: "85cf2403-b419-4364-8ac9-67dd1ceddf9e",
    scryfall_id: "d3a84a2a-6384-497a-8ee2-de0fa74fcc80",
    faces: &[face! {
        name: "Tuktuk Scrapper",
        mana_cost: baylee_core::mana!("{3}{R}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::GOBLIN, creature::ARTIFICER, creature::ALLY],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&YOUR_ALLIES), &[
            Effect::Destroy {
                target: TargetSpec::Object(&Filter::ARTIFACT),
            },
            Effect::DealDamageToTargetController {
                amount: Amount::CountOf {
                    filter: &ALLIES_YOU,
                    zone: ZoneSel::Battlefield,
                },
            },
        ], targets: Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::ARTIFACT))))],
}
