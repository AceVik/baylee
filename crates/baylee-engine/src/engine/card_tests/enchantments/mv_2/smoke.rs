//! `cards/enchantments/mv_2/smoke.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smoke: "Players can't untap more than one creature during their untap
/// steps." Three tapped creatures offer exactly one to untap; the other two
/// stay tapped, and a tapped land beside them — not a creature — untaps on
/// its own.
#[test]
fn smoke_limits_untapping_to_one_creature_and_never_touches_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                smoke(),
                quiet_creature(),
                quiet_creature(),
                quiet_creature(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let creatures = all_on_battlefield(&engine, p0, quiet_creature());
    assert_eq!(creatures.len(), 3, "three creatures are seated");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for &c in &creatures {
            state
                .object_mut(c)
                .expect("seated")
                .status
                .insert(Status::TAPPED);
        }
        state
            .object_mut(land)
            .expect("seated")
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();
    assert!(creatures.iter().all(|&c| is_tapped(&engine, c)));
    assert!(is_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::Untap,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "the active player determines untapping");
    assert_eq!(options.len(), 3, "the three tapped creatures are the menu");
    for c in &creatures {
        assert!(options.contains(c), "every tapped creature is on offer");
    }
    assert!(!options.contains(&land), "the land is not a creature");
    assert_eq!((min, max), (1, 1), "exactly one creature may untap");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creatures[0]],
            },
        )
        .unwrap();
    assert!(
        !is_tapped(&engine, creatures[0]),
        "the named creature untapped"
    );
    assert!(
        is_tapped(&engine, creatures[1]) && is_tapped(&engine, creatures[2]),
        "Smoke's limit kept the other two tapped"
    );
    assert!(
        !is_tapped(&engine, land),
        "Smoke counts only creatures: the land untapped on its own"
    );
}
