//! `cards/creatures/mv_7/quilled_slagwurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn quilled_slagwurm() -> CardIndex {
    card_index("5114f3e0-339d-46d6-9d44-97fa0afbd7c5")
}

/// Quilled Slagwurm — a vanilla 8/8 for {4}{G}{G}{G}.
#[test]
fn quilled_slagwurm_is_a_vanilla_eight_eight_for_seven() {
    let mut engine = table7(&lots(forest(), 7), &[quilled_slagwurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, quilled_slagwurm());
    assert_body(&engine, wurm, (8, 8), KeywordSet::default());
    assert_eq!(keywords(&engine, wurm), KeywordSet::default(), "vanilla");
}

#[test]
fn quilled_slagwurm_hits_for_eight() {
    let (mut engine, wurm, _ogre) = duel_vs_ogre(quilled_slagwurm(), &[]);
    fight(&mut engine, &[wurm], &[]);
    assert_eq!(life_of(&engine, 1), 12);
}
