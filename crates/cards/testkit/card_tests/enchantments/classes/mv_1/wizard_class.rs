//! `cards/enchantments/classes/mv_1/wizard_class.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wizard Class keeps an oversized hand, draws two when it becomes level 2,
/// and gives one creature you control a counter whenever you draw at level 3.
#[test]
fn wizard_class_draws_two_at_level_two_and_rewards_later_draws_at_level_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wizard = card_index("36f68aa3-9955-46f1-bc87-497f16ef5222");
    let mut battlefield = vec![wizard, grizzly_bears()];
    battlefield.extend([island(); 8]);
    let mut engine = Duel::new(919, forest())
        .battlefield(0, &battlefield)
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[forest(); 6])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, wizard).expect("the Class is on the table");
    let ours = on_battlefield(&engine, p0, grizzly_bears()).expect("our Bear is in play");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bear is in play");
    engine
        .state
        .object_mut(class)
        .unwrap()
        .counters
        .set(CounterKind::Level, 1);
    let level = |e: &Engine<RegistryLookup>| e.state().object(class).unwrap().class_level();
    let hand_size = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_except(&mut engine, p0, class);
    let before = hand_size(&engine);
    assert_eq!(before, 6);
    assert_eq!(level(&engine), 1);
    assert!(ability_offered(&engine, class, 1));
    assert!(!ability_offered(&engine, class, 3));

    activate(&mut engine, p0, wizard, 1);
    pass_until(&mut engine, |e| level(e) == 2);
    assert_eq!(
        hand_size(&engine),
        before,
        "the draw trigger is still on the stack"
    );
    assert!(!stack_is_empty(&engine), "becoming level 2 triggered");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine), before + 2);
    assert_eq!(counters_on(&engine, ours, CounterKind::P1P1), 0);
    assert!(!ability_offered(&engine, class, 1));

    activate(&mut engine, p0, wizard, 3);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(level(&engine), 3);
    assert_eq!(counters_on(&engine, class, CounterKind::Level), 1);
    assert!(!ability_offered(&engine, class, 3));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        hand_size(&engine),
        before + 2,
        "the Class removes the maximum hand size"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(options.contains(&ours));
    assert!(!options.contains(&theirs));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ours],
                players: vec![],
            },
        )
        .expect("your own creature is a legal target after you draw");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, ours, CounterKind::P1P1), 1);
}
