//! Skirk Drill Sergeant — {1}{R} — Creature — Goblin
//! Oracle: Whenever this creature or another Goblin dies, you may pay {2}{R}. If you do, reveal the top card of your library. If it's a Goblin permanent card, put it onto the battlefield. Otherwise, put it into your graveyard.
//! Set: EVG #49 — Duel Decks Anthology: Elves vs. Goblins | Scryfall ID: da50d139-bf87-4fae-a41a-d2c343b7992b | Oracle ID: eb0d97a3-6494-4011-9f60-1b12b165a995
// IMPLEMENTED — its own or another Goblin's death offers a reveal-and-recruit payment.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GOBLIN_PERMANENT_CARD: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::BATTLE)
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::KINDRED)
            .union(TypeSet::LAND)
            .union(TypeSet::PLANESWALKER),
    ),
]);
static ANOTHER_GOBLIN: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::GOBLIN),
    Filter::Another,
]);
static THIS_OR_ANOTHER_GOBLIN: Filter = Filter::Or(&[Filter::This, ANOTHER_GOBLIN]);

card!(
    index = index::SKIRK_DRILL_SERGEANT,
    oracle_id = "eb0d97a3-6494-4011-9f60-1b12b165a995",
    scryfall_id = "da50d139-bf87-4fae-a41a-d2c343b7992b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skirk Drill Sergeant",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&THIS_OR_ANOTHER_GOBLIN),
        &[Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: mana!("{2}{R}"),
            effects: &[Effect::RevealTopAndSort {
                filter: &GOBLIN_PERMANENT_CARD,
                matched: SearchDest::Battlefield,
                otherwise: SearchDest::Graveyard,
            }],
        }],
    )],
);
