//! `cards/artifacts/mv_4/smokestack.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ArrangePrompt;
use baylee_cards_dsl::counters;

fn smokestack() -> CardIndex {
    card_index("fda61506-ed79-47fd-badd-639185297913")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn soot(engine: &Engine<RegistryLookup>, stack: ObjectId) -> u16 {
    engine
        .state()
        .object(stack)
        .unwrap()
        .counters
        .get(counters::SOOT)
}

/// Reaches the controller's upkeep and puts the optional soot trigger on top of
/// the mandatory sacrifice trigger. The returned question is therefore
/// the counter choice; declining or accepting it determines what the sacrifice
/// trigger sees when it subsequently resolves.
#[track_caller]
fn offer_soot_before_sacrifice(engine: &mut Engine<RegistryLookup>, stack: ObjectId) {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                player,
                prompt: ArrangePrompt::TriggerOrder,
                ..
            } if *player == p0
        )
    });
    let rows = engine.queued_triggers();
    assert_eq!(rows.len(), 2, "both Smokestack abilities trigger");
    assert!(
        rows.iter()
            .all(|trigger| trigger.loc.source == stack && trigger.loc.card == Some(smokestack())),
        "both triggers belong to the same Smokestack: {rows:?}"
    );
    let add = rows
        .iter()
        .find(|trigger| trigger.loc.index == 1)
        .expect("the optional add-counter ability");
    assert!(
        rows.iter().any(|trigger| trigger.loc.index == 0),
        "the each-upkeep sacrifice ability is the other trigger"
    );
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for the trigger order question")
    };
    let mut top_first = vec![add.handle];
    top_first.extend(cards.iter().copied().filter(|id| *id != add.handle));
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![top_first],
            },
        )
        .expect("the controller chooses the trigger order");
    let order: Vec<_> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .rev()
        .map(|id| engine.state().object(*id).unwrap().ability.unwrap().index)
        .collect();
    assert_eq!(order, vec![1, 0], "the optional counter is on top");
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } if *player == p0
        )
    });
    let Pending::YesNo {
        source: Some(source),
        ..
    } = engine.pending()
    else {
        unreachable!("the counter question names its source")
    };
    assert_eq!((source.card, source.index), (smokestack(), 1));
}

/// The controller may decline the soot counter. With zero counters, neither
/// player's upkeep asks for or causes any sacrifice.
#[test]
fn smokestack_declines_soot_and_nobody_sacrifices_at_zero() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[smokestack(), forest(), grizzly_bears()])
        .battlefield(1, &[forest(), grizzly_bears()])
        .start();
    // The opening upkeep already queues both abilities; order them from the outset.
    engine.house_rules.order_triggers = true;
    keep_mulligans(&mut engine);
    let stack = on_battlefield(&engine, p0, smokestack()).unwrap();

    offer_soot_before_sacrifice(&mut engine, stack);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(soot(&engine, stack), 0, "the counter is optional");
    assert!(on_battlefield(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(soot(&engine, stack), 0, "the opponent adds no counter");
    assert!(on_battlefield(&engine, p1, forest()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}

/// With the soot trigger ordered above the sacrifice trigger, the new counter
/// counts immediately. The active player sacrifices one at their upkeep; the
/// opponent then sacrifices one at theirs without being offered a counter.
#[test]
fn smokestack_new_soot_counts_for_both_players_upkeeps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[smokestack(), forest(), grizzly_bears()])
        .battlefield(1, &[forest(), grizzly_bears()])
        .start();
    // The opening upkeep already queues both abilities; order them from the outset.
    engine.house_rules.order_triggers = true;
    keep_mulligans(&mut engine);
    let stack = on_battlefield(&engine, p0, smokestack()).unwrap();
    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let my_land = on_battlefield(&engine, p0, forest()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let their_land = on_battlefield(&engine, p1, forest()).unwrap();

    offer_soot_before_sacrifice(&mut engine, stack);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    assert_eq!(soot(&engine, stack), 1, "the counter resolves first");
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the sacrifice trigger asks what to keep")
    };
    assert_eq!((min, max), (2, 2), "keep two of three, sacrifice one");
    assert!(options.contains(&stack) && options.contains(&my_bear) && options.contains(&my_land));
    assert!(!options.contains(&their_bear) && !options.contains(&their_land));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![stack, my_bear],
            },
        )
        .unwrap();

    // Only Smokestack's controller gets the optional counter. `pass_until`
    // answers a MayDo itself, so its absence here shows nothing. Soot at one
    // and a keep of (1, 1) show it: a second counter would leave the opponent
    // nothing to keep, and this walk would never reach the question. With
    // triggers ordered, a second trigger stops it sooner, at the order question.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    assert_eq!(soot(&engine, stack), 1);
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the opponent also chooses what to keep")
    };
    assert_eq!((min, max), (1, 1), "the opponent sacrifices exactly one");
    assert!(options.contains(&their_bear) && options.contains(&their_land));
    assert!(!options.contains(&stack) && !options.contains(&my_bear));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![their_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, smokestack()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert_eq!(soot(&engine, stack), 1, "the opponent added no counter");
    assert_eq!(engine.state().turn.active, p1);
}

