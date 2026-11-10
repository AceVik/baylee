//! `cards/lands/utility/plaza_of_harmony.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plaza of Harmony prints `When this land enters, if you control two or more Gates, you gain 3 life`,
/// `{T}: Add {C}`, and `{T}: Add one mana of any type that a Gate you control could produce.`
/// The card is marked `Coverage::Partial` because mana of a type produced by a Gate is not supported.
/// When entering while controlling two `gond_gate` permanents, the enters-the-battlefield trigger fires,
/// gaining 3 life upon resolution, and the land taps for `{C}` at ability index 1.
#[test]
fn plaza_of_harmony_gains_life_with_two_gates_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gond_gate(), gond_gate()])
        .hand(0, &[plaza_of_harmony()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, plaza_of_harmony());
    assert!(!stack_is_empty(&engine));

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, life_before + 3);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(legal.abilities.contains(&(land, 1)));

    activate(&mut engine, p0, plaza_of_harmony(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
