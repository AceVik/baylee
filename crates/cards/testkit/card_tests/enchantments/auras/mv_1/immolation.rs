//! `cards/enchantments/auras/mv_1/immolation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn immolation_gives_the_enchanted_creature_two_power_and_takes_two_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), rootbreaker_wurm(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[immolation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let hill = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the Aura");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and a printed 1/1 beside it"
    );

    cast_from_hand(&mut engine, p0, immolation());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("an Aura asks what it enchants, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster chooses");
    assert!(
        options.contains(&wurm) && options.contains(&bystander) && options.contains(&theirs),
        "\"enchant creature\" reaches any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&hill),
        "a Mountain is a permanent and no creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the offered targets");
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, immolation()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(wurm),
        "it entered attached to the creature it targeted"
    );
    assert_eq!(
        pt(&engine, wurm),
        (8, 4),
        "+2/-2 on the creature it holds: both numbers move, in opposite directions"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static reaches the equipped creature's table and no further"
    );
}
