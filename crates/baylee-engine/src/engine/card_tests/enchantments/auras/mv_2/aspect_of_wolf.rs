//! `cards/enchantments/auras/mv_2/aspect_of_wolf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aspect of Wolf's Enchant line: offered only a creature, never Sol Ring,
/// and ends attached to the Elves; its +X/+Y from Forests you control is
/// played in the tests near the end of this file.
#[test]
fn aspect_of_wolf_attaches_only_to_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), sol_ring()])
        .hand(0, &[aspect_of_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    let rock = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    attaches_only_to(&mut engine, p0, aspect_of_wolf(), elf, rock);
}

// ---------------------------------------------------------------------------
// Aspect of Wolf.
// ---------------------------------------------------------------------------

/// Aspect of Wolf: "Enchanted creature gets +X/+Y, where X is half the
/// number of Forests you control, rounded down, and Y is half the number
/// of Forests you control, rounded up." Three Forests (a Mountain beside
/// them that must not count) give the Elves +1/+2 on a 1/1; a fourth
/// Forest played through the engine brings it to +2/+2.
#[test]
fn aspect_of_wolf_scales_with_its_controllers_forests() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), mountain(), llanowar_elves()],
        )
        .hand(0, &[aspect_of_wolf(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    assert_eq!(pt(&engine, elf), (1, 1), "printed 1/1, before the Aura");

    cast_from_hand(&mut engine, p0, aspect_of_wolf());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Enchant creature asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elves is a legal \"enchant creature\" target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (2, 3),
        "three Forests give +1/+2 (floor(3/2)=1, ceil(3/2)=2) on a 1/1; \
         the Mountain beside them does not count"
    );

    play_land(&mut engine, p0, forest());
    assert_eq!(
        pt(&engine, elf),
        (3, 3),
        "a fourth Forest: +2/+2 (floor(4/2)=ceil(4/2)=2)"
    );
}

/// Aspect of Wolf's "Forests you control" reads its own controller's, not
/// the enchanted creature's controller's: cast at an opponent's creature,
/// the bonus follows the caster's two Forests rather than the four Forests
/// on the other side of the table, which would give a larger bonus.
#[test]
fn aspect_of_wolf_reads_its_controllers_forests_not_the_enchanted_creatures_controllers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[aspect_of_wolf()])
        .battlefield(
            1,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves is seated");
    assert_eq!(pt(&engine, elf), (1, 1), "printed 1/1, before the Aura");

    cast_from_hand(&mut engine, p0, aspect_of_wolf());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Enchant creature asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "an opponent's creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elves is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elf),
        (2, 2),
        "the Aura's controller's two Forests give +1/+1 \
         (floor(2/2)=ceil(2/2)=1); the enchanted creature's controller's \
         four Forests, which would give +2/+2, do not apply"
    );
}
