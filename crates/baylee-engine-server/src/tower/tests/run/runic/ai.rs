#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn expert_recognizes_runic_activations_in_the_command_zone() {
    let player = PlayerId::new(0);
    let def = definition(Sigil::Dawn, Vessel::Scepter, Rarity::Common);
    let mut game = prepared(&[def], &[], 17);
    main(&mut game, 7, player);
    let id = source(&game, player, def);
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
        let used = matches!(action, PlayerAction::ActivateAbility { source, ability_index: 1 } if source == id);
        assert!(
            !matches!(action, PlayerAction::PassPriority),
            "Expert passed over a paid profitable activation: {action:?}"
        );
        game.apply(player, action).unwrap();
        if used {
            resolve(&mut game);
            assert_eq!(game.state().players[1].life, 19);
            assert_eq!(charges(&game, id), 0);
            return;
        }
    }
    panic!("Expert did not use its visible run equipment");
}
