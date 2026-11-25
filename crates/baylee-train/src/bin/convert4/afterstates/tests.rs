use std::fs;

use baylee_train::convert4::search_roots::SearchRoot4;

use super::*;
use crate::search_advantage::fixture::{Directory, event, game, recorded, stream};
use crate::search_advantage::{GameRoots, fixture};

#[test]
fn each_half_reads_its_own_worlds() {
    let worlds = vec![
        vec![0.5, 0.6, 0.4],
        vec![0.5, 0.5, 0.4],
        vec![0.5, 0.8, 0.4],
        vec![0.5, 0.7, 0.4],
    ];
    let (even, even_se) = half(&worlds, 1, 0);
    assert!((even - 0.2).abs() < 1e-12, "leads 0.1 and 0.3");
    assert!((even_se - 0.1).abs() < 1e-12, "sd 0.1414 over sqrt 2");
    let (odd, odd_se) = half(&worlds, 1, 1);
    assert!((odd - 0.1).abs() < 1e-12 && (odd_se - 0.1).abs() < 1e-12);
    let (own, own_se) = half(&worlds, 0, 0);
    assert_eq!(
        (own, own_se),
        (0.0, 0.0),
        "the own answer leads itself by 0"
    );
    let (single, single_se) = half(&worlds[..1], 1, 0);
    assert!((single - 0.1).abs() < 1e-12 && single_se.is_nan());
    assert!(half(&worlds[..1], 1, 1).0.is_nan());
}

#[test]
fn a_candidate_names_the_spell_it_or_its_taps_put_on_the_stack() {
    let card = baylee_core::ids::ObjectId::new(61, 0);
    let tap = PlayerAction::ActivateManaAbility {
        source: baylee_core::ids::ObjectId::new(70, 0),
    };
    let expected = serde_json::to_value(card)
        .unwrap()
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .unwrap();
    let cast = PlayerAction::CastSpell { card };
    assert_eq!(spell_of(&tap, Some(&cast)), expected);
    // What a bare tap was for names a suspend and a foretell as it names a
    // cast, so a candidate that is one is named alike.
    for action in [
        cast,
        PlayerAction::Suspend { card },
        PlayerAction::Foretell { card },
    ] {
        assert_eq!(spell_of(&action, None), expected, "{action:?}");
    }
    assert_eq!(spell_of(&tap, None), u32::MAX);
    assert_eq!(spell_of(&PlayerAction::PassPriority, None), u32::MAX);
}

