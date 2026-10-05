//! Livonya Silone — {2}{R}{R}{G}{G} — Legendary Creature — Human Warrior
//! Oracle: First strike; legendary landwalk (This creature can't be blocked as long as defending player controls a legendary land.)
//! Set: ME3 #160 — Masters Edition III | Scryfall ID: d4ef4c06-9dab-499c-a270-0ba7f0b5fd77 | Oracle ID: 57f2aa02-f5f3-42a5-939d-5cc94200951e
// PARTIAL — first strike is written; legendary landwalk is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIVONYA_SILONE,
    oracle_id = "57f2aa02-f5f3-42a5-939d-5cc94200951e",
    scryfall_id = "d4ef4c06-9dab-499c-a270-0ba7f0b5fd77",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Partial(
        "legendary landwalk names a land *filter* (a legendary land), and \
         the landwalk family is expressible only for the five basic land \
         types: `KeywordSet` carries those five bits and no `Modifier` \
         makes a creature unblockable from what the defending player \
         controls"
    ),
    faces = &[face!(
        name = "Livonya Silone",
        mana_cost = mana!("{2}{R}{R}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(4),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "legendary landwalk (This creature can't be blocked as
    // long as defending player controls a legendary land.)" — CR 702.14's
    // family is a filter over land *types*, and `KeywordSet` carries only
    // the five basic landwalks (PLAINSWALK, ISLANDWALK, SWAMPWALK,
    // MOUNTAINWALK, FORESTWALK). The nearest pieces are the wrong half:
    // `Modifier::CantBeBlockedBy` filters the blockers, where landwalk asks
    // what the defending player controls, and
    // `Modifier::CantAttackUnlessDefenderControls` restricts attacking.
    abilities = &[],
);
