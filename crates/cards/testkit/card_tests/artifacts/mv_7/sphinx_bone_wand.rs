//! `cards/artifacts/mv_7/sphinx_bone_wand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sphinx_bone_wand() -> CardIndex {
    card_index("4b9e529c-4fbd-4944-828d-7fbc8f26c991")
}

/// "Whenever you cast an instant or sorcery spell, you may put a charge
/// counter on this artifact. If you do, this artifact deals damage equal to
/// the number of charge counters on it to any target."
///
/// The counter is the wand's and the target takes the damage. The first
/// reading put the counter on the chosen target and dealt no damage at all.
#[test]
fn sphinx_bone_wand_charges_itself_and_shoots_the_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(404, swamp())
        .battlefield(0, &[sphinx_bone_wand(), swamp()])
        .hand(0, &[dark_ritual()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wand = on_battlefield(&engine, p0, sphinx_bone_wand()).expect("the wand is out");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the angel is out");

    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, wand, CounterKind::Charge),
        1,
        "the charge counter is on the wand"
    );
    assert_eq!(
        counters_on(&engine, angel, CounterKind::Charge),
        0,
        "and not on the creature it was aimed at"
    );
    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        1,
        "the target takes damage equal to the counters on the wand"
    );
}
