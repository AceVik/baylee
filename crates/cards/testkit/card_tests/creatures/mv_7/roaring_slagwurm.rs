//! `cards/creatures/mv_7/roaring_slagwurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn roaring_slagwurm() -> CardIndex {
    card_index("856d8eb4-3834-49f8-9696-579948590fd0")
}

/// Roaring Slagwurm — a 6/4 for {5}{G}{G}.
#[test]
fn roaring_slagwurm_costs_seven_and_is_a_six_four() {
    let mut engine = table7(&lots(forest(), 7), &[roaring_slagwurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, roaring_slagwurm());
    assert_body(&engine, wurm, (6, 4), KeywordSet::default());
}

/// "Whenever this creature attacks, tap all artifacts": both players' Sol
/// Rings are untapped before and tapped after the attack is declared.
#[test]
fn roaring_slagwurm_taps_every_artifact_when_it_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[roaring_slagwurm(), sol_ring()], &[], &[sol_ring()]);
    let wurm = on_battlefield(&engine, p0, roaring_slagwurm()).expect("out");
    let mine = on_battlefield(&engine, p0, sol_ring()).expect("out");
    let theirs = on_battlefield(&engine, p1, sol_ring()).expect("out");
    assert!(!is_tapped(&engine, mine) && !is_tapped(&engine, theirs));
    declare_attack(&mut engine, &[wurm], &[]);
    assert!(is_tapped(&engine, mine), "my artifact too");
    assert!(is_tapped(&engine, theirs), "their artifact");
}
