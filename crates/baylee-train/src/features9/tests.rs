use super::*;
#[path = "fixture.rs"]
mod fixture;
use baylee_engine::choice::PlayerAction;
use fixture::{Fixture, OTHER, OWNER};
use serde_json::json;

fn difference(a: &serde_json::Value, b: &serde_json::Value, path: &str) -> Option<String> {
    if a == b {
        return None;
    }
    if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
        for (i, (a, b)) in a.iter().zip(b).enumerate() {
            if let Some(d) = difference(a, b, &format!("{path}/{i}")) {
                return Some(d);
            }
        }
        if a.len() != b.len() {
            return Some(format!("{path}: lengths {} != {}", a.len(), b.len()));
        }
    } else if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
        for (k, a) in a {
            if let Some(d) = difference(a, &b[k], &format!("{path}/{k}")) {
                return Some(d);
            }
        }
    }
    Some(format!("{path}: {a} != {b}"))
}
#[test]
fn actual_fireball_selected_x_preserves_encoder8_prefix_and_privacy() {
    let (mut f, _) = Fixture::before_x();
    let (pending, none) = f.session.view_for_with_announcement_context(OWNER).unwrap();
    let mut samples = Vec::new();
    for x in [None, Some(0), Some(1), Some(u32::MAX)] {
        let mut view = none.clone();
        view.announced_x = x;
        let e8 = crate::features8::encode_state8(&view, Some(&pending), None, &[]);
        let e9 = encode_state9(&view, Some(&pending), None, &[]);
        assert_eq!(e9.global_feat.len(), GLOBAL_FEAT_WIDTH9);
        assert_eq!(e9.global_feat[..80], e8.global_feat);
        let mut prefix = e9.clone();
        prefix.global_feat.truncate(80);
        assert_eq!(
            serde_json::to_value(prefix).unwrap(),
            serde_json::to_value(e8).unwrap()
        );
        assert_eq!(e9.global_feat[80], f32::from(u8::from(x.is_some())));
        assert!(e9.global_feat[81].is_finite());
        samples.push(json!({"announced_x":x,"state":e9}));
    }
    let none_hash = board_observation_hash9(&none);
    let mut zero = none.clone();
    zero.announced_x = Some(0);
    assert_ne!(none_hash, board_observation_hash9(&zero));
    assert_ne!(
        view_only_state_hash9(&none, &pending),
        view_only_state_hash9(&zero, &pending)
    );
    assert_eq!(
        crate::features8::board_observation_hash8(&none),
        crate::features8::board_observation_hash8(&zero)
    );
    let mut kn = SeatKnowledge4::empty(OWNER, 2);
    assert!(kn.observe_with_announcement_context(&none));
    assert!(!kn.observe_with_announcement_context(&none));
    let cursor = kn.cursor;
    assert!(kn.observe_with_announcement_context(&zero));
    assert_eq!(kn.cursor, cursor + 1);
    assert!(!kn.observe_with_announcement_context(&zero));
    f.act(PlayerAction::ChooseNumber(0));
    let target0 = f.session.view_for_with_announcement_context(OWNER).unwrap();
    assert_eq!(target0.1.announced_x, Some(0));
    assert_eq!(
        &encode_state9(&target0.1, Some(&target0.0), None, &[]).global_feat[80..],
        &[1.0, 0.0]
    );
    let (mut one, _) = Fixture::before_x();
    one.act(PlayerAction::ChooseNumber(1));
    let target1 = one
        .session
        .view_for_with_announcement_context(OWNER)
        .unwrap();
    assert_eq!(
        &encode_state9(&target1.1, Some(&target1.0), None, &[]).global_feat[80..],
        &[1.0, 1.0]
    );
    assert_eq!(
        serde_json::to_value(crate::features8::encode_state8(
            &target0.1,
            Some(&target0.0),
            None,
            &[]
        ))
        .unwrap(),
        serde_json::to_value(crate::features8::encode_state8(
            &target1.1,
            Some(&target1.0),
            None,
            &[]
        ))
        .unwrap()
    );
    let other = one.session.observe_view_with_announcement_context(OTHER);
    assert!(other.announced_x.is_none());
    assert_eq!(
        &encode_state9(&other, None, None, &[]).global_feat[80..],
        &[0.0, 0.0]
    );
    if let Some(path) = std::env::var_os("BAYLEE_E9_FIXTURE") {
        let out = json!({"encoder_version":9,"model_version":5,"global_feat_width":82,"entity_feat_width":128,"sampling_version":2,"relational_revision":2,"cases":samples,"actual_target0":encode_state9(&target0.1,Some(&target0.0),None,&[]),"actual_target1":encode_state9(&target1.1,Some(&target1.0),None,&[])});
        std::fs::write(path, serde_json::to_vec_pretty(&out).unwrap()).unwrap();
    }
}
#[test]
fn converter_and_runtime_observation_clocks_match_on_actual_x_history() {
    let (mut f, _) = Fixture::before_x();
    f.act(PlayerAction::ChooseNumber(1));
    f.act(PlayerAction::ChooseTargets {
        objects: vec![],
        players: vec![OTHER],
    });
    f.session.act(OTHER, PlayerAction::Concede).unwrap();
    let record = f.session.take_record();
    if let Some(path) = std::env::var_os("BAYLEE_E9_FIXTURE_RECORD") {
        std::fs::write(path, &record).unwrap();
    }
    let mut actual = Vec::new();
    let summary = crate::convert4::stream_game4_with_encoder(
        &record,
        1,
        true,
        None,
        None,
        None,
        true,
        9,
        |frame| {
            actual.push(
                json!({"seat":frame.seat,"is_decision":frame.is_decision,"state":frame.state}),
            );
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(summary.unencoded_decisions, 0);
    actual.sort_by_key(|row| row["seat"].as_u64().unwrap());
    f.expected.sort_by_key(|row| row["seat"].as_u64().unwrap());
    let diff = difference(&json!(actual), &json!(f.expected), "");
    if diff.is_some() {
        for (name, rows) in [("actual", &actual), ("expected", &f.expected)] {
            eprintln!(
                "{name}: {:?}",
                rows.iter()
                    .map(|r| (
                        &r["seat"],
                        &r["is_decision"],
                        &r["state"]["global_feat"][34]
                    ))
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(
        diff.is_none(),
        "runtime/converter mismatch: {}",
        diff.unwrap_or_default()
    );
    assert!(
        actual
            .iter()
            .any(|r| r["state"]["global_feat"][80].as_f64() == Some(1.0)
                && r["state"]["global_feat"][81].as_f64() == Some(1.0))
    );
}
