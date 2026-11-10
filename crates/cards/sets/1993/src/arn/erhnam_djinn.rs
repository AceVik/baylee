//! Erhnam Djinn — {3}{G} — Creature — Djinn
//! Oracle: At the beginning of your upkeep, target non-Wall creature an opponent controls gains forestwalk until your next upkeep. (It can't be blocked as long as defending player controls a Forest.)
//! Set: VMA #207 — Vintage Masters | Scryfall ID: a1b20fb7-90f3-442c-b105-dfcaf619348d | Oracle ID: d48a38c9-3dcd-4c18-8840-1b057ede3ff0
// IMPLEMENTED — at your upkeep, target non-Wall creature an opponent
// controls gains forestwalk until your next upkeep
// (Duration::UntilYourNextUpkeep).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Non-Wall creature an opponent controls."
static NON_WALL_OPPONENT_CREATURE: Filter = Filter::And(&[
    Filter::OPPONENT_CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::ERHNAM_DJINN,
    oracle_id = "d48a38c9-3dcd-4c18-8840-1b057ede3ff0",
    scryfall_id = "a1b20fb7-90f3-442c-b105-dfcaf619348d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Erhnam Djinn",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(4),
        toughness = Some(5),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::continuous(
            &Filter::This,
            Modifier::AddKeyword(KeywordSet::FORESTWALK),
            Duration::UntilYourNextUpkeep,
        )],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &NON_WALL_OPPONENT_CREATURE
        ))),
    )],
);
