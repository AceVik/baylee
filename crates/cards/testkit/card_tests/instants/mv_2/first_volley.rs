//! First Volley's simultaneous damage, played with and without redirection.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

#[test]
fn first_volley_redirected_by_pariah_gives_one_trigger_the_combined_amount() {
    let (player, opponent) = (PlayerId::new(0), PlayerId::new(1));
    let volley = card_index("b6a89457-1581-46d4-b75b-c5b8704d9f6f");
    for redirect in [false, true] {
        let mut game = Duel::new(SEED, mountain())
            .battlefield(
                0,
                &[
                    index::TRUEFIRE_CAPTAIN,
                    plains(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                ],
            )
            .hand(0, &[index::PARIAH, volley])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, player);
        let host = on_battlefield(&game, player, index::TRUEFIRE_CAPTAIN).unwrap();
        tap_all_mana(&mut game, player);
        if redirect {
            cast_with_floating(&mut game, player, index::PARIAH);
            aim_at(&mut game, player, host);
            pass_until(&mut game, stack_is_empty);
        }

        cast_with_floating(&mut game, player, volley);
        aim_at(&mut game, player, host);
        pass_until(&mut game, |game| {
            matches!(game.pending(), Pending::ChooseTargets { .. })
        });
        game.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![opponent],
            },
        )
        .unwrap();
        pass_until(&mut game, stack_is_empty);

        let combined = if redirect { 2 } else { 1 };
        assert_eq!(game.state().object(host).unwrap().damage, combined);
        assert_eq!(game.state().players[0].life, if redirect { 20 } else { 19 });
        assert_eq!(game.state().players[1].life, 20 - i32::from(combined));
    }
}