#[test]
fn the_sidecar_rows_are_sorted_and_laid_out() {
    let dir = std::env::temp_dir().join(format!("search-afterstates-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let row = |game: u32, root_cursor: u32, candidate: u8| AfterRow {
        game,
        root_cursor,
        item: 3,
        frame_cursor: root_cursor * 8 + u32::from(candidate),
        seat: 1,
        candidate,
        stop: 0,
        flags: 0b10_0111,
        answers: 2,
        next_kind: 4,
        worlds: 4,
        halves: [0.25, 0.5, -0.25, f32::NAN],
        card: 61,
    };
    let Sidecar {
        header,
        files_sha256,
    } = write(
        &dir,
        vec![row(2, 5, 0), row(1, 9, 1), row(1, 9, 0)],
        &AfterCounts::default(),
        &json!({}),
    )
    .unwrap();
    let index = fs::read(dir.join("index.bin")).unwrap();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(index.len(), 3 * AFTER_ROW);
    let word = |at: usize| u32::from_le_bytes(index[at..at + 4].try_into().unwrap());
    assert_eq!((word(0), word(4), word(8), word(12)), (1, 9, 3, 72));
    assert_eq!(&index[16..20], &[1, 0, 0, 0b10_0111]);
    assert_eq!(u16::from_le_bytes([index[20], index[21]]), 2);
    assert_eq!(&index[22..24], &[4, 4]);
    assert_eq!(f32::from_le_bytes(index[24..28].try_into().unwrap()), 0.25);
    assert!(f32::from_le_bytes(index[36..40].try_into().unwrap()).is_nan());
    assert_eq!(word(40), 61);
    assert_eq!(&index[44..48], &[0; 4]);
    assert_eq!(
        index[AFTER_ROW + 17],
        1,
        "candidate 1 of the same root follows"
    );
    assert_eq!(word(2 * AFTER_ROW), 2);
    assert_eq!(header["index"]["rows"], 3);
    assert_eq!(header["index"]["stops"][8], "forks_differ");
    assert_eq!(header["index"]["sha256"], files_sha256["index.bin"]);
    assert_eq!(files_sha256.len(), 2);
    assert_eq!(header["contract"], AFTERSTATE_CONTRACT);
}

/// A root of the recorded game whose own answer's and pass's afterstates the
/// replay captured, with its frame as the converter writes it.
fn captured() -> (Step4, SearchRoot4) {
    let events: Vec<Value> = recorded()
        .acts
        .iter()
        .map(|(n, action)| event(*n, action))
        .collect();
    let roots = GameRoots::from_game(&game(&events)).unwrap();
    let mut found = None;
    stream(&roots.roots(true), |mut step| {
        if let Some(root) = step.search_root.take()
            && found.is_none()
        {
            let after = root.afterstates.as_ref().expect("asked for");
            if after.root.is_some() && after.candidates.iter().all(|c| c.stop.reached()) {
                step.loss_weight = 0.0;
                found = Some((step, *root));
            }
        }
        Ok(())
    })
    .unwrap();
    found.expect("a root whose candidates both reach a position")
}

/// What `record` makes of a root whose capture is `after`: its rows, its
/// counts and the frames it wrote.
fn recorded_as(step: &Step4, after: &RootAfterstates4) -> (Vec<AfterRow>, AfterCounts, Vec<Value>) {
    let directory = Directory::new();
    let mut game = GameAfterstates::new(
        directory.path(),
        step.game_id,
        recorded().rule_roots.clone(),
    )
    .unwrap();
    let candidates = [PlayerAction::PassPriority, PlayerAction::PassPriority];
    let worlds: Vec<Vec<f64>> = fixture::WORLDS.iter().map(|w| w.to_vec()).collect();
    game.record(
        step,
        after,
        &EventCandidates {
            candidates: &candidates,
            scripts: &[],
            worlds: &worlds,
            items: &[3, 0],
        },
    )
    .unwrap();
    let (rows, counts, shard) = game.finish().unwrap();
    let mut frames = Vec::new();
    baylee_train_ops::trajectory::read_shard(
        &directory.path().join(AFTER_SPLIT).join(shard),
        |frame| {
            frames.push(frame);
            Ok(())
        },
    )
    .unwrap();
    (rows, counts, frames)
}

/// A fault of an afterstate: its name, its stop, the row's first three flags
/// and what makes it.
type Fault = (&'static str, AfterStop4, u8, fn(&mut RootAfterstates4));

#[test]
fn a_valid_afterstate_is_a_row_and_a_frame_and_a_faulty_one_a_row_that_says_why() {
    let (step, root) = captured();
    let after = root.afterstates.unwrap();

    let (rows, counts, frames) = recorded_as(&step, &after);
    assert_eq!((counts.roots, counts.candidates, counts.valid), (1, 2, 2));
    assert_eq!((counts.roots_own_valid, counts.root_mismatch_roots), (1, 0));
    assert_eq!(counts.deal_differs_valid, 2, "two deals gave one frame");
    assert_eq!(frames.len(), 2);
    for (at, (row, frame)) in rows.iter().zip(&frames).enumerate() {
        let cursor = step.step_cursor * 8 + u32::try_from(at).unwrap();
        assert_eq!(row.frame_cursor, cursor);
        assert_eq!(
            (row.game, row.root_cursor),
            (step.game_id, step.step_cursor)
        );
        assert_eq!(usize::from(row.candidate), at);
        assert_eq!(row.item, [3, 0][at]);
        assert_eq!(row.stop, AfterStop4::Question as u8);
        assert_eq!(
            row.flags & 0b111,
            0b111,
            "valid, the root and the forks match"
        );
        assert_eq!(frame["step_cursor"], cursor);
        assert_eq!(frame["game_id"], step.game_id);
        assert_eq!(frame["is_decision"], false);
        assert_eq!(frame["loss_weight"], 0.0);
        assert!(frame["cand_feat"].as_array().unwrap().is_empty());
    }
    // The second candidate's lead on the even worlds and on the odd ones.
    assert!((rows[1].halves[0] - 0.25).abs() < 1e-6);
    assert!((rows[1].halves[2] + 0.05).abs() < 1e-6);

    // Each fault of the pass's afterstate, whose own answer's stays valid.
    let faults: [Fault; 4] = [
        ("forks_differ", AfterStop4::ForksDiffer, 0b010, |after| {
            after.candidates[1].frames[1].as_mut().unwrap().turn += 1;
        }),
        ("dealt_seen", AfterStop4::DealtSeen, 0b110, |after| {
            after.candidates[1].hidden.dealt_seen = true;
        }),
        ("chance", AfterStop4::Chance, 0b110, |after| {
            after.candidates[1].hidden.chance = true;
        }),
        (
            "answer_refused",
            AfterStop4::AnswerRefused,
            0b010,
            |after| {
                after.candidates[1].stop = AfterStop4::AnswerRefused;
                after.candidates[1].frames = [None, None];
            },
        ),
    ];
    for (name, stop, flags, spoil) in faults {
        let mut spoilt = after.clone();
        spoil(&mut spoilt);
        let (rows, counts, frames) = recorded_as(&step, &spoilt);
        assert_eq!(rows[0].flags & 1, 1, "{name}");
        assert_eq!(
            (rows[1].stop, rows[1].flags & 0b111),
            (stop as u8, flags),
            "{name}"
        );
        assert_eq!(rows[1].frame_cursor, u32::MAX, "{name}");
        assert_eq!((counts.valid, counts.by_stop[name]), (1, 1));
        assert_eq!(frames.len(), 1, "{name}");
    }
}

/// A fork whose frame of the root is not the replay's says nothing of any of
/// its candidates.
#[test]
fn a_root_mismatch_leaves_every_candidate_out() {
    let (step, root) = captured();
    let after = root.afterstates.unwrap();
    let mut other_state = after.clone();
    other_state.root.as_mut().unwrap().0.global_feat[0] += 1.0;
    let mut other_menu = after.clone();
    other_menu.root.as_mut().unwrap().1.candidates.pop();
    let mut no_fork = after;
    no_fork.root = None;
    for spoilt in [other_state, other_menu, no_fork] {
        let (rows, counts, frames) = recorded_as(&step, &spoilt);
        for row in &rows {
            assert_eq!(row.stop, AfterStop4::RootMismatch as u8);
            assert_eq!(row.flags & 0b011, 0);
            assert_eq!(row.frame_cursor, u32::MAX);
        }
        assert_eq!((counts.valid, counts.root_mismatch_roots), (0, 1));
        assert_eq!(
            (counts.by_stop["root_mismatch"], counts.roots_own_valid),
            (2, 0)
        );
        assert!(frames.is_empty());
    }
}

#[test]
fn a_root_whose_parts_disagree_or_have_no_room_is_refused() {
    let (step, root) = captured();
    let after = root.afterstates.unwrap();
    let directory = Directory::new();
    let mut game = GameAfterstates::new(
        directory.path(),
        step.game_id,
        recorded().rule_roots.clone(),
    )
    .unwrap();
    let mut crowded = after.clone();
    crowded.candidates.resize(9, after.candidates[0].clone());
    for (captured, n) in [(&after, 3), (&crowded, 9)] {
        let candidates = vec![PlayerAction::PassPriority; n];
        let error = game
            .record(
                &step,
                captured,
                &EventCandidates {
                    candidates: &candidates,
                    scripts: &[],
                    worlds: &[],
                    items: &vec![0; n],
                },
            )
            .unwrap_err();
        assert!(error.contains(&format!("{n} candidates")), "{error}");
    }
}