/// Two soot counters require two permanents. Any permanent type is legal: the
/// opponent keeps only a Bears, sacrificing both a creature and a land.
#[test]
fn smokestack_two_soot_sacrifices_two_permanents_of_any_type() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[smokestack(), forest(), forest()])
        .battlefield(1, &[forest(), grizzly_bears(), hill_giant()])
        .start();
    // The opening upkeep already queues both abilities; order them from the outset.
    engine.house_rules.order_triggers = true;
    keep_mulligans(&mut engine);
    let stack = on_battlefield(&engine, p0, smokestack()).unwrap();
    set_counters(&mut engine, stack, counters::SOOT, 2);
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();

    offer_soot_before_sacrifice(&mut engine, stack);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    let Pending::ChooseCards { min, max, .. } = engine.pending().clone() else {
        unreachable!("the controller also sacrifices two")
    };
    assert_eq!((min, max), (1, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![stack],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("two counters require a keep-one choice from three permanents")
    };
    assert_eq!((min, max), (1, 1), "keep one, sacrifice two");
    assert!(options.contains(&bear) && options.contains(&giant) && options.contains(&land));
    assert!(
        !options.contains(&stack),
        "only the active player's permanents"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p1, hill_giant()).is_some());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(on_battlefield(&engine, p0, smokestack()).is_some());
    assert_eq!(soot(&engine, stack), 2);
}

/// The trigger exists apart from its source once it is on the stack
/// (CR 113.7a), and reads the soot as Smokestack last existed (CR 608.2h). A
/// Naturalize at the opponent's upkeep destroys Smokestack with the trigger
/// still waiting, and the opponent sacrifices the one all the same.
#[test]
fn smokestack_destroyed_in_response_still_counts_its_last_soot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[smokestack(), forest()])
        .battlefield(1, &[forest(), forest(), grizzly_bears()])
        .hand(1, &[naturalize()])
        .start();
    // The opening upkeep already queues both abilities; order them from the outset.
    engine.house_rules.order_triggers = true;
    keep_mulligans(&mut engine);
    let stack = on_battlefield(&engine, p0, smokestack()).unwrap();
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    offer_soot_before_sacrifice(&mut engine, stack);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![stack],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && !stack_is_empty(e)
    });
    cast_from_hand(&mut engine, p1, naturalize());
    aim_at(&mut engine, p1, stack);
    pass_until(&mut engine, |e| in_graveyard(e, p0, smokestack()).is_some());
    assert!(
        !stack_is_empty(&engine),
        "the trigger is still waiting with Smokestack gone"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let Pending::ChooseCards { min, max, .. } = engine.pending().clone() else {
        unreachable!("the opponent chooses what to keep")
    };
    assert_eq!((min, max), (2, 2), "keep two of three, sacrifice one");
    let land = on_battlefield(&engine, p1, forest()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![their_bear, land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert_eq!(
        engine.state().object(their_bear).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Battlefield);
}
