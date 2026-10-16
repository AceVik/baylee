//! `cards/creatures/artifacts/mv_3/s_h_i_e_l_d_deployment_drone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn s_h_i_e_l_d_deployment_drone() -> CardIndex {
    card_index("c1669020-8f81-471a-9083-7e9ce3e8f387")
}

/// S.H.I.E.L.D. Deployment Drone — flying 2/2; "When this creature enters,
/// create a 1/1 white Soldier creature token."
#[test]
fn s_h_i_e_l_d_deployment_drone_creates_a_one_one_soldier_token() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[s_h_i_e_l_d_deployment_drone()], &[island(), island(), island()]);

    cast_and_settle(&mut engine, s_h_i_e_l_d_deployment_drone());

    let drone = on_battlefield(&engine, p0, s_h_i_e_l_d_deployment_drone()).expect("it resolved");
    assert_eq!(pt(&engine, drone), (2, 2));
    assert!(keywords(&engine, drone).contains(KeywordSet::FLYING));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(types(&engine, tokens[0]).contains(TypeSet::CREATURE));
    assert!(!keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}
