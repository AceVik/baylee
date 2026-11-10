//! Granite Shard's two activation costs, played.

use super::*;

fn granite_shard() -> CardIndex {
    card_index("db81c783-a622-41e1-9dd6-47fab1d76967")
}

#[test]
fn granite_shard_deals_one_with_either_printed_cost() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (ability, lands) in [(0, 3), (1, 1)] {
        let mut board = vec![granite_shard()];
        board.extend(vec![mountain(); lands]);
        let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        tap_all_mana(&mut e, p0);
        activate(&mut e, p0, granite_shard(), ability);
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[1].life, 19, "ability {ability}");
        assert!(is_tapped(
            &e,
            on_battlefield(&e, p0, granite_shard()).unwrap()
        ));
    }
}
