//! `cards/sorceries/mv_4/sowing_salt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sowing_salt() -> CardIndex {
    card_index("131d2b70-959d-4814-b0c8-38031f141718")
}

/// Sowing Salt (131d2b70-959d-4814-b0c8-38031f141718) — {2}{R}{R} Sorcery (Partial):
/// "Exile target nonbasic land. Search its controller's graveyard, hand, and
/// library for all cards with the same name as that land and exile them. Then
/// that player shuffles."
#[test]
fn sowing_salt_exiles_target_nonbasic_land_and_excludes_basic_lands() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[index::WASTELAND, forest(), sol_ring()])
        .hand(0, &[sowing_salt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wasteland = on_battlefield(&engine, p1, index::WASTELAND).unwrap();
    let basic_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let basic_mountain = on_battlefield(&engine, p0, mountain()).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();

    cast_from_hand(&mut engine, p0, sowing_salt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Sowing Salt");
    };
    assert!(
        options.contains(&wasteland),
        "nonbasic land is a legal target"
    );
    assert!(
        !options.contains(&basic_forest)
            && !options.contains(&basic_mountain)
            && !options.contains(&ring),
        "basic lands and non-lands are not legal targets"
    );

    aim_at(&mut engine, p0, wasteland);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::WASTELAND).is_none());
    assert_eq!(engine.state().object(wasteland).unwrap().zone, Zone::Exile);
}
