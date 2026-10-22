//! Heat Stroke — {2}{R} — Enchantment
//! Oracle: At end of combat, destroy each creature that blocked or was blocked this turn.
//! Set: WTH #107 — Weatherlight | Scryfall ID: 1baf2a6c-57ec-4b38-8b08-4b3f800dbe99 | Oracle ID: a8127c04-f67f-4780-86bf-bb24480ec51a
// PARTIAL — Destroys each creature that blocked this turn at end of combat; also destroying creatures that were blocked this turn is not yet modeled.

use baylee_cards_dsl::prelude::*;

static BLOCKED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockedThisTurn]);

card!(
    index = index::HEAT_STROKE,
    oracle_id = "a8127c04-f67f-4780-86bf-bb24480ec51a",
    scryfall_id = "1baf2a6c-57ec-4b38-8b08-4b3f800dbe99",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Destroys each creature that blocked this turn at end of combat; also destroying creatures that were blocked this turn is not yet modeled."
    ),
    faces = &[face!(
        name = "Heat Stroke",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatEnd,
            whose: PlayerRel::EachPlayer
        },
        &[Effect::DestroyAll {
            filter: &BLOCKED_CREATURE,
            no_regen: false
        }],
        condition = Some(Condition::SourceMatches(&Filter::InZone(
            ZoneRef::Battlefield
        )))
    ),],
);
