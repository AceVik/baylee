//! `cards/instants/mv_1/purelace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Purelace changes a spell's color and keeps that change when it becomes
/// a permanent (CR 400.7a), without changing its green mana ability.
#[test]
fn alpha_eval_purelace_whitens_a_spell_through_resolution() {
    use baylee_core::color::{Color, ColorSet};
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let lace = card_index("3773001a-8868-49ec-a406-298cf72359c2");
    let mut engine = Duel::new(1002, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[plains()])
        .hand(0, &[llanowar_elves()])
        .hand(1, &[lace])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, llanowar_elves());
    let spell = on_stack(&engine, llanowar_elves()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, lace);
    aim_at(&mut engine, p1, spell);
    pass_until(&mut engine, |e| on_stack(e, lace).is_none());
    let white = ColorSet::from_slice(&[Color::White]);
    assert_eq!(
        engine
            .state()
            .object(spell)
            .unwrap()
            .characteristics()
            .colors,
        white
    );
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().colors,
        white
    );
    // Wait through summoning sickness; the indefinite color change persists.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().colors,
        white
    );
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
