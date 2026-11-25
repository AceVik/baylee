use std::fs;

use super::*;

fn item(advantage: f32, searched: bool) -> Item {
    Item {
        advantage: if searched { advantage } else { f32::NAN },
        se: if searched { 0.01 } else { f32::NAN },
        searched,
        added: false,
        class: 0,
    }
}

#[test]
fn the_sidecar_holds_sorted_roots_and_their_items() {
    let dir = std::env::temp_dir().join(format!("search-advantage-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let row = |game: u32, cursor: u32, n: usize| Row {
        game,
        cursor,
        seat: 0,
        kind: 1,
        own: 0,
        choice4: 1,
        half_a: 1,
        half_b: u32::MAX,
        judged: [0.25, 0.0],
        intent: if game == 1 { 29 } else { u32::MAX },
        items: (0..n)
            .map(|at| item(f32::from(u8::try_from(at).unwrap()) * 0.1, at < 2))
            .collect(),
    };
    let counts = Counts {
        games: 2,
        events: 3,
        labelled: 2,
        dropped: BTreeMap::from([("stopped".to_owned(), 1)]),
        intents: BTreeMap::new(),
    };
    let Sidecar {
        header,
        files_sha256,
    } = write(&dir, vec![row(2, 5, 3), row(1, 9, 2)], &counts, &json!({})).unwrap();
    let index = fs::read(dir.join("index.bin")).unwrap();
    let cands = fs::read(dir.join("cands.bin")).unwrap();
    let intents = fs::read(dir.join("intents.bin")).unwrap();
    let judged = fs::read(dir.join("halves-judged.bin")).unwrap();
    let _ = fs::remove_dir_all(&dir);
    // What meta.json binds: every file, the header among them.
    assert_eq!(files_sha256.len(), 5);
    for (name, part) in [
        ("index.bin", "index"),
        ("cands.bin", "cands"),
        ("intents.bin", "intents"),
        ("halves-judged.bin", "halves_judged"),
    ] {
        assert_eq!(header[part]["sha256"], files_sha256[name], "{name}");
    }
    assert_eq!(index.len(), 2 * INDEX_ROW);
    assert_eq!(cands.len(), 5 * CAND_ROW);
    let word = |at: usize| u32::from_le_bytes(index[at..at + 4].try_into().unwrap());
    // Game 1 first: cursor 9 twice, seat 0, kind tap, two items at 0.
    assert_eq!((word(0), word(4), word(8)), (1, 9, 9));
    assert_eq!(&index[12..16], &[0, 1, 0, 0]);
    assert_eq!(
        (word(16), word(20), word(24), word(28), word(32)),
        (0, 2, 1, 1, u32::MAX)
    );
    assert_eq!(u64::from_le_bytes(index[36..44].try_into().unwrap()), 0);
    assert_eq!(&index[44..48], &[0; 4]);
    // Game 2's items start after game 1's two.
    assert_eq!(
        u64::from_le_bytes(index[INDEX_ROW + 36..INDEX_ROW + 44].try_into().unwrap()),
        2
    );
    let float = |at: usize| f32::from_le_bytes(cands[at..at + 4].try_into().unwrap());
    assert_eq!(float(CAND_ROW), 0.1);
    assert!(float(4 * CAND_ROW).is_nan(), "an item no candidate took");
    assert_eq!(&cands[8..12], &[1, 0, 0, 0]);
    assert_eq!(header["counts"]["dropped"]["stopped"], 1);
    assert_eq!(header["contract"], "search_advantage_v1");
    assert_eq!(header["index"]["rows"], 2);
    // The chain spells, in the index's order: game 1's first.
    assert_eq!(intents, [29, 0, 0, 0, 255, 255, 255, 255]);
    assert_eq!(header["intents"]["rows"], 2);
    assert_eq!(judged.len(), 16);
    assert_eq!(f32::from_le_bytes(judged[0..4].try_into().unwrap()), 0.25);
    assert_eq!(header["halves_judged"]["rows"], 2);
}

#[test]
fn a_game_with_a_fallback_labels_nothing_and_says_why() {
    let event = json!({"accepted": true, "seat": 0, "record_input_n": 5,
        "action": "PassPriority", "candidates": ["PassPriority", {"PlayLand": {"card": 3}}],
        "scripts": [null, null], "worlds": [[0.5, 0.6], [0.5, 0.4]], "chosen": 0,
        "leaves": {"stopped": 0}, "diffs": [0.0, 0.0], "ses": [null, 0.1]});
    let stopped = {
        let mut e = event.clone();
        e["leaves"]["stopped"] = json!(1);
        e["record_input_n"] = json!(9);
        e
    };
    let attack = {
        let mut e = event.clone();
        e["candidates"] = json!([{"DeclareAttackers": {"attackers": []}}, "PassPriority"]);
        e["record_input_n"] = json!(11);
        e
    };
    let game = |fallbacks: u64| {
        json!({"fallback_answers": fallbacks, "refused_answers": 0,
            "search_events": [event, stopped, attack]})
    };
    let clean = GameRoots::from_game(&game(0)).unwrap();
    assert_eq!(clean.events.len(), 1);
    assert_eq!(clean.counts.dropped["stopped"], 1);
    assert_eq!(clean.counts.dropped["not_priority"], 1);
    assert!(clean.roots(false).asks(0, 5));
    let fallen = GameRoots::from_game(&game(1)).unwrap();
    assert!(fallen.events.is_empty());
    assert_eq!(fallen.counts.dropped["game_fallback"], 3);
    assert!(GameRoots::from_game(&json!({"fallback_answers": 0})).is_err());
}

/// What a root is left out for, and that leaving one out is no error: a
/// refused answer has no record input, one world no standard error.
#[test]
fn a_refused_an_unbound_and_a_one_world_event_are_counted_and_left_out() {
    let event = json!({"accepted": true, "seat": 0, "record_input_n": 5,
        "action": "PassPriority", "candidates": ["PassPriority", {"PlayLand": {"card": 3}}],
        "scripts": [null, null], "worlds": [[0.5, 0.6], [0.5, 0.4]], "chosen": 0,
        "leaves": {"stopped": 0}, "diffs": [0.0, 0.0], "ses": [null, 0.1]});
    let mut refused = event.clone();
    refused["accepted"] = json!(false);
    refused.as_object_mut().unwrap().remove("record_input_n");
    let mut unbound = event.clone();
    unbound["record_input_n"] = json!(null);
    let mut one_world = event.clone();
    one_world["record_input_n"] = json!(9);
    one_world["worlds"] = json!([[0.5, 0.6]]);
    let game = json!({"fallback_answers": 0, "refused_answers": 0,
        "search_events": [event, refused, unbound, one_world]});
    let roots = GameRoots::from_game(&game).unwrap();
    assert_eq!(roots.events.keys().collect::<Vec<_>>(), [&(0, 5)]);
    assert_eq!(
        roots.counts.dropped,
        BTreeMap::from([
            ("not_accepted".to_owned(), 1),
            ("one_world".to_owned(), 1),
            ("unbound".to_owned(), 1),
        ])
    );
    assert_eq!(roots.counts.events, 4);
}
