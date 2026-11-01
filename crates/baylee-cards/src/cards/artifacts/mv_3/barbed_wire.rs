//! Barbed Wire — {3} — Artifact
//! Oracle: At the beginning of each player's upkeep, this artifact deals 1 damage to that player.
//! Oracle: {2}: Prevent the next 1 damage that would be dealt by this artifact this turn.
//! Set: MMQ #287 — Mercadian Masques | Scryfall ID: be9e2e59-1527-4c61-9cc9-dcaf1181bd43 | Oracle ID: cdc6e187-73d2-4f79-a0ff-daa6d2de5bdb
// NOT SUPPORTED: optional {2} ability preventing the next 1 damage that would be dealt by this artifact this turn is not implemented
// PARTIAL — upkeep 1-damage trigger to each active player is implemented; optional `{2}: Prevent the next 1 damage that would be dealt by this artifact this turn` is omitted because recipient-free finite source-based prevention is not in `Effect`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BARBED_WIRE,
    oracle_id = "cdc6e187-73d2-4f79-a0ff-daa6d2de5bdb",
    scryfall_id = "be9e2e59-1527-4c61-9cc9-dcaf1181bd43",
    coverage = Coverage::Partial(
        "optional {2} ability preventing the next 1 damage that would be dealt by this artifact this turn is not implemented",
    ),
    faces = &[face!(
        name = "Barbed Wire",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::ActivePlayer),
        }],
    )],
);
