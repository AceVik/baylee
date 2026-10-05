//! Elder Land Wurm — {4}{W}{W}{W} — Creature — Dragon Wurm
//! Oracle: Defender, trample
//! Oracle: When this creature blocks, it loses defender.
//! Set: ME1 #11 — Masters Edition | Scryfall ID: 369b3ebc-572b-4614-aacb-4990309d8848 | Oracle ID: 5948381a-419d-4b8b-8ea2-ea623cb0d606
// PARTIAL — defender and trample are on the card; the block trigger is not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELDER_LAND_WURM,
    oracle_id = "5948381a-419d-4b8b-8ea2-ea623cb0d606",
    scryfall_id = "369b3ebc-572b-4614-aacb-4990309d8848",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Elder Land Wurm",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON, subtypes::creature::WURM],
        power = Some(5),
        toughness = Some(5),
    ),],
    keywords = KeywordSet::DEFENDER.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Partial(
        "there is no one-sided \"blocks\" trigger, and \
         Trigger::BlocksOrBecomesBlockedBy is the union of blocking and \
         becoming blocked"
    ),
    // NOT SUPPORTED: "When this creature blocks, it loses defender." —
    // `Trigger::BlocksOrBecomesBlockedBy` fires on both sides of a block, so
    // writing it would also fire when this creature becomes blocked, and no
    // condition can drop the other half without making the effect an
    // intervening `if` that a removed blocker would fizzle.
);
