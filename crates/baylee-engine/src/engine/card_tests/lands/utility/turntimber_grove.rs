//! `cards/lands/utility/turntimber_grove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Turntimber Grove` is a utility land under `Coverage::Implemented` that enters tapped, triggers a +1/+1 pump on entry, and taps for `{G}`.
/// Playing the land puts it onto the battlefield tapped and triggers its target selection.
/// When targeting a creature, only the chosen creature receives +1/+1 until end of turn while bystanders remain untouched.
#[test]
fn turntimber_grove_enters_tapped_and_pumps_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[turntimber_grove()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(my_elves.len(), 2, "two friendly Elves deployed");
    let (target_elf, bystander) = (my_elves[0], my_elves[1]);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf deployed");

    assert_eq!(pt(&engine, target_elf), (1, 1), "starts as 1/1");

    let land = play_land(&mut engine, p0, turntimber_grove());
    assert!(
        entered_tapped(&engine, land),
        "Turntimber Grove enters tapped"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "controller of the land names the target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&target_elf)
            && options.contains(&bystander)
            && options.contains(&their_elf),
        "creatures on both sides are valid targets: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "the land itself is not a creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_elf],
            },
        )
        .expect("targeting friendly Elf is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target_elf),
        (2, 2),
        "targeted Elf received +1/+1, becoming 2/2"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "untargeted friendly Elf remains 1/1"
    );
    assert_eq!(pt(&engine, their_elf), (1, 1), "opponent's Elf remains 1/1");
    assert!(is_tapped(&engine, land), "Turntimber Grove remains tapped");
}
