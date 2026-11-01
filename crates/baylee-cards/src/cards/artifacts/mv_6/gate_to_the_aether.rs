//! Gate to the Aether — {6} — Artifact
//! Oracle: At the beginning of each player's upkeep, that player reveals the top card of their library. If it's an artifact, creature, enchantment, or land card, the player may put it onto the battlefield.
//! Set: MRD #174 — Mirrodin | Scryfall ID: c28fd840-e633-46d3-991a-b9fea95a2f28 | Oracle ID: a03ab63a-83f3-4dd6-926d-08d09ce2e696
use baylee_cards_dsl::prelude::*;

static PERMITTED: Filter = Filter::HasType(
    TypeSet::ARTIFACT
        .union(TypeSet::CREATURE)
        .union(TypeSet::ENCHANTMENT)
        .union(TypeSet::LAND),
);

card!(
    index = index::GATE_TO_THE_AETHER,
    oracle_id = "a03ab63a-83f3-4dd6-926d-08d09ce2e696",
    scryfall_id = "c28fd840-e633-46d3-991a-b9fea95a2f28",
    faces = &[face!(
        name = "Gate to the Aether",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::RevealTopPlayerMayPutOntoBattlefield {
            player: PlayerRel::ActivePlayer,
            filter: &PERMITTED,
        }],
    )],
);
