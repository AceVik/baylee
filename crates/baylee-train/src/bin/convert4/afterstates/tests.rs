use super::*;

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
    assert_eq!((own, own_se), (0.0, 0.0));
    let (single, single_se) = half(&worlds[..1], 1, 0);
    assert!((single - 0.1).abs() < 1e-12 && single_se.is_nan());
    assert!(half(&worlds[..1], 1, 1).0.is_nan());
}

#[test]
fn a_planned_cast_names_the_spell_its_taps_pay_for() {
    let tap = PlayerAction::ActivateManaAbility {
        source: baylee_core::ids::ObjectId::new(70, 0),
    };
    let cast = PlayerAction::CastSpell {
        card: baylee_core::ids::ObjectId::new(61, 0),
    };
    let expected = serde_json::to_value(baylee_core::ids::ObjectId::new(61, 0))
        .unwrap()
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .unwrap();
    assert_eq!(cast_card(&tap, Some(&cast)), expected);
    assert_eq!(cast_card(&cast, None), expected);
    assert_eq!(cast_card(&tap, None), u32::MAX);
    assert_eq!(cast_card(&PlayerAction::PassPriority, None), u32::MAX);
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
    let header = write(
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
    assert_eq!(header["contract"], AFTERSTATE_CONTRACT);
}
