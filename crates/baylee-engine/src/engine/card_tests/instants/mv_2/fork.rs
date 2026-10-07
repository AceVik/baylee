//! `cards/instants/mv_2/fork.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fork: "Copy target instant or sorcery spell, except that the copy is
/// red. You may choose new targets for the copy." Only instant and sorcery
/// spells are legal targets — a creature spell on the stack underneath the
/// Giant Growth this test points Fork at is never offered. "May" (CR
/// 707.10c: "The player may leave any number of the targets unchanged")
/// means an empty answer retains the original target even though
/// Fork's own caster goes on to choose a different creature (CR 707.10c),
/// and both effects land independently: the original's own target gets
/// its own +3/+3 and the copy's new target gets its own, which is only
/// true if the retargeting actually moved the copy's aim rather than
/// leaving it on the original's creature.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn fork_copies_an_instant_and_its_caster_retargets_the_copy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), swamp(), forest()])
        .hand(0, &[festering_goblin(), giant_growth()])
        .battlefield(1, &[quiet_creature(), mountain(), mountain()])
        .hand(1, &[fork()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let qc_a = on_battlefield(&engine, p0, quiet_creature()).expect("p0's creature is seated");
    let qc_b = on_battlefield(&engine, p1, quiet_creature()).expect("p1's creature is seated");
    let (base_a, base_b) = (pt(&engine, qc_a), pt(&engine, qc_b));

    tap_all_mana(&mut engine, p0);
    // A creature spell first, so something that is neither instant nor
    // sorcery sits on the stack beneath the one Fork actually points at.
    cast_with_floating(&mut engine, p0, festering_goblin());
    let goblin_spell = top_of_stack(&engine);

    // Still p0's own priority: Giant Growth targets their own creature and
    // stacks above the Goblin.
    cast_with_floating(&mut engine, p0, giant_growth());
    let gg_options = aim_at(&mut engine, p0, qc_a);
    assert!(
        gg_options.contains(&qc_a) && gg_options.contains(&qc_b),
        "\"target creature\" is not \"target creature you control\": {gg_options:?}"
    );
    let growth_spell = top_of_stack(&engine);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, fork());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Fork asks for a target spell, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "Fork's own caster names its target");
    assert!(
        options.contains(&growth_spell),
        "an instant on the stack is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&goblin_spell),
        "a creature spell is neither instant nor sorcery: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![growth_spell],
            },
        )
        .expect("the instant was on the menu");

    // Both players still have to pass priority for Fork to resolve.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    // Fork resolves; the copy is already on the stack, above the
    // original, asking its controller to retarget it (CR 707.10c).
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the copy asks to be retargeted, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the copy's controller picks its new target");
    assert_eq!((min, max), (0, 1), "one target; an empty answer keeps it");
    let copy = top_of_stack(&engine);
    assert_ne!(
        copy, growth_spell,
        "the copy is a new object, not the original"
    );
    let copy_colors = engine
        .state()
        .object(copy)
        .expect("the copy is on the stack")
        .characteristics()
        .colors;
    assert_eq!(
        copy_colors,
        ColorSet::of(Color::Red),
        "\"except that the copy is red\" — not Giant Growth's own green"
    );
    assert!(
        options.contains(&qc_b),
        "the other creature is a legal new target: {options:?}"
    );
    assert!(
        !options.contains(&qc_a),
        "the original target is kept by an empty answer: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![qc_b],
                players: vec![],
            },
        )
        .expect("the retarget names a legal creature");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, qc_b),
        (base_b.0 + 3, base_b.1 + 3),
        "the copy's new target got its own +3/+3"
    );
    assert_eq!(
        pt(&engine, qc_a),
        (base_a.0 + 3, base_a.1 + 3),
        "the original spell still resolved on the target it was cast at, \
         not doubled up by the retargeted copy"
    );
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "the creature spell beneath both was never Fork's target"
    );
}
