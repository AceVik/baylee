//! The labels of a real game's roots, from its events to its sidecar rows and
//! its twins.

use super::fixture::{AHEAD_SE, Directory, SEAT, event, game, recorded, stream};
use super::*;

/// What a game's labels come to: its rows, its counts and its twins.
struct Labelled {
    rows: Vec<Row>,
    counts: Counts,
    twins: Vec<Value>,
    /// Frames written with a policy weight.
    weighted: usize,
}

fn label_game(entry: &Value) -> Result<Labelled, String> {
    let directory = Directory::new();
    let roots = GameRoots::from_game(entry).unwrap();
    let mut labels = GameLabels::new(
        &roots,
        directory.path(),
        4,
        recorded().rule_roots.clone(),
        &recorded().record,
        false,
    )
    .unwrap();
    let mut weighted = 0;
    stream(&roots.roots(false), |mut step| {
        labels.apply(&mut step)?;
        weighted += usize::from(step.loss_weight != 0.0);
        assert!(
            step.search_root.is_none(),
            "a root is taken with its labels"
        );
        Ok(())
    })?;
    let (rows, counts, shard, after) = labels.finish().unwrap();
    assert!(after.is_none());
    let mut twins = Vec::new();
    baylee_train_ops::trajectory::read_twins(
        &directory.path().join(MENUS_SPLIT).join(shard.unwrap()),
        |twin| {
            twins.push(twin);
            Ok(())
        },
    )
    .unwrap();
    Ok(Labelled {
        rows,
        counts,
        twins,
        weighted,
    })
}

fn accounted(counts: &Counts) -> u64 {
    counts.labelled + counts.dropped.values().sum::<u64>()
}

#[test]
fn a_labelled_root_has_its_candidates_labels_and_its_twin() {
    let acts = &recorded().acts;
    let events: Vec<Value> = acts.iter().map(|(n, action)| event(*n, action)).collect();
    let Labelled {
        rows,
        counts,
        twins,
        weighted,
    } = label_game(&game(&events)).unwrap();
    assert!(counts.labelled > 5, "{counts:?}");
    assert_eq!(counts.events, acts.len() as u64);
    assert_eq!(accounted(&counts), counts.events, "{counts:?}");
    assert_eq!(rows.len() as u64, counts.labelled);
    assert_eq!(twins.len(), rows.len());
    assert_eq!(weighted, 0, "no answer of the search is a policy target");
    for (row, twin) in rows.iter().zip(&twins) {
        // The policy's own answer is the baseline, and the other candidate's
        // labels are the worlds'.
        let own = &row.items[row.own as usize];
        assert!(own.searched && own.advantage == 0.0 && own.se == 0.0);
        let other = &row.items[row.choice4 as usize];
        assert!(other.searched && row.choice4 != row.own);
        assert!((f64::from(other.advantage) - 0.1).abs() < 1e-6, "{other:?}");
        assert!((f64::from(other.se) - AHEAD_SE).abs() < 1e-5, "{other:?}");
        for (at, item) in row.items.iter().enumerate() {
            let candidate = [row.own, row.choice4].contains(&u32::try_from(at).unwrap());
            assert_eq!(item.searched, candidate);
            assert_eq!(item.advantage.is_nan(), !candidate);
        }
        // The half {0, 2} leads by 0.25 and the half {1, 3} is behind: one
        // chooses the pass, the other keeps the answer.
        assert_eq!((row.half_a, row.half_b), (row.choice4, row.own));
        assert!((row.judged[0] + 0.05).abs() < 1e-6 && row.judged[1] == 0.0);
        // The twin is the root's frame with the payable menu, the record's
        // answer at its own index.
        assert_eq!(twin["game_id"], 4);
        assert_eq!(twin["seat"], SEAT);
        assert_eq!(twin["step_cursor"], row.cursor);
        assert_eq!(twin["is_decision"], true);
        assert_eq!(twin["loss_weight"], 0.0);
        assert_eq!(twin["target"]["single_idx"], row.own);
        assert_eq!(twin["cand_feat"].as_array().unwrap().len(), row.items.len());
    }
    assert!(
        rows.iter()
            .any(|row| row.items.iter().any(|item| item.added)),
        "some root's payable menu adds a spell"
    );
}

/// A twin's menu is the frame's own and, after it, what the payable menu adds.
#[test]
fn a_twin_is_its_frame_with_the_payable_menu_after_its_own() {
    let acts = &recorded().acts;
    let events: Vec<Value> = acts.iter().map(|(n, action)| event(*n, action)).collect();
    let roots = GameRoots::from_game(&game(&events)).unwrap();
    let (mut seen, mut longer) = (0, 0);
    stream(&roots.roots(false), |mut step| {
        let Some(root) = step.search_root.take() else {
            return Ok(());
        };
        step.loss_weight = 1.0;
        let twin = twin_of(&step, &root);
        let own = step.cand_feat.len();
        assert_eq!(own, root.first_added);
        assert_eq!(twin.cand_feat.len(), root.menu.candidates.len());
        assert_eq!(twin.cand_feat[..own], step.cand_feat[..]);
        assert_eq!(twin.cand_card_id[..own], step.cand_card_id[..]);
        assert_eq!(twin.cand_entity_idx[..own], step.cand_entity_idx[..]);
        assert_eq!(twin.cand_ability_idx[..own], step.cand_ability_idx[..]);
        assert_eq!(twin.cand_face_id[..own], step.cand_face_id[..]);
        assert_eq!(twin.cand_rule_refs.len(), twin.cand_feat.len());
        assert_eq!(twin.target.single_idx, step.target.single_idx);
        assert_eq!(
            (twin.game_id, twin.seat, twin.step_cursor),
            (step.game_id, step.seat, step.step_cursor)
        );
        assert_eq!(twin.loss_weight, 0.0);
        assert!(twin.cand_classes.is_none() && twin.search_root.is_none());
        seen += 1;
        longer += usize::from(twin.cand_feat.len() > own);
        Ok(())
    })
    .unwrap();
    assert!(seen > 5 && longer > 0, "{seen} {longer}");
}

