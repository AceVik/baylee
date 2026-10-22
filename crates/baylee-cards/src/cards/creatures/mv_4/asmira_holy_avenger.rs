//! Asmira, Holy Avenger — {2}{G}{W} — Legendary Creature — Human Cleric
//! Oracle: Flying
//! Oracle: At the beginning of each end step, put a +1/+1 counter on Asmira for each creature put into your graveyard from the battlefield this turn.
//! Set: MIR #256 — Mirage | Scryfall ID: a7d64600-84fc-42a5-a6a6-b26f98fac0a4 | Oracle ID: a0397f06-d557-4c9f-af89-238ab38ada3e
// NOT SUPPORTED: counting only creatures put into your graveyard from the battlefield this turn rather than any graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASMIRA_HOLY_AVENGER,
    oracle_id = "a0397f06-d557-4c9f-af89-238ab38ada3e",
    scryfall_id = "a7d64600-84fc-42a5-a6a6-b26f98fac0a4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "has flying and at the beginning of each end step puts a +1/+1 counter on itself for each creature that died this turn; restricting the count to creatures put into your graveyard rather than any graveyard is not supported",
    ),
    faces = &[face!(
        name = "Asmira, Holy Avenger",
        mana_cost = mana!("{2}{G}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(3),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::AddCounterSource {
            kind: CounterKind::P1P1,
            amount: Amount::CreaturesDiedThisTurn,
        }],
    )],
);
