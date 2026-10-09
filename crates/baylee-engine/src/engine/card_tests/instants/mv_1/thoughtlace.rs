//! `cards/instants/mv_1/thoughtlace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thoughtlace: "Target spell or permanent becomes blue." Read off a
/// permanent, empirically: the color changes on the object targeted, not on
/// Thoughtlace's own (already-left-play) source.
#[test]
fn thoughtlace_turns_its_target_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), island()])
        .hand(0, &[thoughtlace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().colors,
        ColorSet::of(Color::Green),
        "green as printed, before"
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, thoughtlace());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("its own Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(elf).unwrap().characteristics().colors,
        ColorSet::of(Color::Blue),
        "\"becomes blue\" — the target, not the spell's own source"
    );
}

/// Thoughtlace: "Target spell or permanent becomes blue." Aimed at a
/// creature spell on the stack, which the opponent answers it with: the
/// spell turns blue while it waits, keeps its mana cost, and the creature
/// it resolves into is still blue.
#[test]
fn thoughtlace_turns_a_spell_on_the_stack_blue_and_the_permanent_stays_blue() {
    a_lace_recolours_a_spell(
        thoughtlace(),
        island(),
        llanowar_elves(),
        forest(),
        Color::Green,
        Color::Blue,
    );
}
