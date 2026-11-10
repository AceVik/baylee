//! Asmira, Holy Avenger — {2}{G}{W} — Legendary Creature — Human Cleric
//! Oracle: Flying
//! Oracle: At the beginning of each end step, put a +1/+1 counter on Asmira for each creature put into your graveyard from the battlefield this turn.
//! Set: MIR #256 — Mirage | Scryfall ID: a7d64600-84fc-42a5-a6a6-b26f98fac0a4 | Oracle ID: a0397f06-d557-4c9f-af89-238ab38ada3e
// PARTIAL — Flying only: the end-step +1/+1 counters are not played.
// NOT SUPPORTED: "At the beginning of each end step, put a +1/+1 counter on Asmira for each creature put into your graveyard from the battlefield this turn." The only count of creatures that died this turn takes every player's creatures, not just those put into your graveyard, so the ability as written would grow Asmira on the opponent's creatures too.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASMIRA_HOLY_AVENGER,
    oracle_id = "a0397f06-d557-4c9f-af89-238ab38ada3e",
    scryfall_id = "a7d64600-84fc-42a5-a6a6-b26f98fac0a4",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "Flying works; the end-step +1/+1 counters are not played, because the only count of creatures that died this turn takes every player's creatures, not only those put into your graveyard, and would grow Asmira on the opponent's deaths.",
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
);
