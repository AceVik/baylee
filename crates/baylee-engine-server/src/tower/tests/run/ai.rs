#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::preset::AIProfile;
use baylee_engine::{choice::YesNoPrompt, engine::DecisionContext};

#[test]
fn run_expert_funds_cradle_from_its_only_command_mana_source() {
    let player = PlayerId::new(0);
    let reservoir = item(Family::RootReservoir, Rarity::Common);
    let mut game = opening(
        &[
            reservoir,
            item(Family::Grove, Rarity::MythicRare),
            item(Family::SeedCradle, Rarity::Rare),
            item(Family::EmberLantern, Rarity::Rare),
        ],
        &["Spellbook"],
    );
    let root = source(&game, player, reservoir);
    let starting_charges = charges(&game, root);
    assert!(starting_charges >= 3);
    let before = plants(&game, player);
    let card = in_hand(&game, player, "Spellbook").unwrap();
    game.apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    until(&mut game, |game| {
        matches!(
            game.pending(),
            Pending::YesNo {
                player: payer,
                prompt: YesNoPrompt::PayTax { .. } | YesNoPrompt::PayMana { .. },
                ..
            } if *payer == player
        )
    });
    assert!(game.optional_mana_offer_for(PlayerId::new(1)).is_none());
    let offer = game.optional_mana_offer_for(player).expect("payer's offer");
    assert!(offer.mana_abilities.is_empty());
    assert_eq!(offer.abilities, vec![(root, 1)]);

    let view = baylee_gamehost::view::player_view(
        game.state(),
        player,
        0,
        Some(game.pending()),
        &baylee_gamehost::view::SeatContext {
            awaiting: Some(player),
            ..Default::default()
        },
        &[],
    );
    let context = DecisionContext {
        optional_mana_offer: Some(&offer),
        ..game.decision_context()
    };
    let expert = |generation| {
        baylee_ai::HeuristicAgent::new(AIProfile {
            generation,
            ..AIProfile::TOWER_EXPERT
        })
    };
    assert_eq!(
        expert(AIProfile::OPTIONAL_TOKEN_SATURATION).act_with_context(
            &view,
            game.pending(),
            &context
        ),
        PlayerAction::YesNo(false)
    );
    let answer =
        expert(AIProfile::RUN_EQUIPMENT_MANA).act_with_context(&view, game.pending(), &context);
    assert_eq!(answer, PlayerAction::YesNo(true));
    game.apply(player, answer).unwrap();
    game.apply(
        player,
        PlayerAction::ActivateAbility {
            source: root,
            ability_index: 1,
        },
    )
    .unwrap();
    game.apply(player, PlayerAction::PassPriority).unwrap();
    resolve(&mut game);
    assert_eq!(charges(&game, root), starting_charges - 2);
    assert_eq!(plants(&game, player), before + 1);
}
