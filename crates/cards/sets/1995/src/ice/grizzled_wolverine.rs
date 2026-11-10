//! Grizzled Wolverine — {1}{R}{R} — Creature — Wolverine
//! Oracle: {R}: This creature gets +2/+0 until end of turn. Activate only during the declare blockers step, only if at least one creature is blocking this creature, and only once each turn.
//! Set: ICE #192 — Ice Age | Scryfall ID: 95bb17b9-55c4-4cc1-83f6-75490b9a97d0 | Oracle ID: da4ed552-a85a-47e1-ab67-5c93a9a20048
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKING_SOURCE: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockingSource]);
static WOLVERINE_COND: [Condition; 2] = [
    Condition::DuringStep(StepKind::DeclareBlockers),
    Condition::BattlefieldCount(&BLOCKING_SOURCE, 1),
];

card!(
    index = index::GRIZZLED_WOLVERINE,
    oracle_id = "da4ed552-a85a-47e1-ab67-5c93a9a20048",
    scryfall_id = "95bb17b9-55c4-4cc1-83f6-75490b9a97d0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Grizzled Wolverine",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLVERINE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, 0),
            duration: Duration::UntilEndOfTurn,
        }],
        limit = ActivationLimit::PerTurn(1),
        condition = Some(Condition::All(&WOLVERINE_COND))
    )],
);
