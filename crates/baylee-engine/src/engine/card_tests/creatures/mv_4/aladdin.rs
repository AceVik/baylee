//! `cards/creatures/mv_4/aladdin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aladdin — `{2}{R}{R}` 1/1: "{1}{R}{R}, {T}: Gain control of target
/// artifact for as long as you control this creature."
///
/// Two Aladdins take one Sol Ring each in one main phase, which is the
/// 2004-10-04 ruling ("Aladdin's ability can take control of more than one
/// artifact, although only one each time the ability is used"). The control
/// is a layer-2 effect bound to the Aladdin that made it (CR 613.1b, the
/// whole reason for "for as long as you control this creature"): destroying
/// the first one hands its Ring back while the second Ring stays stolen, and
/// destroying the second returns that one too.
#[test]
fn aladdin_steals_one_artifact_per_activation_and_gives_each_back_when_he_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                aladdin(),
                aladdin(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[sol_ring(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rings = all_on_battlefield(&engine, p1, sol_ring());
    assert_eq!(rings.len(), 2, "two artifacts to take");
    let controller = |e: &Engine<RegistryLookup>, id: ObjectId| {
        e.state()
            .object(id)
            .expect("the artifact is out")
            .controller
    };
    assert_eq!(
        controller(&engine, rings[0]),
        p1,
        "the first starts at home"
    );
    assert_eq!(controller(&engine, rings[1]), p1, "and so does the second");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, aladdin(), 0);
    aim_at(&mut engine, p0, rings[0]);
    activate(&mut engine, p0, aladdin(), 0);
    aim_at(&mut engine, p0, rings[1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        controller(&engine, rings[0]),
        p0,
        "the first activation took its artifact"
    );
    assert_eq!(
        controller(&engine, rings[1]),
        p0,
        "and the second activation took the other"
    );

    let first_thief = all_on_battlefield(&engine, p0, aladdin())
        .into_iter()
        .find(|id| is_tapped(&engine, *id))
        .expect("the Aladdin that activated first is the tapped one");
    kill(&mut engine, first_thief);
    assert_eq!(
        controller(&engine, rings[0]),
        p1,
        "the first steal ended when its Aladdin left"
    );
    assert_eq!(
        controller(&engine, rings[1]),
        p0,
        "the second Aladdin still holds its own"
    );

    let second_thief = all_on_battlefield(&engine, p0, aladdin())[0];
    kill(&mut engine, second_thief);
    assert_eq!(
        controller(&engine, rings[1]),
        p1,
        "and the last artifact went home too"
    );
}
