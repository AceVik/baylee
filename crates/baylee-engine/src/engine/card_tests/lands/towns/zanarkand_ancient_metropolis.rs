//! `cards/lands/towns/zanarkand_ancient_metropolis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn zanarkand_ancient_metropolis_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[zanarkand_ancient_metropolis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, zanarkand_ancient_metropolis());
    assert!(entered_tapped(&engine, land));
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert!(!types(&engine, land).contains(TypeSet::SORCERY));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, zanarkand_ancient_metropolis(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn lasting_fayth_counts_lands_at_resolution_and_doubles_tokens_and_counters_independently() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for doubled in [false, true] {
        let mut board = vec![forest(), forest(), forest()];
        if doubled {
            board.push(index::DOUBLING_SEASON);
        }
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[forest()])
            .hand(0, &[zanarkand_ancient_metropolis()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let card = begin_town_adventure(
            &mut e,
            p0,
            zanarkand_ancient_metropolis(),
            ManaColor::Green,
            6,
        );
        pass_until(&mut e, stack_is_empty);
        let tokens = tokens_of(&e, p0);
        assert_eq!(tokens.len(), if doubled { 2 } else { 1 });
        for token in tokens {
            assert_eq!(
                e.state()
                    .object(token)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                if doubled { 6 } else { 3 }
            );
            assert_eq!(pt(&e, token), if doubled { (7, 7) } else { (4, 4) });
        }
        assert_adventure_exiled(&e, p0, card);
    }
}
