//! Nether Spirit — {1}{B}{B} — Creature — Spirit
//! Oracle: At the beginning of your upkeep, if this card is the only creature card in your graveyard, you may return this card to the battlefield.
//! Set: MH1 #98 — Modern Horizons | Scryfall ID: 82f1f3e1-d986-4c0b-850e-5e8f2eaa0642 | Oracle ID: 194e2563-c0bf-4151-bbb0-0d626130b055
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SELF_CREATURE: Filter = Filter::And(&[Filter::This, Filter::CREATURE]);
const OTHER_CREATURE: Filter = Filter::And(&[Filter::Another, Filter::CREATURE]);
const ONLY_CREATURE_IN_YOUR_GRAVEYARD: [Condition; 2] = [
    Condition::AmountAtLeast(
        Amount::CountOf {
            filter: &SELF_CREATURE,
            zone: ZoneSel::GraveyardYou,
        },
        1,
    ),
    Condition::Not(&Condition::AmountAtLeast(
        Amount::CountOf {
            filter: &OTHER_CREATURE,
            zone: ZoneSel::GraveyardYou,
        },
        1,
    )),
];

card!(
    index = index::NETHER_SPIRIT,
    oracle_id = "194e2563-c0bf-4151-bbb0-0d626130b055",
    scryfall_id = "82f1f3e1-d986-4c0b-850e-5e8f2eaa0642",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nether Spirit",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::ReturnSourceFromGraveyard {
                face: None,
                tapped: false,
                owner_control: false,
                counters: None,
            }],
        }],
        zone = TriggerZone::Graveyard,
        condition = Some(Condition::All(&ONLY_CREATURE_IN_YOUR_GRAVEYARD)),
    ),],
);
