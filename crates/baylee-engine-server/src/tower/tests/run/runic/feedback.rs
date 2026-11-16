#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn a_single_mythic_feedback_item_cannot_replenish_its_own_full_free_cost() {
    let player = PlayerId::new(0);
    for (sigil, vessel, cost, remaining) in [
        (Sigil::Bloom, Vessel::Seed, 7, 6),
        (Sigil::Lore, Vessel::Lens, 3, 2),
        (Sigil::Echo, Vessel::Lens, 3, 2),
    ] {
        let def = definition(sigil, vessel, Rarity::MythicRare);
        let mut game = prepared(&[def], &[], 17);
        main(&mut game, 1, player);
        let id = source(&game, player, def);
        assert_eq!(charges(&game, id), cost);
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        resolve(&mut game);
        assert_eq!(charges(&game, id), remaining);
        assert!(
            game.apply(player, action).is_err(),
            "one relic cannot form a self-sustaining free loop"
        );
    }
}

#[test]
fn expert_stops_optional_paid_growth_with_an_ample_board_but_human_still_can() {
    let player = PlayerId::new(0);
    let def = definition(Sigil::Dawn, Vessel::Seed, Rarity::MythicRare);
    let mut game = prepared(&[def, item(Family::Grove, Rarity::MythicRare)], &[], 17);
    main(&mut game, 11, player);
    let id = source(&game, player, def);
    let before = plants(&game, player);
    assert!(before >= 32);
    let expert = baylee_ai::HeuristicAgent::new(v2::generation::CURRENT.expert());
    for _ in 0..8 {
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
        let action = expert.act_with_context(&view, game.pending(), &game.decision_context());
        assert!(!matches!(action, PlayerAction::ActivateAbility { source, .. } if source == id));
        if action == PlayerAction::PassPriority {
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: 1,
                },
            )
            .unwrap();
            resolve(&mut game);
            assert_eq!(plants(&game, player), before + 3);
            return;
        }
        game.apply(player, action).unwrap();
    }
    panic!("Expert did not finish its finite development window");
}
