//! `cards/creatures/artifacts/mv_2/mandible_justiciar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mandible_justiciar() -> CardIndex {
    card_index("ee8f3dbd-6e86-46c2-9393-7da6c3c4b46d")
}

/// Mandible Justiciar: a 2/1 with "Lifelink". Hitting unblocked for two, its
/// controller gains two.
#[test]
fn mandible_justiciar_gains_life_by_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[mandible_justiciar()], &[]);
    let justiciar = on_battlefield(&engine, p0, mandible_justiciar()).expect("seated");
    assert_eq!(pt(&engine, justiciar), (2, 1));
    assert!(keywords(&engine, justiciar).contains(KeywordSet::LIFELINK));
    swing_unblocked(&mut engine, justiciar);
    assert_eq!(life_of(&engine, 1), 18);
    assert_eq!(life_of(&engine, 0), 22);
}

/// "Whenever another artifact you control enters, this creature gets +1/+1
/// until end of turn": a Gold Myr makes it a 3/2; a Bears makes nothing.
#[test]
fn mandible_justiciar_gets_plus_one_plus_one_when_another_artifact_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[gold_myr(), grizzly_bears()],
        &[mandible_justiciar(), forest(), forest(), forest(), forest()],
        &[],
    );
    let justiciar = on_battlefield(&engine, p0, mandible_justiciar()).expect("seated");
    cast_and_land(&mut engine, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, justiciar), (2, 1), "the Bears are no artifact");
    cast_and_land(&mut engine, gold_myr());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, justiciar), (3, 2), "+1/+1");
    to_my_next_turn(&mut engine);
    assert_eq!(pt(&engine, justiciar), (2, 1), "until end of turn");
}
