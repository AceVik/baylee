//! `cards/artifacts/mv_1/aether_vial.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Aether Vial` prints `At the beginning of your upkeep, you may put a charge counter on this artifact.` and `{{T}}: You may put a creature card with mana value equal to the number of charge counters on this artifact from your hand onto the battlefield.`
///
/// The upkeep trigger uses `Trigger::StepBegin` with `StepKind::Upkeep` to prompt via `Pending::YesNo` for `Effect::MayDo`, placing a `CounterKind::Charge`.
/// With one counter, the `{{T}}` ability puts the mana value 1 Elves from hand onto the battlefield and leaves the mana value 2 Spider where it is.
#[test]
fn aether_vial_adds_charge_counter_at_upkeep_and_puts_a_matching_creature_into_play() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aether_vial()])
        .hand(0, &[llanowar_elves(), canopy_spider()])
        .start();
    keep_mulligans(&mut engine);

    // The Vial is already on the battlefield, so its own upkeep trigger asks
    // its question on turn one, before anybody reaches a main phase.
    // Declined here, which is what makes the counter asserted below the one
    // the *next* upkeep put there.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let vial = on_battlefield(&engine, p0, aether_vial()).expect("aether vial on battlefield");
    assert_eq!(counters_on(&engine, vial, CounterKind::Charge), 0);

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });

    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!("expected YesNo prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    reach_main_phase(&mut engine, p0);
    assert_eq!(counters_on(&engine, vial, CounterKind::Charge), 1);
    assert!(!is_tapped(&engine, vial));

    // One charge counter: the Elves (mana value 1) may come in, the Spider
    // (2) may not, and nothing is paid for either.
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("the Elves wait in hand");
    activate(&mut engine, p0, aether_vial(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1), "\"you may put a creature card\"");
    assert_eq!(
        options,
        vec![elves],
        "mana value equal to one counter: not the Spider, not the land"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(in_hand(&engine, p0, canopy_spider()).is_some());
    assert!(is_tapped(&engine, vial));
}
