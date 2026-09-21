//! Lake-town Lookout — {W} — Creature — Human Scout
//! Oracle: When this creature dies, recruit. (Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token.)
//! Set: HOB #18 — The Hobbit | Scryfall ID: 178c4cf6-6b11-40e4-9673-c560d6818a6b | Oracle ID: cf765efe-884c-48e2-9edb-9d45cf2756dd
// PARTIAL — the death trigger draws a card and then discards one, which is
// recruit's first two sentences; the token clause has nothing to hang on.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAKE_TOWN_LOOKOUT,
    oracle_id = "cf765efe-884c-48e2-9edb-9d45cf2756dd",
    scryfall_id = "178c4cf6-6b11-40e4-9673-c560d6818a6b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "recruit: no effect branches on which card was discarded, and the \
         1/1 white Human Soldier token is not in the pool"
    ),
    faces = &[face!(
        name = "Lake-town Lookout",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SCOUT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[
            Effect::draw(1),
            Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 1,
            },
        ]
    )],
);

// NOT SUPPORTED: "If you discarded a nonland card, create a 1/1 white Human
// Soldier creature token." — no `Effect` asks what the discard was (the four
// conditionals are about kicker, creatures died, lost life, greatest cmc, and
// the source's own counters), and `crate::tokens` carries no 1/1 white Human
// Soldier to create either way.
