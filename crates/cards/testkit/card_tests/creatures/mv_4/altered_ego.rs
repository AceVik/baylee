//! `cards/creatures/mv_4/altered_ego.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Altered Ego` prints `This spell can't be countered.` and `You may have this creature enter as a copy of any creature on the battlefield, except it enters with X additional +1/+1 counters on it.`
///
/// It carries `KeywordSet::UNCOUNTERABLE`: a `counterspell()` may still point at it and resolves without countering it (#243).
/// Through `AbilityDef::CopyOnEnter`, it enters copying `young_wolf()` with the X = 1
/// announced for the spell as one additional `CounterKind::P1P1` counter (CR 107.3m).
/// This test pinned the missing counters until `CopyMod::AddCounterX` said them.
#[test]
fn altered_ego_is_uncounterable_and_copies_creature_with_x_counters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                young_wolf(),
            ],
        )
        .hand(0, &[altered_ego()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf deployed");
    assert_eq!(pt(&engine, wolf), (1, 1));

    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, altered_ego()).expect("altered ego in hand");
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();

    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber prompt, got {:?}", engine.pending());
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let cs = in_hand(&engine, p1, counterspell()).expect("counterspell in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .expect("Altered Ego is a legal target for Counterspell");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .expect("Counterspell points at Altered Ego");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, counterspell()).is_some()
    });

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&wolf));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wolf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let ego = on_battlefield(&engine, p0, altered_ego()).expect("altered ego on battlefield");
    assert_ne!(ego, wolf);
    assert_eq!(
        pt(&engine, ego),
        (2, 2),
        "a copy of the 1/1 Young Wolf with one additional +1/+1 counter"
    );
    assert_eq!(
        counters_on(&engine, ego, CounterKind::P1P1),
        1,
        "X was announced as 1, so one additional counter"
    );
}

/// The counters are the copy's "except" and nothing else's: an Altered Ego
/// that declines to copy is the 0/0 it prints, whatever X was, and dies to
/// the state-based action (CR 704.5f).
#[test]
fn altered_ego_that_copies_nothing_gets_no_counters_and_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                young_wolf(),
            ],
        )
        .hand(0, &[altered_ego()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, altered_ego()).expect("altered ego in hand");
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected ChooseNumber prompt, got {:?}", engine.pending());
    };
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { min, .. } = engine.pending().clone() else {
        unreachable!("the predicate above matched a target choice")
    };
    assert_eq!(
        min, 0,
        "\"you may have\" — naming nothing declines the copy"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, altered_ego()).is_none(),
        "a 0/0 with no counters does not survive"
    );
    assert!(in_graveyard(&engine, p0, altered_ego()).is_some());
}
