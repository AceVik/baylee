//! `cards/lands/caves/volatile_fault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volatile Fault prints `{{T}}: Add {{C}}` and `{{1}}, {{T}}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it onto the battlefield, then shuffle. You create a Treasure token.`
/// Activating the sacrifice ability destroys the opponent's nonbasic land, allows them to search for a basic land, and creates a Treasure token for the activator.
#[test]
fn volatile_fault_destroys_opponent_nonbasic_and_creates_treasure() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[volatile_fault(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, badlands()).expect("Badlands deployed");
    let fault = on_battlefield(&engine, p0, volatile_fault()).expect("Fault deployed");

    tap_mana_except(&mut engine, p0, fault);
    activate(&mut engine, p0, volatile_fault(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&target));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, volatile_fault()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected search");
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, badlands()).is_some());
    assert_eq!(tokens_of(&engine, p0).len(), 1);
}

#[test]
fn volatile_faults_basic_land_arrives_untapped_for_the_target_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[volatile_fault(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana_but(&mut e, p0, Some(volatile_fault()));
    activate(&mut e, p0, volatile_fault(), 1);
    let land = on_battlefield(&e, p1, badlands()).unwrap();
    aim_at(&mut e, p0, land);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, p1);
    let found = options[0];
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(found).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(found).unwrap().controller, p1);
    assert!(!is_tapped(&e, found));
    assert_eq!(tokens_of(&e, p0).len(), 1);
}

/// "Destroy target nonbasic land an opponent controls": the menu is the
/// opponent's nonbasic land and nothing else — not the opponent's basic
/// Forest, not the activator's own nonbasic land (Treetop Village), not the
/// activator's basic Forest and not the Fault itself.
#[test]
fn volatile_fault_offers_only_an_opponents_nonbasic_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[volatile_fault(), forest(), treetop_village()])
        .battlefield(1, &[badlands(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let fault = on_battlefield(&e, p0, volatile_fault()).expect("the Fault");
    let theirs = on_battlefield(&e, p1, badlands()).expect("Badlands");

    tap_mana_except(&mut e, p0, fault);
    activate(&mut e, p0, volatile_fault(), 1);

    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice, got {:?}", e.pending())
    };
    assert_eq!(
        options,
        vec![theirs],
        "only Badlands: not the opponent's Forest (basic), nor the activator's \
         own Treetop Village (not an opponent's), Forest or the Fault itself"
    );
}
