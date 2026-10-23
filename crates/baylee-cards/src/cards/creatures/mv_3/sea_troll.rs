//! Sea Troll — {2}{U} — Creature — Troll
//! Oracle: {U}: Regenerate this creature. Activate only if this creature blocked or was blocked by a blue creature this turn.
//! Set: HML #39 — Homelands | Scryfall ID: b7da23d4-f9fb-40a5-8395-51b47a064600 | Oracle ID: feabe30a-3c28-43a1-b538-b0207758520c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

const BLOCKED_BY_BLUE_THIS_TURN: Filter = Filter::BlockedThisTurnBy(&BLUE_CREATURE);
// NOT SUPPORTED: "this creature blocked [a blue creature]" this turn. No Filter reads the colour of the attacker a blocker blocked (`Filter::BlockedThisTurn` takes no inner filter), so the gate counts only the printed "was blocked by a blue creature this turn" half; a Sea Troll that blocked a blue attacker cannot activate.
card!(
    index = index::SEA_TROLL,
    oracle_id = "feabe30a-3c28-43a1-b538-b0207758520c",
    scryfall_id = "b7da23d4-f9fb-40a5-8395-51b47a064600",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the 'blocked a blue creature this turn' half of the activation gate is not in Filter (no filter reads the colour of the attacker a blocker blocked); the {U} regeneration can be activated only after Sea Troll was blocked by a blue creature this turn, never after it blocked one"
    ),
    faces = &[face!(
        name = "Sea Troll",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TROLL],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{U}"),
        &[Effect::regenerate(TargetSpec::ThisObject)],
        condition = Some(Condition::SourceMatches(&BLOCKED_BY_BLUE_THIS_TURN)),
    )],
);
