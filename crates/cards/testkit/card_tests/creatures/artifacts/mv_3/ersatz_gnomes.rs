//! `cards/creatures/artifacts/mv_3/ersatz_gnomes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P1: PlayerId = PlayerId::new(1);

const P0: PlayerId = PlayerId::new(0);

fn ersatz_gnomes() -> CardIndex {
    card_index("ea016b84-7f77-42e7-bd14-2e62a1e35f3b")
}

/// Ersatz Gnomes (ea016b84-7f77-42e7-bd14-2e62a1e35f3b) — {3} 1/1 Artifact Creature:
/// - "{T}: Target spell becomes colorless."
/// - "{T}: Target permanent becomes colorless until end of turn."
#[test]
fn ersatz_gnomes_makes_target_permanent_colorless_until_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[ersatz_gnomes(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );

    activate(&mut engine, P0, ersatz_gnomes(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );
}

/// "Target permanent becomes colorless until end of turn": the colour is back
/// on the opponent's turn.
#[test]
fn ersatz_gnomes_colour_returns_after_the_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[ersatz_gnomes(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    activate(&mut engine, P0, ersatz_gnomes(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );
}

/// "{T}: Target spell becomes colorless.": a Grizzly Bears spell on the stack
/// is colourless once the first ability has resolved.
#[test]
fn ersatz_gnomes_makes_a_spell_on_the_stack_colourless() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ersatz_gnomes(), forest(), forest()])
        .hand(0, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, GRIZZLY_BEARS);
    let spell = on_stack(&engine, GRIZZLY_BEARS).unwrap();
    assert!(
        !engine
            .state()
            .object(spell)
            .unwrap()
            .characteristics()
            .colors
            .is_empty(),
        "a green spell to begin with"
    );

    activate(&mut engine, P0, ersatz_gnomes(), 0);
    aim_at(&mut engine, P0, spell);
    // The ability resolves; the Bears spell is still on the stack.
    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).len() == 1
    });

    assert!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );
}
