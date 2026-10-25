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
