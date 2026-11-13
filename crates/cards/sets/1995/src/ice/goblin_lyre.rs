//! Goblin Lyre — {3} — Artifact
//! Oracle: Sacrifice this artifact: Flip a coin. If you win the flip, this artifact deals damage to target opponent or planeswalker equal to the number of creatures you control. If you lose the flip, this artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls.
//! Set: ICE #319 — Ice Age | Scryfall ID: 951114fb-5ae5-4eb0-8e03-6e39b0b634b5 | Oracle ID: 0709e9c8-dbe7-4806-ae75-d2e2d745d6a4
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GOBLIN_LYRE,
    oracle_id = "0709e9c8-dbe7-4806-ae75-d2e2d745d6a4",
    scryfall_id = "951114fb-5ae5-4eb0-8e03-6e39b0b634b5",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Lyre",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::FlipCoin {
            won: &[Effect::DealDamage {
                target: TargetSpec::OpponentOrObject(&Filter::PLANESWALKER),
                amount: Amount::ControlledCount {
                    who: PlayerRel::You,
                    filter: &Filter::YOUR_CREATURE,
                },
            }],
            lost: &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::You),
                amount: Amount::ControlledCount {
                    who: PlayerRel::TargetPlayerOrController,
                    filter: &Filter::CREATURE,
                },
            }],
        }],
        target = Some(TargetSpec::OpponentOrObject(&Filter::PLANESWALKER))
    )],
);
