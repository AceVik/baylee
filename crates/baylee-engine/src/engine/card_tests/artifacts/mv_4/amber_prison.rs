//! `cards/artifacts/mv_4/amber_prison.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn amber_prison() -> CardIndex {
    card_index("1c69fdcf-ba87-480a-88df-70aa4ec9fff0")
}

#[test]
fn amber_prison_taps_and_locks_down_while_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[amber_prison(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amber_prison(), 1);
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "Amber Prison should tap the target permanent"
    );

    // Advance to p1's first main phase; target should remain tapped.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "target should not untap while Amber Prison remains tapped"
    );
}

const UNLIKELY_ALLIANCE: &str = "f7ac18bc-ea63-4808-b709-92c1e69cf541";

#[test]
fn amber_prison_targets_artifacts_creatures_and_lands_but_not_enchantments() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let enchantment = card_index(UNLIKELY_ALLIANCE);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[amber_prison(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS, plains(), enchantment])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, p1, plains()).unwrap();
    let enchantment_id = on_battlefield(&engine, p1, enchantment).unwrap();
    let prison = on_battlefield(&engine, p0, amber_prison()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amber_prison(), 1);
    let options = aim_at(&mut engine, p0, land);
    assert!(options.contains(&bear), "a creature");
    assert!(options.contains(&land), "a land");
    assert!(options.contains(&prison), "an artifact");
    assert!(
        !options.contains(&enchantment_id),
        "an enchantment is none of them"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, land));
}

#[test]
fn amber_prison_lock_ends_when_it_untaps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[amber_prison(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let prison = on_battlefield(&engine, p0, amber_prison()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amber_prison(), 1);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    // `reach_their_main_phase(p0)` would return at once: it is p0's main
    // phase already. Cross p1's turn first, with the Prison still tapped ...
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, prison));
    assert!(
        is_tapped(&engine, bear),
        "locked while the Prison is tapped"
    );
    // ... then p0's next turn: the untap step untaps the Prison (the walk
    // answers "leave tapped" with nothing) ...
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, prison));
    assert!(
        is_tapped(&engine, bear),
        "still tapped until its own untap step"
    );
    // ... so the Bears are free at p1's next untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, bear),
        "the lock lasts only as long as Amber Prison remains tapped"
    );
}

#[test]
fn amber_prison_keeps_its_lock_when_its_controller_chooses_not_to_untap_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[amber_prison(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let prison = on_battlefield(&engine, p0, amber_prison()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, amber_prison(), 1);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    // p0's untap step asks which permanents to leave tapped: the Prison.
    let mut asked = false;
    for _ in 0..100 {
        if let Pending::ChooseCards {
            options,
            prompt: crate::choice::ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            assert!(options.contains(&prison));
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![prison],
                    },
                )
                .unwrap();
            asked = true;
            break;
        }
        let (player, action) = answer_one(&engine).expect("walk to untap");
        engine.apply(player, action).unwrap();
    }
    assert!(asked, "the untap step offers to leave Amber Prison tapped");
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, prison));
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, bear), "still locked down");
}
