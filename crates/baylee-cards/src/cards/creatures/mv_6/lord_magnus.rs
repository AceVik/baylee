//! Lord Magnus — {3}{G}{W}{W} — Legendary Creature — Human Druid
//! Oracle: First strike
//! Oracle: Creatures with plainswalk can be blocked as though they didn't have plainswalk.
//! Oracle: Creatures with forestwalk can be blocked as though they didn't have forestwalk.
//! Set: LEG #243 — Legends | Scryfall ID: 2a02aabb-c464-4672-b37b-d5d713ef8939 | Oracle ID: 59258149-1c18-421b-b940-d0aa29da572a
// PARTIAL — first strike is written; the two "as though they didn't have"
// sentences are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LORD_MAGNUS,
    oracle_id = "59258149-1c18-421b-b940-d0aa29da572a",
    scryfall_id = "2a02aabb-c464-4672-b37b-d5d713ef8939",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Lord Magnus",
        mana_cost = mana!("{3}{G}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(4),
        toughness = Some(3),
    ),],
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Partial(
        "no Modifier makes a landwalk creature blockable as though it lacked \
         the keyword — `RemoveKeyword` would take the keyword away entirely, \
         which is a different card — so both landwalk sentences are off",
    ),
    // NOT SUPPORTED: "Creatures with plainswalk can be blocked as though they
    // didn't have plainswalk." — no `Modifier` ignores one keyword for one
    // rule the way `AttacksDespiteDefender` ignores defender for attacking;
    // the engine's block check reads `KeywordSet::PLAINSWALK` directly.
    // NOT SUPPORTED: "Creatures with forestwalk can be blocked as though they
    // didn't have forestwalk." — same missing modifier, for
    // `KeywordSet::FORESTWALK`.
);
