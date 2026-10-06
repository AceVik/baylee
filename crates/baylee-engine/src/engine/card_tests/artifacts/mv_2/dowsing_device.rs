//! `cards/artifacts/mv_2/dowsing_device.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dowsing Device` // `Geode Grotto` (`Coverage::Partial`): "Whenever this artifact or another
/// artifact you control enters, up to one target creature you control gets +1/+0 and gains
/// haste until end of turn. Then transform this artifact if you control four or more artifacts.
/// // {T}: Add {R}. {2}{R}, {T}: Until end of turn, target creature gains haste and gets +X/+0,
/// where X is the number of artifacts you control. Activate only as a sorcery."
///
/// Under `Coverage::Partial`, the conditional transform clause is omitted, while the ETB trigger
/// targeting up to one creature you control for +1/+0 and `KeywordSet::HASTE` is implemented.
/// The test casts `Dowsing Device` from hand, targets a controlled creature, verifies that the
/// opponent's creature cannot be targeted via `Filter::YOUR_CREATURE`, confirms the +1/+0 pump
/// and granted haste, and verifies that `Dowsing Device` remains on face 0.
#[test]
fn dowsing_device_triggers_on_entry_to_pump_and_grant_haste_to_controlled_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(146, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dowsing_device()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls an elf");

    assert_eq!(pt(&engine, my_elf), (1, 1), "elf starts as a 1/1");
    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::HASTE),
        "elf does not have haste yet"
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, dowsing_device());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent's creature is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_elf),
        (2, 1),
        "controlled creature received +1/+0 pump"
    );
    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::HASTE),
        "controlled creature gained haste"
    );

    let device = on_battlefield(&engine, p0, dowsing_device()).expect("device on battlefield");
    assert_eq!(
        engine.state().object(device).map(|o| o.face_index),
        Some(0),
        "device remains on front face 0 without the transform clause"
    );
}
