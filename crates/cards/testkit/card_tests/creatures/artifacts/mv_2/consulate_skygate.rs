//! `cards/creatures/artifacts/mv_2/consulate_skygate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn consulate_skygate() -> CardIndex {
    card_index("671db0d7-b103-4c13-9914-a444f2ecf9d6")
}

/// Consulate Skygate: a 0/4 artifact creature with "Defender" and "Reach".
/// Cast for `{2}`, it carries both keywords and, unlike the Bears beside it,
/// is never offered as an attacker.
#[test]
fn consulate_skygate_has_defender_and_reach_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[consulate_skygate()],
        &[forest(), forest(), grizzly_bears()],
        &[],
    );
    let gate = cast_and_land(&mut engine, consulate_skygate());
    assert_eq!(pt(&engine, gate), (0, 4));
    assert!(keywords(&engine, gate).contains(KeywordSet::DEFENDER));
    assert!(keywords(&engine, gate).contains(KeywordSet::REACH));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert!(attackers.contains(&bears), "the control may attack");
    assert!(!attackers.contains(&gate), "defender may not");
}

/// Reach lets it block a flier that a plain creature cannot block.
#[test]
fn consulate_skygate_blocks_a_flier_and_the_bears_cannot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[consulate_skygate(), grizzly_bears()],
        &[storm_crow()],
    );
    let gate = on_battlefield(&engine, p0, consulate_skygate()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    reach_their_main_phase(&mut engine, p1);
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, crow, p0);
    assert!(
        options
            .iter()
            .any(|o| o.blocker == gate && o.attackers.contains(&crow)),
        "reach may block a flier: {options:?}"
    );
    assert!(
        !options
            .iter()
            .any(|o| o.blocker == bears && o.attackers.contains(&crow)),
        "a plain creature may not: {options:?}"
    );
}
