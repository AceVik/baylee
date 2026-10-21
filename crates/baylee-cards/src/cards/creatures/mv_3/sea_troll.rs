//! Sea Troll — {2}{U} — Creature — Troll
//! Oracle: {U}: Regenerate this creature. Activate only if this creature blocked or was blocked by a blue creature this turn.
//! Set: HML #39 — Homelands | Scryfall ID: b7da23d4-f9fb-40a5-8395-51b47a064600 | Oracle ID: feabe30a-3c28-43a1-b538-b0207758520c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLUE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

const BLOCKED_OR_BLOCKED_BY_BLUE_THIS_TURN: Filter =
    Filter::Or(&[Filter::BlockedThisTurn, Filter::BlockedThisTurnBy(&BLUE_CREATURE)]);

// NOT SUPPORTED: Filter::BlockedThisTurn does not filter on the characteristics of the blocked attacker, so blocking any creature this turn or being blocked by a blue creature this turn satisfies the activation gate.
card!(
    index = index::SEA_TROLL,
    oracle_id = "feabe30a-3c28-43a1-b538-b0207758520c",
    scryfall_id = "b7da23d4-f9fb-40a5-8395-51b47a064600",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "checking whether the creature Sea Troll blocked this turn was blue is not in Filter; requires that Sea Troll blocked a creature or was blocked by a blue creature this turn to activate its {U} regeneration ability"
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
        condition = Some(Condition::SourceMatches(
            &BLOCKED_OR_BLOCKED_BY_BLUE_THIS_TURN
        )),
    )],
);
