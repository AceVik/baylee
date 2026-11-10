//! Heartwood Shard grants trample with either activation cost.

use super::*;

fn heartwood_shard() -> CardIndex {
    card_index("64d4bdc5-ff40-469d-bb25-82de0a49dfa4")
}

#[test]
fn heartwood_shard_grants_trample_with_either_printed_cost() {
    let p0 = PlayerId::new(0);
    for (ability, lands) in [(0, 3), (1, 1)] {
        let mut board = vec![heartwood_shard(), gray_ogre()];
        board.extend(vec![forest(); lands]);
        let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
        assert!(!keywords(&e, ogre).contains(KeywordSet::TRAMPLE));
        tap_all_mana(&mut e, p0);
        activate(&mut e, p0, heartwood_shard(), ability);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert!(keywords(&e, ogre).contains(KeywordSet::TRAMPLE));
    }
}
