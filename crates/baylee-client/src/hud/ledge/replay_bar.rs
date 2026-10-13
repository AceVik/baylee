//! The transport bar (`.claude/b8/REPLAY-DESIGN.md` §7.2): what stands in
//! the shelf's middle while the table plays a replay.
//!
//! Two rows over the shelf's middle column, between the hand tools and the
//! tray's doors: the timeline (a cell per turn as wide as its steps, the
//! weaker checks as bands, the playhead) and the controls (start, a turn
//! back, a moment back, play/pause, a moment on, a turn on, the end; the
//! speed; where the step stands; the perspective). A loading veil, a
//! refusal, and the sentence of a tape that stops early stand over it.
//!
//! Its own root (`DetachedHud`) and revision ([`ReplayBarRevision`]): the
//! tree is rebuilt only when what it draws changes (play, speed, a new
//! tape); the playhead and the step's words are written in place every
//! frame, guarded, so a playing replay rebuilds nothing.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;

use crate::replay::{Loading, ReplayAsk, ReplayStatus};
use baylee_client_core::automation::RailRow;
use baylee_client_core::replay::words::ReplayPhrase;
use baylee_client_core::replay::{Check, Command, StopReason};

/// How tall the timeline row is.
const SCRUB_H: f32 = 14.0;
/// How tall the controls row is.
const CONTROLS_H: f32 = LEDGE_H - 6.0;
/// A control's side.
const CONTROL_W: f32 = 30.0;

/// What the bar's controls and keys asked of the replay this frame, carried
/// out by [`drive`] against the host.
#[derive(Resource, Default)]
pub struct ReplayAsks(pub Vec<ReplayAsk>);

/// A drag along the timeline: the step it is over, sent at most every
/// 100 ms and on release (§7.2).
#[derive(Resource, Default)]
pub struct ScrubDrag {
    pending: Option<usize>,
    since: f32,
}

/// The bar's root.
#[derive(Component)]
pub struct ReplayBar;

/// The sheet over the bar: loading, a refusal, a tape that stops early.
#[derive(Component)]
pub struct ReplaySheet;

/// The timeline row.
#[derive(Component)]
pub struct Scrubber;

/// The playhead on it.
#[derive(Component)]
pub struct Playhead;

/// Where the step stands, in words.
#[derive(Component)]
pub struct WhereLabel;

/// What the bar's tree draws: compared whole, rebuilt on a difference.
#[derive(Clone, PartialEq, Debug)]
pub struct BarLook {
    steps: usize,
    timeline: Option<usize>,
    playing: bool,
    speed: baylee_client_core::replay::Speed,
    perspective: baylee_client_core::replay::Perspective,
    several: bool,
    loading: Option<Loading>,
    problem: Option<String>,
    stopped: Option<String>,
    play_on: bool,
    lang: Lang,
    window_w: i32,
    seat: PlayerId,
}

/// The bar and its sheet: what a rebuild takes down.
type BarParts = Or<(With<ReplayBar>, With<ReplaySheet>)>;

/// The bar's revision.
#[derive(Resource, Default)]
pub struct ReplayBarRevision(Option<BarLook>);

impl BarLook {
    fn of(status: &ReplayStatus, lang: Lang, window_w: i32) -> Self {
        let offers = status.offers;
        let several = [offers.seat, offers.public, offers.all_hands]
            .iter()
            .filter(|o| **o)
            .count()
            > 1;
        let (stopped, play_on) = stop_words(status, lang);
        Self {
            steps: status.steps,
            timeline: status
                .timeline
                .as_ref()
                .map(|t| std::sync::Arc::as_ptr(&t.0) as usize),
            playing: status.playing,
            speed: status.speed,
            perspective: status.perspective,
            several,
            loading: status.loading.clone(),
            problem: status.problem.clone(),
            stopped: stopped.filter(|_| status.at_end()),
            play_on,
            lang,
            window_w,
            seat: status.perspective.seat(),
        }
    }
}

