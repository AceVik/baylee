//! `cards/creatures/mv_7/veldrane_of_sengir.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn veldrane_of_sengir() -> CardIndex {
    card_index("39322103-c40e-4d05-8e87-a92d1cbc7b68")
}

/// Veldrane of Sengir — a legendary 5/5 for {5}{B}{B}.
#[test]
fn veldrane_of_sengir_costs_seven_and_is_a_five_five() {
    let mut engine = table7(&lots(swamp(), 7), &[veldrane_of_sengir()], &[]);
    let veldrane = cast_and_resolve(&mut engine, veldrane_of_sengir());
    assert_body(&engine, veldrane, (5, 5), KeywordSet::default());
}

/// "{1}{B}{B}: Veldrane gets -3/-0 and gains forestwalk until end of turn."
/// Before the ability the opposing Ogre may block it; after, it is a 2/5 with
/// forestwalk and the defender controls a Forest, so the Ogre may not.
#[test]
fn veldrane_of_sengir_gets_forestwalk_for_three_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(
        &[veldrane_of_sengir(), swamp(), swamp(), swamp()],
        &[],
        &[forest(), gray_ogre()],
    );
    let veldrane = on_battlefield(&engine, p0, veldrane_of_sengir()).expect("out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    assert!(!keywords(&engine, veldrane).contains(KeywordSet::FORESTWALK));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, veldrane_of_sengir(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, veldrane), (2, 5), "-3/-0");
    assert!(keywords(&engine, veldrane).contains(KeywordSet::FORESTWALK));

    let blocks = attack_and_collect_blocks(&mut engine, veldrane, p1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == ogre && b.attackers.contains(&veldrane)),
        "forestwalk against a Forest controller: unblockable"
    );
}

/// The control: without the ability the Ogre may block.
#[test]
fn veldrane_of_sengir_is_blockable_without_forestwalk() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[veldrane_of_sengir()], &[], &[forest(), gray_ogre()]);
    let veldrane = on_battlefield(&engine, p0, veldrane_of_sengir()).expect("out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    let blocks = attack_and_collect_blocks(&mut engine, veldrane, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == ogre && b.attackers.contains(&veldrane))
    );
}
