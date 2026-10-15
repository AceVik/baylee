// Oracle checklist: each players upkeep only active player discards exactly one at random; both players; no selection prompt; one-card/empty hand boundaries; deterministic seeded repeat and varying choices across seeds.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("91e6fb47-59e4-4616-b8dd-3a7e30070074")
}

fn one_random_discard(seed: u64) -> CardIndex {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(seed, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[subject(), island(), mountain(), plains()])
        .hand(1, &[island(), mountain(), plains()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, stack_is_empty);
    for active in [p1, p0] {
        if active == p0 {
            reach_their_main_phase(&mut game, p1);
        }
        let own_before = game.state().zones.list(ZoneLocation::Hand(active)).clone();
        let other = if active == p0 { p1 } else { p0 };
        let other_before = game.state().zones.list(ZoneLocation::Hand(other)).clone();
        pass_until(&mut game, |e| {
            e.state().turn.active == active
                && e.state().turn.step == crate::turn::Step::Upkeep
                && !stack_is_empty(e)
        });
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(active)),
            &own_before
        );
        pass_until(&mut game, stack_is_empty);
        let after = game.state().zones.list(ZoneLocation::Hand(active));
        assert_eq!(after.len(), own_before.len() - 1);
        let removed: Vec<_> = own_before
            .iter()
            .copied()
            .filter(|id| !after.contains(id))
            .collect();
        assert_eq!(removed.len(), 1);
        assert!(
            game.state()
                .zones
                .list(ZoneLocation::Graveyard(active))
                .contains(&removed[0])
        );
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(other)),
            &other_before
        );
    }
    let grave = game.state().zones.list(ZoneLocation::Graveyard(p1));
    assert_eq!(grave.len(), 1);
    game.state().object(grave[0]).unwrap().card.unwrap().index
}

#[test]
fn each_upkeep_discards_randomly_with_reproducible_seeded_choices() {
    let first = one_random_discard(SEED);
    assert_eq!(one_random_discard(SEED), first);
    let mut choices = vec![first];
    for seed in SEED + 1..SEED + 8 {
        let chosen = one_random_discard(seed);
        if !choices.contains(&chosen) {
            choices.push(chosen);
        }
    }
    assert!(
        choices.len() > 1,
        "a random discard must not always choose the same hand position"
    );
}

#[test]
fn one_card_is_discarded_and_an_empty_hand_is_safe_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[subject()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, stack_is_empty);
    pass_until(&mut game, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == crate::turn::Step::Upkeep
            && !stack_is_empty(e)
    });
    pass_until(&mut game, stack_is_empty);
    assert!(game.state().zones.list(ZoneLocation::Hand(p1)).is_empty());
    assert!(in_graveyard(&game, p1, island()).is_some());
    // The controller has spent its sole card and starts its next upkeep empty.
    let turn = game.state().turn.number;
    pass_until(&mut game, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(game.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
    let grave = game.state().zones.list(ZoneLocation::Graveyard(p0)).clone();
    pass_until(&mut game, stack_is_empty);
    assert!(game.state().zones.list(ZoneLocation::Hand(p0)).is_empty());
    assert_eq!(game.state().zones.list(ZoneLocation::Graveyard(p0)), &grave);
}
