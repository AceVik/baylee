//! Loxodon Peacekeeper moves to the player with least life, including ties.

use super::*;

fn loxodon_peacekeeper() -> CardIndex {
    card_index("89390fc2-17af-4754-bb1c-7af905d4a2a8")
}

#[test]
fn peacekeeper_moves_to_unique_lowest_life_and_to_a_chosen_tied_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for tied in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[loxodon_peacekeeper()])
            .life(0, 20)
            .life(1, 10)
            .start();
        keep_mulligans(&mut e);
        if tied {
            e.dev_state_mut(p0).unwrap().players[0].life = 10;
            e.refresh_offer();
        }
        if tied {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChoosePlayer { .. })
            });
            let Pending::ChoosePlayer { player, options } = e.pending().clone() else {
                panic!("a tie asks its controller")
            };
            assert_eq!(player, p0);
            assert_eq!(options, vec![p0, p1]);
            e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
            pass_until(&mut e, stack_is_empty);
        } else {
            reach_main_phase(&mut e, p0);
        }
        let peacekeeper = all_on_battlefield(&e, p1, loxodon_peacekeeper())[0];
        assert_eq!(e.state().object(peacekeeper).unwrap().controller, p1);
    }
}
