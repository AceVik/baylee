//! First Volley: damage to target creature and its controller, with and without redirection.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b6a89457-1581-46d4-b75b-c5b8704d9f6f"
fn first_volley() -> CardIndex {
    card_index("b6a89457-1581-46d4-b75b-c5b8704d9f6f")
}

#[test]
fn first_volley_redirected_by_pariah_gives_one_trigger_the_combined_amount() {
    let (player, opponent) = (PlayerId::new(0), PlayerId::new(1));
    let volley = first_volley();
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

/// First Volley ({1}{R}): deals 1 damage to target creature and 1 damage to
/// THAT creature's controller (tested on both opponent's 1/1 and your own 2/2).
#[test]
fn first_volley_deals_one_to_target_creature_and_one_to_its_controller() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::LLANOWAR_ELVES],
        &[first_volley(), first_volley()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    // 1. Target opponent's 1/1 Elf: kills Elf and deals 1 to P1.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut engine, P0, first_volley());
    lk::aim(&mut engine, &[their_elf], &[]);
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::LLANOWAR_ELVES).is_some());
    assert_eq!(lk::life(&engine, P1), 19, "opponent took 1 damage");
    assert_eq!(lk::life(&engine, P0), 20);

    // 2. Target your own 2/2 Bears: 1 damage marked on Bears and 1 to P0.
    cast_with_floating(&mut engine, P0, first_volley());
    lk::aim(&mut engine, &[my_bears], &[]);
    lk::settle(&mut engine);

    assert_eq!(engine.state().object(my_bears).unwrap().damage, 1);
    assert_eq!(lk::life(&engine, P0), 19, "target's controller (P0) took 1");
    assert_eq!(lk::life(&engine, P1), 19);
}
