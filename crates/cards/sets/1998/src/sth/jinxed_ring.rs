//! Jinxed Ring — {2} — Artifact
//! Oracle: Whenever a nontoken permanent is put into your graveyard from the battlefield, this artifact deals 1 damage to you.
//! Oracle: Sacrifice a creature: Target opponent gains control of this artifact. (This effect lasts indefinitely.)
//! Set: STH #137 — Stronghold | Scryfall ID: 1c675814-094c-45d4-93ee-0f50b4755e79 | Oracle ID: 29cb8f47-7bb5-4881-98ef-59fc858cf9d7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONTOKEN_OWNED_BY_YOU: Filter =
    Filter::And(&[Filter::OwnedByYou, Filter::Not(&Filter::IsToken)]);

card!(
    index = index::JINXED_RING,
    oracle_id = "29cb8f47-7bb5-4881-98ef-59fc858cf9d7",
    scryfall_id = "1c675814-094c-45d4-93ee-0f50b4755e79",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jinxed Ring",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::Dies(&NONTOKEN_OWNED_BY_YOU),
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::You),
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!(Sacrifice(&Filter::YOUR_CREATURE)),
            &[Effect::GainControlAll {
                filter: &Filter::This,
                to: PlayerRel::TargetPlayerOrController,
            }],
            target = Some(TargetSpec::AnyOpponent),
        ),
    ],
);