/// What a tape that stops before the game's end says at its last step, and
/// whether "Play on anyway" is offered (a divergence under `Verify::Stop`).
pub(crate) fn stop_words(status: &ReplayStatus, lang: Lang) -> (Option<String>, bool) {
    let Some(stop) = status.stop else {
        return (None, false);
    };
    let turn = status
        .timeline
        .as_ref()
        .and_then(|t| t.0.step(t.0.last()))
        .map_or(0, |m| m.place.turn)
        .to_string();
    let header = status.header.as_ref();
    let recorded_by = header.map_or("", |h| h.recorded_by.as_str());
    match stop.stop {
        StopReason::End | StopReason::Cancelled | StopReason::TooLarge => (None, false),
        StopReason::Cut => (Some(ReplayPhrase::StoppedCut.text(lang).to_string()), false),
        StopReason::Refused => (
            Some(ReplayPhrase::StoppedRefused.fill(lang, &[&turn])),
            false,
        ),
        StopReason::Diverged => {
            let same_build = header.is_some_and(|h| h.build == h.recorded_by);
            let by_hash = stop
                .diverged
                .is_some_and(|d| d.by == baylee_client_core::replay::DivergedBy::Hash);
            if same_build && by_hash {
                (
                    Some(ReplayPhrase::StoppedMismatch.text(lang).to_string()),
                    false,
                )
            } else {
                (
                    Some(ReplayPhrase::StoppedDiverged.fill(lang, &[&turn, recorded_by])),
                    true,
                )
            }
        }
    }
}

/// Where the step stands, in words: "Turn 7 · Bob · Blockers".
pub(crate) fn where_words(duel: &Duel, status: &ReplayStatus, lang: Lang) -> String {
    let Some(mark) = status.timeline.as_ref().and_then(|t| t.0.step(status.step)) else {
        return String::new();
    };
    let place = mark.place;
    let who = duel
        .statics
        .as_ref()
        .and_then(|s| s.seats.iter().find(|seat| seat.player == place.active))
        .map_or_else(
            || format!("{}", place.active.get() + 1),
            |seat| seat.display_name.clone(),
        );
    let step = RailRow::current(place.phase, place.step).name().text(lang);
    ReplayPhrase::WhereStep.fill(lang, &[&place.turn.to_string(), &who, step])
}

/// Builds the bar when what it draws changed; takes it down when the table
/// plays no replay.
#[allow(clippy::too_many_arguments)]
pub fn sync_replay_bar(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
    mut revision: ResMut<ReplayBarRevision>,
    bars: Query<Entity, BarParts>,
    phase: Option<Res<State<crate::DuelPhase>>>,
) {
    let lang = Lang::of(&settings.lang);
    let window_w =
        crate::hud::scale::window_space(&windows, ui.as_deref()).map_or(1200, |w| w.x as i32);
    // The duel keeps its last replay status after the table closed, until
    // the next opens: a closed table draws no bar.
    let closed = phase.is_some_and(|p| *p.get() == crate::DuelPhase::Closed);
    let look = duel
        .replay
        .as_ref()
        .filter(|_| !closed)
        .map(|status| BarLook::of(status, lang, window_w));
    let standing = !bars.is_empty();
    if revision.0 == look && standing == look.is_some() {
        return;
    }
    for bar in &bars {
        commands.entity(bar).despawn();
    }
    revision.0.clone_from(&look);
    let (Some(look), Some(status)) = (look, duel.replay.as_ref().filter(|_| !closed)) else {
        return;
    };
    spawn_bar(&mut commands, &fonts, &look, status);
    spawn_sheet(&mut commands, &fonts, &look);
}

