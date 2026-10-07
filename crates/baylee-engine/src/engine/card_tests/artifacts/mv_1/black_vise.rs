//! `cards/artifacts/mv_1/black_vise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Black Vise pays its printed cost, then chooses its opponent on entry.
#[test]
fn alpha_eval_black_vise_supported_cast_and_resolution() {
    let p0 = PlayerId::new(0);
    let card = card_index("de7839fb-7040-48ab-a6d4-d1952972943d");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[forest(); 1])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    assert!(on_stack(&engine, card).is_some(), "the card was cast");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost was paid"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    assert!(engine.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err());
    engine
        .apply(p0, PlayerAction::ChoosePlayer(PlayerId::new(1)))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, card).is_some());
    assert!(in_hand(&engine, p0, card).is_none());
}

/// Damage counts the chosen opponent's hand before their draw and floors at zero.
#[test]
fn black_vise_counts_only_the_chosen_opponents_hand_at_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let vise = card_index("de7839fb-7040-48ab-a6d4-d1952972943d");
    for (cards, damage) in [(0, 0), (4, 0), (7, 3)] {
        let mut engine = Duel::new(1009, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[vise])
            .hand(1, &vec![forest(); cards])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        cast_from_hand(&mut engine, p0, vise);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        let source = on_battlefield(&engine, p0, vise).unwrap();
        assert_eq!(
            engine.state().object(source).unwrap().chosen_opponent(),
            Some(p1)
        );
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && !stack_is_empty(e)
        });
        assert_eq!(engine.state().turn.step, crate::turn::Step::Upkeep);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[1].life, 20 - damage);
        assert_eq!(engine.state().players[0].life, 20);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
            cards,
            "the draw step has not happened yet"
        );
    }
}

/// Destroying the source does not stop its stacked trigger; drawing in response
/// changes the eventual damage because the hand count is read on resolution.
#[test]
fn black_vise_resolves_after_destruction_and_counts_response_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let vise = card_index("de7839fb-7040-48ab-a6d4-d1952972943d");
    let disenchant = card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a");
    let recall = card_index("550c74d4-1fcb-406a-b02a-639a760a4380");
    let mut engine = Duel::new(1010, forest())
        .battlefield(0, &[forest(), plains(), plains(), island()])
        .hand(0, &[vise, disenchant, recall])
        .hand(1, &[forest(); 7])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    cast_with_floating(&mut engine, p0, vise);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    let source = on_battlefield(&engine, p0, vise).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, disenchant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, vise).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().object(source).unwrap().chosen_opponent(),
        None,
        "the card in the graveyard remembers no permanent's choice"
    );
    cast_from_hand(&mut engine, p0, recall);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p1)).len(), 10);
    assert_eq!(engine.state().players[1].life, 14);
}

/// Entry offers only opponents; another opponent's upkeep does not fire it.
#[test]
fn black_vise_chooses_among_opponents_and_ignores_other_upkeeps() {
    let (p0, p1, p2, p3) = (
        PlayerId::new(0),
        PlayerId::new(1),
        PlayerId::new(2),
        PlayerId::new(3),
    );
    let vise = card_index("de7839fb-7040-48ab-a6d4-d1952972943d");
    let mut engine = Duel::table(1011, forest(), 4)
        .team(0, 0)
        .team(1, 0)
        .team(2, 1)
        .team(3, 2)
        .battlefield(0, &[forest()])
        .hand(0, &[vise])
        .hand(2, &[forest(); 6])
        .hand(3, &[forest(); 7])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, vise);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(options, &[p2, p3]);
    assert!(engine.apply(p0, PlayerAction::ChoosePlayer(p1)).is_err());
    engine.apply(p0, PlayerAction::ChoosePlayer(p3)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p3 && !stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[2].life,
        20,
        "the other opponent was not chosen"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[3].life, 17);
}

/// A bounced Vise loses its first choice and names a new opponent on re-entry.
#[test]
fn black_vise_reentry_makes_a_fresh_choice() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let vise = card_index("de7839fb-7040-48ab-a6d4-d1952972943d");
    let boomerang = card_index("dc4a4996-108a-4aac-850f-2d9f76403446");
    let mut engine = Duel::table(1012, forest(), 3)
        .battlefield(0, &[forest(), forest(), island(), island()])
        .hand(0, &[vise, boomerang])
        .hand(1, &[forest(); 4])
        .hand(2, &[forest(); 7])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    cast_with_floating(&mut engine, p0, vise);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    let source = on_battlefield(&engine, p0, vise).unwrap();
    cast_from_hand(&mut engine, p0, boomerang);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, vise).is_some());
    assert_eq!(
        engine.state().object(source).unwrap().chosen_opponent(),
        None
    );
    cast_with_floating(&mut engine, p0, vise);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(
        stack_is_empty(&engine),
        "the first choice no longer triggers"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p2 && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[2].life, 17);
}
