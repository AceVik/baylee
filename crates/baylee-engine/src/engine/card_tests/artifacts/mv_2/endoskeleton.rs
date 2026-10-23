#[allow(clippy::wildcard_imports)]
use super::*;

fn endoskeleton() -> CardIndex {
    card_index("e9ef7f8c-21ef-45f0-af64-8ffa9f0089d1")
}

#[test]
fn endoskeleton_grants_plus_zero_plus_three_while_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[endoskeleton(), plains(), plains(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, endoskeleton(), 1);
    aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (1, 4));
    let source = on_battlefield(&engine, p0, endoskeleton()).unwrap();
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, elves), (1, 4));
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, source));
    assert_eq!(pt(&engine, elves), (1, 4));
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, source));
    assert_eq!(pt(&engine, elves), (1, 1));
}
