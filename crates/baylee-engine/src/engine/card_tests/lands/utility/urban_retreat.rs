//! `cards/lands/utility/urban_retreat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Urban Retreat` prints `This land enters tapped.`, `{{T}}: Add {{G}}, {{W}}, or {{U}}.`, and `{{2}}, Return a tapped creature you control to its owner's hand: Put this card from your hand onto the battlefield. Activate only as a sorcery.`
///
/// Under `Coverage::Partial`, `EnterModifier::Tapped` and the three-color mana ability are implemented, while the hand-zone activation is omitted because putting the source card from hand onto the battlefield has no DSL effect.
/// Playing `Urban Retreat` from hand enters tapped. After untapping on the next turn, activating ability 0 prompts via `Pending::ChooseColor` between green, white, and blue, adds one blue mana, and leaves `Urban Retreat` tapped.
#[test]
fn urban_retreat_enters_tapped_and_taps_for_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[urban_retreat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let retreat = play_land(&mut engine, p0, urban_retreat());
    assert!(
        entered_tapped(&engine, retreat),
        "urban retreat enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, retreat));

    activate(&mut engine, p0, urban_retreat(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![ManaColor::Green, ManaColor::White, ManaColor::Blue]
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, retreat));
}
