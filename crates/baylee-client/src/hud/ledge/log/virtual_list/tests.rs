use super::*;
use bevy::ecs::system::SystemState;

fn line(index: usize) -> LogLine {
    LogLine {
        at: index as u64,
        players: Vec::new(),
        index,
        turn: 1,
        times: 1,
        header: false,
        text: format!("line {index}"),
        names: Vec::new(),
        ability: None,
        subject: None,
        crowned: false,
        phase: None,
        follows: false,
    }
}

fn lines(count: usize) -> Vec<Entry> {
    (0..count)
        .map(|order| Entry::Line {
            order,
            line: line(order),
        })
        .collect()
}

fn list(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            LogList,
            LogFollow::default(),
            ScrollPosition::default(),
            ComputedNode {
                size: Vec2::new(360.0, 400.0),
                ..ComputedNode::default()
            },
        ))
        .id()
}

type TestSystem<'a> = SystemState<(
    Commands<'a, 'a>,
    Query<'a, 'a, (&'a mut LogFollow, &'a mut ScrollPosition), With<LogList>>,
    Query<'a, 'a, &'a ComputedNode>,
    Query<'a, 'a, &'a mut Node, With<LogSpacer>>,
)>;

fn sync(app: &mut App, rows: &mut VirtualLog, list: Entity) {
    let fonts = UiFonts::default();
    let mut state: TestSystem = SystemState::new(app.world_mut());
    {
        let (mut commands, mut lists, nodes, mut spacers) = state
            .get_mut(app.world_mut())
            .expect("the log's test queries are valid");
        let (mut follow, mut scroll) = lists.single_mut().unwrap();
        rows.sync(
            &mut commands,
            list,
            &fonts,
            Lang::En,
            None,
            &PANEL_INKS,
            &mut follow,
            &mut scroll,
            &nodes,
            &mut spacers,
            &mut |_, _, _, _| {},
        );
    }
    state.apply(app.world_mut());
}

fn children(app: &App, list: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(list)
        .map_or_else(Vec::new, |kids| kids.iter().collect())
}

#[test]
fn thousands_of_lines_mount_only_a_bounded_window_and_follow_appends() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    rows.replace(lines(6_239));
    sync(&mut app, &mut rows, list);
    assert!(rows.mounted.len() <= MAX_MOUNTED);
    assert_eq!(rows.range.end, 6_239);
    assert!(rows.spacers.is_some());
    assert_eq!(children(&app, list).len(), rows.mounted.len() + 2);
    let kept = rows.mounted[&6_238];

    rows.append(vec![line_entry_for_test(6_239)]);
    sync(&mut app, &mut rows, list);
    assert!(rows.mounted.len() <= MAX_MOUNTED);
    assert!(rows.mounted.contains_key(&6_239));
    assert_eq!(rows.mounted[&6_238], kept, "an append remounted an old row");
}

fn line_entry_for_test(order: usize) -> Entry {
    Entry::Line {
        order,
        line: line(order),
    }
}

#[test]
fn an_arbitrary_scroll_position_and_measured_heights_keep_their_rows() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    rows.replace(lines(6_239));
    sync(&mut app, &mut rows, list);
    app.world_mut().get_mut::<LogFollow>(list).unwrap().stuck = false;
    app.world_mut().get_mut::<ScrollPosition>(list).unwrap().y = rows.offset(42);
    sync(&mut app, &mut rows, list);
    assert!(rows.range.contains(&42));
    let entity = rows.mounted[&42];
    app.world_mut().entity_mut(entity).insert(ComputedNode {
        size: Vec2::new(10.0, 80.0),
        ..ComputedNode::default()
    });
    sync(&mut app, &mut rows, list);
    assert_eq!(rows.heights[42], 80.0);
    assert_eq!(rows.mounted[&42], entity);
    assert!(rows.mounted.len() <= MAX_MOUNTED);
}

#[test]
fn a_phase_caption_never_replaces_its_log_line() {
    let mut first = line(0);
    first.phase = Some(Phase::FirstMain);
    let (entries, _) = entries(&[first.clone()]);
    assert_eq!(
        entries,
        vec![
            Entry::Phase(Phase::FirstMain),
            Entry::Line {
                order: 0,
                line: first,
            },
        ]
    );
}

#[test]
fn spacers_reproduce_the_full_height_without_double_flex_gaps() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    rows.replace(lines(500));
    app.world_mut().get_mut::<LogFollow>(list).unwrap().stuck = false;
    sync(&mut app, &mut rows, list);
    let (before, after) = rows.spacer_heights();
    let mounted: f32 = rows.range.clone().map(|index| rows.heights[index]).sum();
    let internal_gaps = LOG_ROW_GAP * (rows.range.len() - 1) as f32;
    let spacer_gaps = LOG_ROW_GAP * (usize::from(before > 0.0) + usize::from(after > 0.0)) as f32;
    let occupied = before + mounted + internal_gaps + after + spacer_gaps;
    assert!((occupied - rows.total()).abs() < 0.01);
}

