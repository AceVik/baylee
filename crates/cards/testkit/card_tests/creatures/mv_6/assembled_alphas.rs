//! Assembled Alphas' damage after the opposing combatant blinks.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

#[test]
fn assembled_alphas_hits_the_departed_combatants_former_controller() {
    use baylee_engine::event::DamageTarget;

    let (player, opponent) = (PlayerId::new(0), PlayerId::new(1));
    let alphas = card_index("151a681d-a350-4d1d-86cd-b80d58602810");
    for alphas_attacks in [false, true] {
        let mut game = Duel::new(SEED, plains())
            .battlefield(0, &[alphas, grizzly_bears(), plains(), plains()])
            .hand(0, &[index::FLICKER_OF_FATE])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, player);

        let wolf = on_battlefield(&game, player, alphas).unwrap();
        let bear = on_battlefield(&game, player, grizzly_bears()).unwrap();
        let old_version = game.state().object(bear).unwrap().version;
        // The opponent controls the bear in combat. Flicker returns it to its owner.
        game.dev_state_mut(player)
            .unwrap()
            .object_mut(bear)
            .unwrap()
            .set_controller(opponent);
        game.refresh_offer();

        let (attacker, blocker, defender) = if alphas_attacks {
            (wolf, bear, opponent)
        } else {
            reach_their_main_phase(&mut game, opponent);
            (bear, wolf, player)
        };
        let blocks = attack_and_collect_blocks(&mut game, attacker, defender);
        assert!(blocks.iter().any(|option| option.blocker == blocker));
        let journal_start = game.journal().entries().len();
        game.apply(
            defender,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
        assert!(
            !stack_is_empty(&game),
            "the non-targeted trigger is on the stack"
        );
        pass_until(
            &mut game,
            |game| matches!(game.pending(), Pending::Priority { player: seat, .. } if *seat == player),
        );

        cast_from_hand(&mut game, player, index::FLICKER_OF_FATE);
        aim_at(&mut game, player, bear);
        pass_until(&mut game, stack_is_empty);

        let returned = game.state().object(bear).unwrap();
        assert_eq!(returned.zone, Zone::Battlefield);
        assert_ne!(returned.version, old_version);
        assert_eq!(returned.controller, player);
        assert_eq!(returned.damage, 0);
        assert_eq!(game.state().players[0].life, 20);
        assert_eq!(game.state().players[1].life, 17);
        assert_eq!(
            damage_events(&game, journal_start),
            [(wolf, DamageTarget::Player(opponent), 3, false)]
        );
    }
}
