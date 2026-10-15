//! `cards/creatures/artifacts/mv_1/swooping_lookout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn swooping_lookout() -> CardIndex {
    card_index("0594f19c-c4c2-4812-8f4d-5630fc24c47a")
}

/// "Flying, vigilance": the 1/2 attacks over a ground blocker's reach and is
/// not tapped by attacking.
#[test]
fn swooping_lookout_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[swooping_lookout()], &[]);
    let me = on_battlefield(&engine, p0, swooping_lookout()).expect("seated");
    assert_eq!(pt(&engine, me), (1, 2));
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, me).contains(KeywordSet::VIGILANCE));
    let life = engine.state().players[1].life;

    attack_with(&mut engine, me, None);
    assert_eq!(engine.state().players[usize::from(p1.0)].life, life - 1);
    assert!(
        !is_tapped(&engine, me),
        "vigilance: attacking did not tap it"
    );
}