#[test]
fn measuring_overscan_keeps_the_visible_line_at_the_same_offset() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    rows.replace(lines(500));
    app.world_mut().get_mut::<LogFollow>(list).unwrap().stuck = false;
    app.world_mut().get_mut::<ScrollPosition>(list).unwrap().y = rows.offset(42) + 7.0;
    sync(&mut app, &mut rows, list);
    let above = rows.mounted[&40];
    app.world_mut().entity_mut(above).insert(ComputedNode {
        size: Vec2::new(300.0, 80.0),
        ..ComputedNode::default()
    });
    sync(&mut app, &mut rows, list);
    let scroll = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert!(
        (scroll - rows.offset(42) - 7.0).abs() < 0.01,
        "measuring an offscreen row moved the visible text: {scroll}"
    );
}

#[test]
fn replacing_a_measured_log_with_a_shorter_book_discards_the_old_rows() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    rows.replace(lines(500));
    sync(&mut app, &mut rows, list);
    for entity in rows.mounted.values() {
        app.world_mut().entity_mut(*entity).insert(ComputedNode {
            size: Vec2::new(300.0, 80.0),
            ..default()
        });
    }
    for count in [2, 0] {
        rows.replace(lines(count));
        sync(&mut app, &mut rows, list);
        assert_eq!(rows.mounted.len(), count);
        assert!(
            rows.heights
                .iter()
                .all(|height| *height == ESTIMATED_HEIGHT)
        );
        assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
    }
}

/// A book of `turns` turns of `per` lines each: every turn a heading, its
/// first line opening its first main phase.
fn turns(turns: u32, per: usize) -> Vec<Entry> {
    let mut book = Vec::new();
    let mut order = 0;
    let mut run = PhaseRun::default();
    for turn in 1..=turns {
        let mut heading = line(order);
        heading.turn = turn;
        heading.header = true;
        heading.text = format!("Turn {turn}");
        book.extend(append_entries(&[heading], order, &mut run));
        order += 1;
        for at in 0..per {
            let mut body = line(order);
            body.turn = turn;
            body.phase = Some(if at == 0 {
                Phase::FirstMain
            } else {
                Phase::Combat
            });
            book.extend(append_entries(&[body], order, &mut run));
            order += 1;
        }
    }
    book
}

/// What a folded list shows, read back: each turn as `T<turn>+` (open) or
/// `T<turn>-<lines>` (folded), and how many other rows follow it.
fn shape(entries: &[Entry]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in entries {
        if let Entry::Turn {
            line, open, lines, ..
        } = entry
        {
            out.push(if *open {
                format!("T{}+", line.turn)
            } else {
                format!("T{}-{lines}", line.turn)
            });
        } else {
            let last = out.last_mut().expect("a row under a heading");
            last.push('.');
        }
    }
    out
}

/// Feedback of 24.10.2026: every turn but the newest stands folded, saying
/// how many lines it holds, and a folded turn's phase captions go with it.
#[test]
fn every_turn_but_the_newest_is_folded_with_its_count() {
    let book = turns(3, 4);
    let folds = LogFolds::default();
    // Turn 3 open: its four lines and its two phase captions.
    assert_eq!(shape(&fold(&book, &folds)), ["T1-4", "T2-4", "T3+......"]);

    let mut folds = LogFolds::default();
    folds.set(1, true);
    folds.set(3, false);
    assert_eq!(shape(&fold(&book, &folds)), ["T1+......", "T2-4", "T3-4"]);
}

/// A list that folds appends a line of the newest, open turn without
/// remounting a row; the next turn's heading folds the turn before it.
#[test]
fn a_new_turn_folds_the_one_before_and_lines_append_in_place() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    let mut folds = LogFolds::default();
    let book = turns(2, 3);
    let len = book.len();
    rows.replace(book);
    rows.fold_with(&folds, 0.0);
    sync(&mut app, &mut rows, list);
    assert_eq!(shape(&rows.entries), ["T1-3", "T2+....."]);
    let kept = rows.mounted.clone();

    let mut more = line(len);
    more.turn = 2;
    rows.append(vec![Entry::Line {
        order: len,
        line: more,
    }]);
    rows.fold_with(&folds, 0.0);
    sync(&mut app, &mut rows, list);
    assert_eq!(shape(&rows.entries), ["T1-3", "T2+......"]);
    for (index, entity) in kept {
        assert_eq!(
            rows.mounted.get(&index),
            Some(&entity),
            "row {index} remounted"
        );
    }

    let mut heading = line(len + 1);
    heading.turn = 3;
    heading.header = true;
    rows.append(vec![Entry::Line {
        order: len + 1,
        line: heading,
    }]);
    rows.fold_with(&folds, 0.0);
    sync(&mut app, &mut rows, list);
    assert_eq!(shape(&rows.entries), ["T1-3", "T2-4", "T3+"]);

    folds.set(1, true);
    rows.fold_with(&folds, 0.0);
    sync(&mut app, &mut rows, list);
    assert_eq!(shape(&rows.entries), ["T1+.....", "T2-4", "T3+"]);
}