/// Why a root is left out, and that each event is counted once.
#[test]
fn a_root_that_cannot_be_labelled_is_counted_by_its_reason() {
    let acts = &recorded().acts;
    let spoil = |entry: &mut Value, how: usize| match how {
        0 => entry["diffs"] = json!([0.0, 0.3]),
        1 => entry["ses"] = json!([0.0, 0.3]),
        2 => entry["candidates"][1] = json!({"PlayLand": {"card": 4_000_000_000_u32}}),
        3 => entry["candidates"][1] = entry["candidates"][0].clone(),
        4 => {
            entry["candidates"][1] = entry["candidates"][0].clone();
            entry["targets"] = json!([null, {"ChooseTargets": {"objects": [], "players": [1]}}]);
        }
        5 => entry["scripts"] = json!([null, {"CastSpell": {"card": 4_000_000_000_u32}}]),
        _ => entry["record_input_n"] = json!(u64::MAX),
    };
    assert!(acts.len() >= 7, "{acts:?}");
    let mut events = Vec::new();
    for (how, (n, action)) in acts.iter().take(7).enumerate() {
        let mut entry = event(*n, action);
        spoil(&mut entry, how);
        events.push(entry);
    }
    let labelled = label_game(&game(&events)).unwrap();
    assert_eq!(
        labelled.counts.dropped,
        BTreeMap::from([
            ("label_mismatch".to_owned(), 2),
            ("unmapped".to_owned(), 1),
            ("shared_item".to_owned(), 1),
            ("target_variants".to_owned(), 1),
            ("not_payable".to_owned(), 1),
            ("no_frame".to_owned(), 1),
        ])
    );
    assert!(labelled.rows.is_empty() && labelled.twins.is_empty());
    assert_eq!(accounted(&labelled.counts), 7);
}

/// An event whose played answer is another than the record's is of another
/// game, which is refused rather than left out.
#[test]
fn an_event_that_does_not_bind_to_its_frame_rejects_the_game() {
    let (n, action) = &recorded().acts[0];
    let mut entry = event(*n, action);
    entry["action"] = json!("PassPriority");
    let error = label_game(&game(&[entry])).err().unwrap();
    assert!(error.contains("not the record's answer"), "{error}");
}

/// A root whose own answer has no kind: a priority answer without a code, and
/// a question that is none.
#[test]
fn a_root_without_a_kind_says_whether_it_is_a_priority_question() {
    let own = |action: Value| {
        let mut entry = event(5, &PlayerAction::PassPriority);
        entry["candidates"][0] = action;
        GameRoots::from_game(&game(&[entry]))
            .unwrap()
            .counts
            .dropped
    };
    for action in [
        json!({"Suspend": {"card": 3}}),
        json!({"Foretell": {"card": 3}}),
        json!("CancelCast"),
    ] {
        assert_eq!(
            own(action),
            BTreeMap::from([("no_root_kind".to_owned(), 1)])
        );
    }
    assert_eq!(
        own(json!({"DeclareAttackers": {"attackers": []}})),
        BTreeMap::from([("not_priority".to_owned(), 1)])
    );
    assert!(own(json!("PassPriority")).is_empty());
}

/// A candidate is played on from as its search tried it.
#[test]
fn a_candidates_script_and_target_go_with_it() {
    let target = json!({"ChooseTargets": {"objects": [], "players": [1]}});
    let cast = json!({"CastSpell": {"card": 3}});
    let mut entry = event(5, &PlayerAction::PassPriority);
    entry["candidates"] = json!(["PassPriority", cast, {"ActivateManaAbility": {"source": 9}}]);
    entry["scripts"] = json!([null, null, cast]);
    entry["targets"] = json!([null, target, null]);
    entry["worlds"] = json!([[0.5, 0.5, 0.5], [0.5, 0.5, 0.5]]);
    let roots = GameRoots::from_game(&game(&[entry])).unwrap();
    let asked = roots.roots(true).afterstates.remove(&(SEAT, 5)).unwrap();
    let parse = |value: &Value| serde_json::from_value::<PlayerAction>(value.clone()).unwrap();
    assert_eq!(
        asked
            .iter()
            .map(|tried| (tried.script.clone(), tried.target.clone()))
            .collect::<Vec<_>>(),
        [
            (None, None),
            (None, Some(parse(&target))),
            (Some(parse(&cast)), None),
        ]
    );
    assert!(roots.roots(false).afterstates.is_empty());
}