#[allow(clippy::too_many_lines)] // one bar, its two rows in order
fn spawn_bar(commands: &mut Commands, fonts: &UiFonts, look: &BarLook, status: &ReplayStatus) {
    let lang = look.lang;
    let root = commands
        .spawn((
            ReplayBar,
            DetachedHud,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(HAND_ZONE_H - LEDGE_H),
                left: px(tools_reserved(look.window_w)),
                right: px(RIGHT_RESERVED),
                height: px(LEDGE_H + SCRUB_H + 4.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(2.0),
                padding: UiRect::axes(px(8.0), px(3.0)),
                border_radius: BorderRadius::top(px(8.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            GlobalZIndex(3),
        ))
        .id();

    // ---- the timeline
    let scrub = commands
        .spawn((
            Scrubber,
            Node {
                width: percent(100),
                height: px(SCRUB_H),
                flex_direction: FlexDirection::Row,
                column_gap: px(1.0),
                ..default()
            },
            BackgroundColor(palette::SEAT_TRACK),
        ))
        .observe(seek_on_press)
        .observe(seek_on_drag)
        .observe(seek_on_release)
        .id();
    commands.entity(root).add_child(scrub);
    if let Some(timeline) = status.timeline.as_ref().map(|t| &t.0) {
        for turn in timeline.turns() {
            let own = turn.active == look.seat;
            let cell = commands
                .spawn((
                    Node {
                        flex_grow: turn.len as f32,
                        flex_basis: px(0),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(if own {
                        palette::PARCHMENT.with_alpha(0.55)
                    } else {
                        palette::PANEL_HOT
                    }),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(scrub).add_child(cell);
        }
        let steps = timeline.len().max(1) as f32;
        for band in timeline.bands() {
            let left = band.from as f32 / steps * 100.0;
            let width = (band.to - band.from) as f32 / steps * 100.0;
            let (top, height, colour) = match band.check {
                Check::Seen => (SCRUB_H - 2.0, 2.0, palette::MUTED),
                _ => (0.0, SCRUB_H, palette::DANGER.with_alpha(0.35)),
            };
            let node = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(left),
                        width: percent(width),
                        top: px(top),
                        height: px(height),
                        justify_content: JustifyContent::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(colour),
                    Pickable::IGNORE,
                ))
                .id();
            if band.check == Check::Unverified && width > 12.0 {
                let words = commands
                    .spawn((
                        Text::new(ReplayPhrase::UnverifiedBand.text(lang)),
                        tf(fonts, 9.0),
                        TextColor(palette::INK),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(node).add_child(words);
            }
            commands.entity(scrub).add_child(node);
        }
        if timeline
            .stopped()
            .is_some_and(|s| s != StopReason::Cancelled)
        {
            let mark = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        width: px(3.0),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(palette::DANGER),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(scrub).add_child(mark);
        }
    }
    let head = commands
        .spawn((
            Playhead,
            Node {
                position_type: PositionType::Absolute,
                left: percent(0),
                top: px(-2.0),
                width: px(3.0),
                height: px(SCRUB_H + 4.0),
                ..default()
            },
            BackgroundColor(palette::ACTIVE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(scrub).add_child(head);

    // ---- the controls
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(CONTROLS_H),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(row);
    let play = if look.playing {
        ('\u{f04c}', ReplayPhrase::Pause)
    } else {
        ('\u{f04b}', ReplayPhrase::Play)
    };
    let controls = [
        ('\u{f049}', ReplayPhrase::Start, Command::Start),
        ('\u{f04a}', ReplayPhrase::PrevTurn, Command::Turn(-1)),
        ('\u{f048}', ReplayPhrase::StepBack, Command::Step(-1)),
        (play.0, play.1, Command::Toggle),
        ('\u{f051}', ReplayPhrase::StepOn, Command::Step(1)),
        ('\u{f04e}', ReplayPhrase::NextTurn, Command::Turn(1)),
        ('\u{f050}', ReplayPhrase::End, Command::End),
    ];
    for (glyph, words, command) in controls {
        let button = control(
            commands,
            fonts,
            Glyph::Icon(glyph),
            ReplayAsk::Transport(command),
            words.text(lang),
        );
        commands.entity(row).add_child(button);
    }
    let gap = commands
        .spawn(Node {
            width: px(10.0),
            ..default()
        })
        .id();
    commands.entity(row).add_child(gap);
    let slower = control(
        commands,
        fonts,
        Glyph::Icon(crate::hud::glyph::MINUS),
        ReplayAsk::Transport(Command::Slower),
        ReplayPhrase::Speed.text(lang),
    );
    let speed = commands
        .spawn((
            Text::new(look.speed.label()),
            tf_bold(fonts, LABEL_PT),
            TextColor(palette::DOCK_INK),
            Node {
                min_width: px(34.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let faster = control(
        commands,
        fonts,
        Glyph::Icon(crate::hud::glyph::PLUS),
        ReplayAsk::Transport(Command::Faster),
        ReplayPhrase::Speed.text(lang),
    );
    commands.entity(row).add_children(&[slower, speed, faster]);
    let place = commands
        .spawn((
            WhereLabel,
            Text::default(),
            tf(fonts, LABEL_PT),
            TextColor(palette::LEDGE_SOFT),
            TextLayout::justify(Justify::Center).with_no_wrap(),
            Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(place);
    let pill_words = crate::replay::perspective_words(look.perspective, lang);
    if look.several {
        let pill = control(
            commands,
            fonts,
            Glyph::Words(format!("{pill_words} \u{25be}")),
            ReplayAsk::NextPerspective,
            pill_words,
        );
        commands.entity(row).add_child(pill);
    } else {
        let words = commands
            .spawn((
                Text::new(pill_words),
                tf(fonts, LABEL_PT),
                TextColor(palette::LEDGE_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(words);
    }
}

/// What a control shows.
enum Glyph {
    Icon(char),
    Words(String),
}

/// One control of the bar: a press is an ask for [`drive`].
fn control(
    commands: &mut Commands,
    fonts: &UiFonts,
    glyph: Glyph,
    ask: ReplayAsk,
    hint: &str,
) -> Entity {
    let (text, font, width) = match glyph {
        Glyph::Icon(c) => (c.to_string(), icon_tf(fonts, 12.0), px(CONTROL_W)),
        Glyph::Words(w) => (w, tf(fonts, LABEL_PT), Val::Auto),
    };
    commands
        .spawn((
            Button,
            Node {
                min_width: width,
                height: px(CONTROLS_H - 6.0),
                padding: UiRect::horizontal(px(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(5.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Feel::new(palette::PANEL_LIT),
            crate::hud::Hint(hint.to_string()),
        ))
        .with_child((
            Text::new(text),
            font,
            TextColor(palette::DOCK_INK),
            Pickable::IGNORE,
        ))
        .observe(
            move |mut click: On<Pointer<Click>>, mut asks: ResMut<ReplayAsks>| {
                click.propagate(false);
                asks.0.push(ask);
            },
        )
        .id()
}

/// The sheet over the bar, when there is something to say.
#[allow(clippy::too_many_lines)] // one sheet: its words, then its ways
fn spawn_sheet(commands: &mut Commands, fonts: &UiFonts, look: &BarLook) {
    let lang = look.lang;
    let (words, play_on, cancel) = if let Some(problem) = &look.problem {
        (ReplayPhrase::Failed.fill(lang, &[problem]), false, false)
    } else if let Some(loading) = &look.loading {
        let words = match loading {
            Loading::Here { percent } => {
                ReplayPhrase::RebuildingHere.fill(lang, &[&percent.unwrap_or(0).to_string()])
            }
            Loading::Gateway { name } => ReplayPhrase::RebuildingThere.fill(lang, &[name]),
        };
        (words, false, true)
    } else if let Some(stopped) = &look.stopped {
        (stopped.clone(), look.play_on, false)
    } else {
        return;
    };
    let sheet = commands
        .spawn((
            ReplaySheet,
            DetachedHud,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(HAND_ZONE_H + SCRUB_H + 24.0),
                left: percent(25),
                right: percent(25),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(10.0),
                padding: UiRect::all(px(14.0)),
                border_radius: BorderRadius::all(px(8.0)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            GlobalZIndex(4),
        ))
        .id();
    let text = commands
        .spawn((
            Text::new(words),
            tf(fonts, SENTENCE_PT),
            TextColor(palette::DIALOG_INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(text);
    let row = commands
        .spawn((
            Node {
                column_gap: px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(row);
    if play_on {
        let button = control(
            commands,
            fonts,
            Glyph::Words(ReplayPhrase::PlayOnAnyway.text(lang).to_string()),
            ReplayAsk::PlayOn,
            ReplayPhrase::PlayOnAnyway.text(lang),
        );
        commands.entity(row).add_child(button);
    }
    if cancel {
        let hint = commands
            .spawn((
                Text::new(ReplayPhrase::EscCancels.text(lang)),
                tf(fonts, LABEL_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(hint);
    } else {
        let back = commands
            .spawn((
                Button,
                Node {
                    height: px(CONTROLS_H - 6.0),
                    padding: UiRect::horizontal(px(10.0)),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(5.0)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Feel::new(palette::PANEL_LIT),
            ))
            .with_child((
                Text::new(ReplayPhrase::BackToReplays.text(lang)),
                tf(fonts, LABEL_PT),
                TextColor(palette::DOCK_INK),
                Pickable::IGNORE,
            ))
            .observe(|mut click: On<Pointer<Click>>, mut duel: ResMut<Duel>| {
                click.propagate(false);
                duel.leave_asked = true;
            })
            .id();
        commands.entity(row).add_child(back);
    }
}

/// Writes the playhead's place and the step's words, where they moved.
pub fn place_playhead(
    duel: Res<Duel>,
    settings: Res<crate::settings::ClientSettings>,
    mut heads: Query<&mut Node, With<Playhead>>,
    mut labels: Query<&mut Text, With<WhereLabel>>,
) {
    let Some(status) = duel.replay.as_ref() else {
        return;
    };
    let fraction = status
        .timeline
        .as_ref()
        .map_or(0.0, |t| t.0.fraction_of(status.step));
    let left = percent(fraction * 100.0);
    for mut node in &mut heads {
        if node.left != left {
            node.left = left;
        }
    }
    let words = where_words(&duel, status, Lang::of(&settings.lang));
    for mut text in &mut labels {
        if text.0 != words {
            text.0.clone_from(&words);
        }
    }
}

/// The step under the pointer on the timeline.
fn step_under(
    entity: Entity,
    x: f32,
    nodes: &Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: &Query<&Window>,
    duel: &Duel,
) -> Option<usize> {
    let (node, place) = nodes.get(entity).ok()?;
    let scale = windows.iter().next().map_or(1.0, Window::scale_factor);
    let width = node.size().x;
    let left = place.translation.x - width / 2.0;
    let fraction = if width > 0.0 {
        (x * scale - left) / width
    } else {
        0.0
    };
    let timeline = &duel.replay.as_ref()?.timeline.as_ref()?.0;
    Some(timeline.step_at(fraction))
}

fn seek_on_press(
    mut press: On<Pointer<Press>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
    duel: Res<Duel>,
    mut asks: ResMut<ReplayAsks>,
) {
    press.propagate(false);
    if let Some(step) = step_under(
        press.entity,
        press.pointer_location.position.x,
        &nodes,
        &windows,
        &duel,
    ) {
        asks.0.push(ReplayAsk::Transport(Command::Seek(step)));
    }
}

fn seek_on_drag(
    drag: On<Pointer<Drag>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
    duel: Res<Duel>,
    mut pending: ResMut<ScrubDrag>,
) {
    if let Some(step) = step_under(
        drag.entity,
        drag.pointer_location.position.x,
        &nodes,
        &windows,
        &duel,
    ) {
        pending.pending = Some(step);
    }
}

fn seek_on_release(
    _release: On<Pointer<DragEnd>>,
    mut pending: ResMut<ScrubDrag>,
    mut asks: ResMut<ReplayAsks>,
) {
    if let Some(step) = pending.pending.take() {
        asks.0.push(ReplayAsk::Transport(Command::Seek(step)));
    }
    pending.since = 0.0;
}

/// Carries the frame's asks to the host, a drag's seek at most every
/// 100 ms, and the clock: the virtual one, so dev-control's `/pause` and
/// `/step` hold a replay too.
pub fn drive(
    time: Res<Time>,
    host: Option<ResMut<crate::InstalledHost>>,
    mut asks: ResMut<ReplayAsks>,
    mut drag: ResMut<ScrubDrag>,
) {
    let Some(mut host) = host else {
        return;
    };
    if host.0.replay().is_none() {
        return;
    }
    if !asks.0.is_empty() {
        for ask in std::mem::take(&mut asks.0) {
            host.0.control(ask);
        }
    }
    if drag.pending.is_some() {
        drag.since += time.delta_secs();
        if drag.since >= 0.1 {
            drag.since = 0.0;
            if let Some(step) = drag.pending.take() {
                host.0.control(ReplayAsk::Transport(Command::Seek(step)));
            }
        }
    }
    let dt = time.delta();
    if !dt.is_zero() {
        host.0.control(ReplayAsk::Transport(Command::Tick(dt)));
    }
}

/// The replay's keys (§7.5), while the table plays one: the borrowed
/// actions mean the replay's thing here (the live handler is handed them
/// stripped, `keys::REPLAY_BORROWS`), the new ones mean nothing elsewhere.
/// Over the end sheet `Space` and `Esc` stay the sheet's.
pub fn replay_keys(
    keys: Res<ButtonInput<KeyCode>>,
    prefs: Res<crate::prefs::Prefs>,
    duel: Res<Duel>,
    phase: Res<State<crate::DuelPhase>>,
    desk: Option<Res<crate::report::ReportDesk>>,
    mut asks: ResMut<ReplayAsks>,
) {
    let Some(status) = duel.replay.as_ref() else {
        return;
    };
    if desk.is_some_and(|desk| desk.holds_keyboard()) || duel.browser.is_typing() {
        return;
    }
    let fired = crate::keys::Fired::of(&keys, prefs.keymap());
    if fired.quiet() {
        return;
    }
    let finished = *phase.get() == crate::DuelPhase::Finished;
    let loading = status.loading.is_some();
    let found = crate::keys::replay_asks(&fired, finished, loading);
    if !found.is_empty() {
        asks.0.extend(found);
    }
}