/// Folding a turn above the row the player is reading keeps that row where
/// it stood; folding the turn it is in puts that turn's heading there.
#[test]
fn a_refold_keeps_the_row_being_read() {
    let mut app = App::new();
    let list = list(&mut app);
    let mut rows = VirtualLog::default();
    let mut folds = LogFolds::default();
    for turn in 1..=40 {
        folds.set(turn, true);
    }
    rows.replace(turns(40, 6));
    rows.fold_with(&folds, 0.0);
    sync(&mut app, &mut rows, list);
    // Read a line of turn 30, part way into its row.
    let reading = rows
        .entries
        .iter()
        .position(
            |entry| matches!(entry, Entry::Line { line, .. } if line.turn == 30 && !line.header),
        )
        .expect("a line of turn 30");
    let key = row_key(&rows.entries, reading);
    let at = rows.offset(reading) + 5.0;
    app.world_mut().get_mut::<LogFollow>(list).unwrap().stuck = false;
    app.world_mut().get_mut::<ScrollPosition>(list).unwrap().y = at;
    sync(&mut app, &mut rows, list);

    folds.set(3, false);
    let y = app.world().get::<ScrollPosition>(list).unwrap().y;
    rows.fold_with(&folds, y);
    sync(&mut app, &mut rows, list);
    let now = (0..rows.entries.len())
        .find(|&i| row_key(&rows.entries, i) == key)
        .expect("the row is still shown");
    let y = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert!(
        (y - (rows.offset(now) + 5.0)).abs() < 0.01,
        "the row read moved: at {y}, the row at {}",
        rows.offset(now)
    );

    folds.set(30, false);
    rows.fold_with(&folds, y);
    sync(&mut app, &mut rows, list);
    let heading = rows
        .entries
        .iter()
        .position(|entry| matches!(entry, Entry::Turn { line, .. } if line.turn == 30))
        .expect("turn 30's heading");
    let y = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert!(
        (y - rows.offset(heading)).abs() < 0.01,
        "folded away: its heading stands there"
    );
}

/// A press on a turn's heading opens or folds it, and nothing else writes
/// the folds; another game forgets them.
#[test]
fn a_press_on_a_heading_opens_and_folds_its_turn() {
    let mut app = App::new();
    app.init_resource::<LogFolds>();
    let mut heading = line(0);
    heading.turn = 4;
    heading.header = true;
    let fonts = UiFonts::default();
    let row = {
        let mut state: SystemState<Commands> = SystemState::new(app.world_mut());
        let mut commands = state.get_mut(app.world_mut()).unwrap();
        let row = spawn_turn(
            &mut commands,
            &fonts,
            Lang::En,
            &heading,
            &PANEL_INKS,
            false,
            12,
        );
        state.apply(app.world_mut());
        row
    };
    let click = |app: &mut App, row: Entity| {
        use bevy::camera::NormalizedRenderTarget;
        use bevy::picking::pointer::{Location, PointerId};
        use bevy::window::WindowRef;
        app.world_mut().trigger(crate::test_pointer::aim(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Window(
                    WindowRef::Primary
                        .normalize(Some(Entity::PLACEHOLDER))
                        .expect("a window reference"),
                ),
                position: Vec2::ZERO,
            },
            crate::test_pointer::Click {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: std::time::Duration::ZERO,
                count: 1,
            },
            row,
        ));
    };
    let list = list(&mut app);
    app.world_mut().entity_mut(list).add_child(row);
    click(&mut app, row);
    let folds = app.world().resource::<LogFolds>();
    assert!(folds.open(4, false), "the folded turn opened");
    assert_eq!(folds.generation(), 1);
    assert!(
        !app.world().get::<LogFollow>(list).unwrap().stuck,
        "the list stops following its end, so the heading stays put"
    );

    let mut folds = app.world_mut().resource_mut::<LogFolds>();
    folds.about((1, 0));
    assert!(!folds.open(4, false), "another game starts folded again");
}
