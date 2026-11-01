//! Ib Halfheart, Goblin Tactician — {3}{R} — Legendary Creature — Goblin Advisor
//! Oracle: Whenever another Goblin you control becomes blocked, sacrifice it. If you do, it deals 4 damage to each creature blocking it.
//! Oracle: Sacrifice two Mountains: Create two 1/1 red Goblin creature tokens.
//! Set: EVG #43 — Duel Decks Anthology: Elves vs. Goblins | Scryfall ID: e950415c-2abe-4afe-9ef4-6d63eba2c833 | Oracle ID: 6742c4b9-8fa6-439f-844d-04a09fa20c45
// IMPLEMENTED — another Goblin's block trigger and a two-Mountain token ability.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_GOBLIN_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::Another,
    Filter::ControlledByYou,
]);
static MOUNTAIN_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
    Filter::ControlledByYou,
]);

card!(
    index = index::IB_HALFHEART_GOBLIN_TACTICIAN,
    oracle_id = "6742c4b9-8fa6-439f-844d-04a09fa20c45",
    scryfall_id = "e950415c-2abe-4afe-9ef4-6d63eba2c833",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ib Halfheart, Goblin Tactician",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ADVISOR],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::ControlledBecomesBlocked(&ANOTHER_GOBLIN_YOU_CONTROL),
            &[Effect::SacrificeEventDealDamageToEachBlocker {
                amount: Amount::Fixed(4)
            }]
        ),
        activated!(
            cost!(
                Sacrifice(&MOUNTAIN_YOU_CONTROL),
                Sacrifice(&MOUNTAIN_YOU_CONTROL)
            ),
            &[Effect::CreateTokenN {
                amount: Amount::Fixed(2),
                token: &generated_tokens::GOBLIN_1_1_RED
            }]
        ),
    ],
);
