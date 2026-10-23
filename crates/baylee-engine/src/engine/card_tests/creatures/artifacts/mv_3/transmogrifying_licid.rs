#[allow(clippy::wildcard_imports)]
use super::*;

fn transmogrifying_licid() -> CardIndex {
    card_index("4f484927-0e8e-493d-93bc-ed190965dad9")
}

#[test]
fn transmogrifying_licid_grants_plus_one_plus_one_and_artifact_type_when_attached() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[transmogrifying_licid()])
        .battlefield(0, &[forest(), forest(), forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, transmogrifying_licid());
    pass_until(&mut engine, stack_is_empty);
    let licid = on_battlefield(&engine, p0, transmogrifying_licid()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    assert_eq!(pt(&engine, licid), (2, 2));
    assert_eq!(pt(&engine, bears), (2, 2));

    // Attach Transmogrifying Licid to Grizzly Bears to verify its static Aura bonuses.
    engine.state.object_mut(licid).unwrap().attached_to = Some(bears);
    engine.state.invalidate_projections();
    engine.sync_static_effects();

    assert_eq!(pt(&engine, bears), (3, 3));
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT)
    );
}
