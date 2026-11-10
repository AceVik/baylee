//! `cards/lands/utility/teetering_peaks.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Teetering Peaks is a land under `Coverage::Implemented` that enters tapped, triggers a +2/+0 pump on entry, and taps for {R}.
/// When played from hand, it enters tapped and places its enters-the-battlefield trigger on the stack.
/// The trigger requires targeting a creature, offering creatures on either side of the table while excluding lands.
/// Upon resolution, only the targeted creature receives +2/+0 until end of turn.
#[test]
fn teetering_peaks_enters_tapped_and_pumps_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[teetering_peaks()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(my_elves.len(), 2, "two Elves controlled by p0");
    let (target_elf, bystander) = (my_elves[0], my_elves[1]);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(pt(&engine, target_elf), (1, 1), "starts as 1/1");

    let land = play_land(&mut engine, p0, teetering_peaks());
    assert!(
        entered_tapped(&engine, land),
        "Teetering Peaks enters tapped"
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
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "controller of the land names the target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&target_elf)
            && options.contains(&bystander)
            && options.contains(&their_elf),
        "any creature is a legal target: {options:?}"
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
        .expect("targeting the Elf is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target_elf),
        (3, 1),
        "targeted Elf received +2/+0, becoming 3/1"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "untargeted friendly creature is untouched"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "opponent creature is untouched"
    );
    assert!(is_tapped(&engine, land), "Teetering Peaks remains tapped");
}
