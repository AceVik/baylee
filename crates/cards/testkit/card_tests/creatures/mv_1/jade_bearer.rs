// Oracle checklist: another own Merfolk only; self/non-Merfolk/opponent Merfolk excluded; controller rather than owner; exact one counter and +1/+1 persist across turn; no legal target branch.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("72ddbc9b-f584-433f-9d2d-6e4c374129c4")
}

#[test]
fn entry_counters_only_another_merfolk_controlled_by_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let merfolk = baylee_core::generated::index::MERFOLK_OF_THE_PEARL_TRIDENT;
    for borrowed in [false, true] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(0, &[forest(), merfolk, llanowar_elves()])
            .battlefield(1, &[merfolk])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, p0);
        let ours = on_battlefield(&game, p0, merfolk).unwrap();
        let theirs = on_battlefield(&game, p1, merfolk).unwrap();
        let elf = on_battlefield(&game, p0, llanowar_elves()).unwrap();
        let target = if borrowed { theirs } else { ours };
        if borrowed {
            game.dev_state_mut(p0)
                .unwrap()
                .object_mut(theirs)
                .unwrap()
                .set_controller(p0);
            game.refresh_offer();
            assert_eq!(game.state().object(theirs).unwrap().owner, p1);
            assert_eq!(game.state().object(theirs).unwrap().controller, p0);
        }
        cast_from_hand(&mut game, p0, subject());
        pass_until(&mut game, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let source = on_battlefield(&game, p0, subject()).unwrap();
        let options = aim_at(&mut game, p0, target);
        assert!(options.contains(&ours));
        assert_eq!(options.contains(&theirs), borrowed);
        assert!(!options.contains(&source));
        assert!(!options.contains(&elf));
        pass_until(&mut game, stack_is_empty);
        assert_eq!(
            game.state()
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            1
        );
        assert_eq!(pt(&game, target), (2, 2));
        assert_eq!(pt(&game, source), (1, 1));
        assert_eq!(pt(&game, elf), (1, 1));
        let bystander = if borrowed { ours } else { theirs };
        assert_eq!(pt(&game, bystander), (1, 1));
        let turn = game.state().turn.number;
        pass_until(&mut game, |e| e.state().turn.number > turn);
        assert_eq!(pt(&game, target), (2, 2));
        assert_eq!(
            game.state()
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            1
        );
    }
}

#[test]
fn with_no_other_own_merfolk_no_counter_is_placed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let merfolk = baylee_core::generated::index::MERFOLK_OF_THE_PEARL_TRIDENT;
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .battlefield(1, &[merfolk])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, stack_is_empty);
    for (seat, card) in [(p0, subject()), (p0, llanowar_elves()), (p1, merfolk)] {
        let id = on_battlefield(&game, seat, card).unwrap();
        assert_eq!(pt(&game, id), (1, 1));
        assert_eq!(
            game.state()
                .object(id)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            0
        );
    }
}
