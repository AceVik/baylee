//! `cards/lands/tapland/wooded_ridgeline.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Wooded Ridgeline` is a Mountain Forest tapland under `Coverage::Implemented` that enters tapped
/// and taps for `{R}` or `{G}`.
/// Playing the land enters it tapped, preventing activation on its entry turn.
/// On the subsequent turn after untapping, activating its mana ability prompts for a color choice
/// and adds the chosen mana directly to the pool without using the stack.
#[test]
fn wooded_ridgeline_enters_tapped_and_produces_red_or_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wooded_ridgeline()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, wooded_ridgeline());
    assert!(
        entered_tapped(&engine, land),
        "Wooded Ridgeline enters tapped"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "tapped land cannot activate its mana ability on entry turn"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, land),
        "Wooded Ridgeline untaps on next turn"
    );

    activate(&mut engine, p0, wooded_ridgeline(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses color");
    assert_eq!(options.len(), 2, "offers two colors");
    assert!(options.contains(&ManaColor::Red), "offers Red");
    assert!(options.contains(&ManaColor::Green), "offers Green");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("choosing Green is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green mana added to pool"
    );
    assert!(
        is_tapped(&engine, land),
        "Wooded Ridgeline is tapped after activation"
    );
}
