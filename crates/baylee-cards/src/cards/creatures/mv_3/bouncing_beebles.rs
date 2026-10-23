//! Bouncing Beebles — {2}{U} — Creature — Beeble
//! Oracle: This creature can't be blocked as long as defending player controls an artifact.
//! Set: ULG #28 — Urza's Legacy | Scryfall ID: 8656bdd4-0c45-43f9-b2dc-d11a355ff747 | Oracle ID: 4fa46972-d953-4328-9c31-96e134234133
// PARTIAL: checking specifically the defending player rather than any opponent is not in Condition; this creature can't be blocked as long as an opponent controls an artifact.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByOpponent]);

card!(
    index = index::BOUNCING_BEEBLES,
    oracle_id = "4fa46972-d953-4328-9c31-96e134234133",
    scryfall_id = "8656bdd4-0c45-43f9-b2dc-d11a355ff747",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "checking specifically the defending player rather than any opponent is not in Condition; this creature can't be blocked as long as an opponent controls an artifact",
    ),
    faces = &[face!(
        name = "Bouncing Beebles",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEEBLE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
        condition = Some(Condition::OpponentControlCount(&OPPONENT_ARTIFACT, 1)),
    )],
);
