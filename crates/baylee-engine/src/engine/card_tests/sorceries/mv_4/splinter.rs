//! `cards/sorceries/mv_4/splinter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn splinter() -> CardIndex {
    card_index("90f01e9f-68f1-4e41-97dc-8de990a91504")
}

/// Splinter (90f01e9f-68f1-4e41-97dc-8de990a91504) — {2}{G}{G} Sorcery (Partial):
/// "Exile target artifact. Search its controller's graveyard, hand, and
/// library for all cards with the same name as that artifact and exile them.
/// Then that player shuffles."
#[test]
fn splinter_exiles_target_artifact_and_excludes_non_artifacts() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[sol_ring(), index::GRIZZLY_BEARS, forest()])
        .hand(0, &[splinter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();

    cast_from_hand(&mut engine, p0, splinter());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Splinter");
    };
    assert!(options.contains(&ring), "artifact is a legal target");
    assert!(
        !options.contains(&bears) && !options.contains(&land),
        "creatures and lands are not legal targets"
    );

    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert_eq!(engine.state().object(ring).unwrap().zone, Zone::Exile);
}
