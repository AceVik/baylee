//! The replay bar (`.claude/b8/REPLAY-DESIGN.md` §7.2, the owner's redesign
//! of 13.10.2026): a replay asks nothing, so the actions bar's whole place
//! is the replay's controls.
//!
//! ```text
//!  ╭──────────────────────────── timeline ────────────────────────────╮
//!  │ 1 │ 2  │    3     │ 4 │ ◆                 │ ░░ unverified ░░     │
//!  ╰──────────────────────────────────────────────────────────────────╯
//!  [½ 1 2 4 8]  Turn 3 · Bob · Blockers    ⏮ ⏪ ◀ (⏯) ▶ ⏩ ⏭    [👁 Your seat ⌃] [✋ ⌃]
//! ```
//!
//! Three groups in one row, the transport in the middle and the two sides
//! as wide as each other, so the play button stands on the bar's centre
//! line; over them the timeline, a cell per turn as wide as its steps with
//! its number in it, own turns ivory and the others' slate, the weaker
//! checks as bands, the playhead a brass needle. Every control lights under
//! the pointer and gives under a press (`ambience::Feel`).
//!
//! Its own root (`DetachedHud`) and revision ([`ReplayBarRevision`]): the
//! tree is rebuilt only when what it draws changes (play, speed, a new
//! tape, the picker, the hand); the playhead and the step's words are
//! written in place every frame, guarded. The root rides the hand's drawer
//! (`hand_drawer::RidesTheHand`), so it sinks with the hand when the hand
//! folds away.

#[allow(clippy::wildcard_imports)] // the ledge's shared vocabulary
use super::*;
use crate::ui_button::UiButton;

use crate::replay::{Loading, ReplayAsk, ReplayStatus};
use baylee_client_core::automation::RailRow;
use baylee_client_core::replay::words::ReplayPhrase;
use baylee_client_core::replay::{Check, Command, Perspective, Speed, StopReason};

/// How tall the timeline is.
pub const TIMELINE_H: f32 = 22.0;
/// The air between the timeline and the controls.
const ROW_GAP: f32 = 4.0;
/// How tall the whole bar stands over the shelf's bottom.
pub const BAR_H: f32 = LEDGE_H + TIMELINE_H + ROW_GAP + 4.0;
/// A transport button's side.
const KEY: f32 = 28.0;
/// The play button's side.
const PLAY: f32 = 34.0;
/// The brass the play button, the chosen speed and the playhead wear.
const BRASS: Color = palette::BRASS;

/// What the bar's controls and keys asked this frame, carried out by
/// [`drive`]: the replay's (the host's) and the bar's own.
#[derive(Resource, Default)]
pub struct ReplayAsks(pub Vec<BarAsk>);

/// One ask of the bar's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarAsk {
    /// The replay's: a transport command, a perspective.
    Host(ReplayAsk),
    /// Fold the hand away, or bring it back.
    ToggleHand,
    /// Open the seat picker, or shut it.
    TogglePicker,
}

/// What the bar keeps that is the viewer's, not the replay's: whether the
/// hand is folded by hand, and the seat picker.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct BarUi {
    /// The hand as the viewer set it, and the perspective it was set for:
    /// another perspective goes back to what it shows by itself.
    pub hand: Option<(Perspective, bool)>,
    /// Whether the seat picker stands open.
    pub picker: bool,
}

impl BarUi {
    /// Whether the hand stands open in a replay at `status`: what the
    /// perspective shows by itself (open where the hand is visible), or
    /// what the viewer chose for it.
    #[must_use]
    pub fn hand_open(&self, status: &ReplayStatus) -> bool {
        match self.hand {
            Some((perspective, open)) if perspective == status.perspective => open,
            _ => !status.hand_hidden(status.perspective),
        }
    }
}

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

/// The sheet over the bar (loading, a refusal, a tape that stops early),
/// and the seat picker.
#[derive(Component)]
pub struct ReplaySheet;

/// The timeline.
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
#[allow(clippy::struct_excessive_bools)] // independent facts the tree is drawn from, compared whole
pub struct BarLook {
    steps: usize,
    timeline: Option<usize>,
    playing: bool,
    speed: Speed,
    perspective: Perspective,
    choices: Vec<(Perspective, String, bool)>,
    picker: bool,
    hand_open: bool,
    hidden: bool,
    loading: Option<Loading>,
    problem: Option<String>,
    stopped: Option<String>,
    play_on: bool,
    lang: Lang,
    window_w: i32,
    viewer: PlayerId,
    /// What the right column reserves: more by the chat's door with a
    /// social session, which the bar keeps clear of as the shelf does.
    right: f32,
}

/// The bar's revision.
#[derive(Resource, Default)]
pub struct ReplayBarRevision(Option<BarLook>);

/// The bar and its sheets: what a rebuild takes down.
type BarParts = Or<(With<ReplayBar>, With<ReplaySheet>)>;

impl BarLook {
    fn of(status: &ReplayStatus, ui: &BarUi, lang: Lang, window_w: i32, chat_door: bool) -> Self {
        let (stopped, play_on) = stop_words(status, lang);
        let choices = status
            .choices()
            .into_iter()
            .map(|p| (p, status.words(p, lang), status.hand_hidden(p)))
            .collect();
        Self {
            steps: status.steps,
            timeline: status
                .timeline
                .as_ref()
                .map(|t| std::sync::Arc::as_ptr(&t.0) as usize),
            playing: status.playing,
            speed: status.speed,
            perspective: status.perspective,
            choices,
            picker: ui.picker,
            hand_open: ui.hand_open(status),
            hidden: status.hand_hidden(status.perspective),
            loading: status.loading.clone(),
            problem: status.problem.clone(),
            stopped: stopped.filter(|_| status.at_end()),
            play_on,
            lang,
            window_w,
            viewer: status.origin.viewer,
            right: RIGHT_RESERVED
                + if chat_door {
                    super::tray::CHAT_DOOR
                } else {
                    0.0
                },
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
    let who = seat_name(duel, place.active);
    let step = RailRow::current(place.phase, place.step).name().text(lang);
    ReplayPhrase::WhereStep.fill(lang, &[&place.turn.to_string(), &who, step])
}

/// What the table calls a seat.
fn seat_name(duel: &Duel, seat: PlayerId) -> String {
    duel.statics
        .as_ref()
        .and_then(|s| s.seats.iter().find(|s| s.player == seat))
        .map_or_else(|| format!("{}", seat.get() + 1), |s| s.display_name.clone())
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
    ui_scale: Option<Res<UiScale>>,
    ui: Res<BarUi>,
    mut revision: ResMut<ReplayBarRevision>,
    bars: Query<Entity, BarParts>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    social: Option<Res<crate::social::SocialHost>>,
) {
    let lang = Lang::of(&settings.lang);
    let chat_door = social.as_ref().is_some_and(|s| s.machine().is_some());
    let window_w =
        crate::hud::scale::window_space(&windows, ui_scale.as_deref()).map_or(1200, |w| w.x as i32);
    // The duel keeps its last replay status after the table closed, until
    // the next opens: a closed table draws no bar.
    let closed = phase.is_some_and(|p| *p.get() == crate::DuelPhase::Closed);
    let status = duel.replay.as_ref().filter(|_| !closed);
    let look = status.map(|status| BarLook::of(status, &ui, lang, window_w, chat_door));
    let standing = !bars.is_empty();
    if revision.0 == look && standing == look.is_some() {
        return;
    }
    for bar in &bars {
        commands.entity(bar).despawn();
    }
    revision.0.clone_from(&look);
    let (Some(look), Some(status)) = (look, status) else {
        return;
    };
    spawn_bar(&mut commands, &fonts, &duel, &look, status);
    spawn_sheet(&mut commands, &fonts, &look);
    if look.picker {
        spawn_picker(&mut commands, &fonts, &look);
    }
}

fn spawn_bar(
    commands: &mut Commands,
    fonts: &UiFonts,
    duel: &Duel,
    look: &BarLook,
    status: &ReplayStatus,
) {
    let root = commands
        .spawn((
            ReplayBar,
            DetachedHud,
            crate::hud::hand_drawer::RidesTheHand,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(HAND_ZONE_H - LEDGE_H),
                left: px(EDGE),
                right: px(look.right),
                height: px(BAR_H),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                row_gap: px(ROW_GAP),
                padding: UiRect::new(px(10.0), px(10.0), px(4.0), px(3.0)),
                ..default()
            },
            UiTransform::default(),
            GlobalZIndex(G_REPLAY_BAR),
        ))
        .id();
    let timeline = spawn_timeline(commands, fonts, duel, look, status);
    let controls = spawn_controls(commands, fonts, look);
    commands.entity(root).add_children(&[timeline, controls]);
}

/// The timeline: a cell per turn as wide as its steps, its number in it
/// where it has the room, the bands, the stop, the playhead.
#[allow(clippy::too_many_lines)] // one row, its layers in drawing order
fn spawn_timeline(
    commands: &mut Commands,
    fonts: &UiFonts,
    duel: &Duel,
    look: &BarLook,
    status: &ReplayStatus,
) -> Entity {
    let lang = look.lang;
    let track = commands
        .spawn((
            Scrubber,
            // As wide as the bar less the players' strip's room
            // (`place_playhead` writes the margin): stretched, not 100 %.
            Node {
                height: px(TIMELINE_H),
                flex_direction: FlexDirection::Row,
                column_gap: px(2.0),
                padding: UiRect::all(px(3.0)),
                border_radius: BorderRadius::all(px(TIMELINE_H / 2.0)),
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(palette::DOCK_GROUND.with_alpha(0.92)),
            BoxShadow::from(ShadowStyle {
                color: palette::SHADOW,
                x_offset: px(0.0),
                y_offset: px(2.0),
                spread_radius: px(0.0),
                blur_radius: px(6.0),
            }),
        ))
        .observe(seek_on_press)
        .observe(seek_on_drag)
        .observe(seek_on_release)
        .id();
    let Some(timeline) = status.timeline.as_ref().map(|t| &t.0) else {
        return track;
    };
    // How wide the track is, near enough, for which cells have room for
    // their number: the bar's width between its two ends.
    #[allow(clippy::cast_precision_loss)]
    let track_w = look.window_w as f32 - EDGE - look.right - 26.0;
    let steps = timeline.len().max(1) as f32;
    let last = timeline.turns().len().saturating_sub(1);
    for (i, turn) in timeline.turns().iter().enumerate() {
        let own = turn.active == look.viewer;
        let base = if own {
            palette::PARCHMENT.with_alpha(0.62)
        } else {
            palette::SEAT_STOP.with_alpha(0.16)
        };
        let radius = |end: bool| if end { px(8.0) } else { px(2.0) };
        let cell = commands
            .spawn((
                Node {
                    flex_grow: turn.len as f32,
                    flex_basis: px(0),
                    height: percent(100),
                    min_width: px(2.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    border_radius: BorderRadius {
                        top_left: radius(i == 0).into(),
                        bottom_left: radius(i == 0).into(),
                        top_right: radius(i == last).into(),
                        bottom_right: radius(i == last).into(),
                    },
                    ..default()
                },
                BackgroundColor(base),
                Feel::tinting_to(base, crate::ambience::lighter(base, 0.22)),
                crate::hud::Hint(ReplayPhrase::TurnHint.fill(
                    lang,
                    &[&turn.turn.to_string(), &seat_name(duel, turn.active)],
                )),
            ))
            .id();
        let room = turn.len as f32 / steps * track_w;
        if room >= 24.0 {
            let number = commands
                .spawn((
                    Text::new(turn.turn.to_string()),
                    tf_bold(fonts, 10.0),
                    TextColor(if own {
                        palette::PARCHMENT_INK
                    } else {
                        palette::LEDGE_SOFT
                    }),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(cell).add_child(number);
        }
        commands.entity(track).add_child(cell);
    }
    for band in timeline.bands() {
        let left = band.from as f32 / steps * 100.0;
        let width = (band.to - band.from) as f32 / steps * 100.0;
        let (top, height, colour) = match band.check {
            Check::Seen => (TIMELINE_H - 5.0, 2.0, palette::MUTED),
            _ => (3.0, TIMELINE_H - 6.0, palette::DANGER.with_alpha(0.30)),
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
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                    border_radius: BorderRadius::all(px(3.0)),
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
        commands.entity(track).add_child(node);
    }
    if timeline
        .stopped()
        .is_some_and(|s| s != StopReason::Cancelled)
    {
        let mark = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(-1.0),
                    top: px(-2.0),
                    width: px(4.0),
                    height: px(TIMELINE_H + 4.0),
                    border_radius: BorderRadius::all(px(2.0)),
                    ..default()
                },
                BackgroundColor(palette::DANGER),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(mark);
    }
    // The needle: a brass knob over a brass line through the track.
    let head = commands
        .spawn((
            Playhead,
            Node {
                position_type: PositionType::Absolute,
                left: percent(0),
                top: px(-5.0),
                width: px(12.0),
                height: px(TIMELINE_H + 10.0),
                margin: UiRect::left(px(-6.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|head| {
            head.spawn((
                Node {
                    width: px(12.0),
                    height: px(12.0),
                    border: UiRect::all(px(2.0)),
                    border_radius: BorderRadius::all(px(6.0)),
                    ..default()
                },
                BackgroundColor(BRASS),
                BorderColor::all(palette::DOCK_GROUND),
                Pickable::IGNORE,
            ));
            head.spawn((
                Node {
                    width: px(2.0),
                    flex_grow: 1.0,
                    margin: UiRect::top(px(-2.0)),
                    ..default()
                },
                BackgroundColor(BRASS),
                Pickable::IGNORE,
            ));
        })
        .id();
    commands.entity(track).add_child(head);
    track
}

/// One side of the controls: as wide as the other, so the transport stands
/// on the bar's centre line.
fn side(commands: &mut Commands, end: JustifyContent) -> Entity {
    commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: end,
                column_gap: px(8.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// The controls: speed and where the step stands left, the transport in
/// the middle, whose table and the hand right.
#[allow(clippy::too_many_lines)] // one row, three groups left to right
fn spawn_controls(commands: &mut Commands, fonts: &UiFonts, look: &BarLook) -> Entity {
    let lang = look.lang;
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(LEDGE_H - 6.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // ---- left: the speed, then where the step stands
    let left = side(commands, JustifyContent::FlexStart);
    let speeds = commands
        .spawn((
            Node {
                padding: UiRect::all(px(2.0)),
                column_gap: px(2.0),
                border_radius: BorderRadius::all(px(13.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DOCK_GROUND.with_alpha(0.85)),
            crate::hud::Hint(ReplayPhrase::Speed.text(lang).to_string()),
        ))
        .id();
    for speed in Speed::ALL {
        let on = speed == look.speed;
        let pill = pill(
            commands,
            fonts,
            &format!("{}\u{d7}", speed.short()),
            on,
            BarAsk::Host(ReplayAsk::Transport(Command::SetSpeed(speed))),
        );
        commands.entity(speeds).add_child(pill);
    }
    let place = commands
        .spawn((
            WhereLabel,
            Text::default(),
            tf(fonts, LABEL_PT),
            TextColor(palette::LEDGE_SOFT),
            TextLayout::no_wrap(),
            Node {
                min_width: px(0),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(left).add_children(&[speeds, place]);

    // ---- middle: the transport, the play button on the centre line
    let middle = commands
        .spawn((
            Node {
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let play = if look.playing {
        ('\u{f04c}', ReplayPhrase::Pause)
    } else {
        ('\u{f04b}', ReplayPhrase::Play)
    };
    let keys = [
        ('\u{f049}', ReplayPhrase::Start, Command::Start),
        ('\u{f04a}', ReplayPhrase::PrevTurn, Command::Turn(-1)),
        ('\u{f048}', ReplayPhrase::StepBack, Command::Step(-1)),
        (play.0, play.1, Command::Toggle),
        ('\u{f051}', ReplayPhrase::StepOn, Command::Step(1)),
        ('\u{f04e}', ReplayPhrase::NextTurn, Command::Turn(1)),
        ('\u{f050}', ReplayPhrase::End, Command::End),
    ];
    for (glyph, words, command) in keys {
        let big = command == Command::Toggle;
        let button = round(
            commands,
            fonts,
            glyph,
            big,
            words.text(lang),
            BarAsk::Host(ReplayAsk::Transport(command)),
        );
        commands.entity(middle).add_child(button);
    }

    // ---- right: whose table, and the hand
    let right = side(commands, JustifyContent::FlexEnd);
    if look.hidden && look.perspective != Perspective::Public {
        let lock = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(5.0),
                    flex_shrink: 1.0,
                    min_width: px(0),
                    ..default()
                },
                crate::hud::Hint(ReplayPhrase::NotSharedHint.text(lang).to_string()),
            ))
            .with_children(|lock| {
                lock.spawn((
                    Text::new('\u{f023}'.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(palette::LEDGE_SOFT),
                    Pickable::IGNORE,
                ));
                lock.spawn((
                    Text::new(ReplayPhrase::NotShared.text(lang)),
                    tf(fonts, LABEL_PT),
                    TextColor(palette::LEDGE_SOFT),
                    TextLayout::no_wrap(),
                    Pickable::IGNORE,
                ));
            })
            .id();
        commands.entity(right).add_child(lock);
    }
    let whose = look
        .choices
        .iter()
        .find(|(p, _, _)| *p == look.perspective)
        .map_or_else(
            || crate::replay::perspective_words(look.perspective, lang).to_string(),
            |(_, words, _)| words.clone(),
        );
    let picker = chip(
        commands,
        fonts,
        Chip {
            icon: '\u{f06e}',
            words: &whose,
            chevron: Some(if look.picker { '\u{f078}' } else { '\u{f077}' }),
            on: look.picker,
            hint: ReplayPhrase::WatchFrom.text(lang),
        },
        BarAsk::TogglePicker,
    );
    let hand_words = if look.hand_open {
        ReplayPhrase::FoldHand
    } else {
        ReplayPhrase::ShowHand
    };
    let hand = chip(
        commands,
        fonts,
        Chip {
            icon: '\u{f256}',
            words: "",
            chevron: Some(if look.hand_open {
                '\u{f078}'
            } else {
                '\u{f077}'
            }),
            on: false,
            hint: hand_words.text(lang),
        },
        BarAsk::ToggleHand,
    );
    commands.entity(right).add_children(&[picker, hand]);
    commands.entity(row).add_children(&[left, middle, right]);
    row
}

/// A press on a bar control: its ask for [`drive`], and nothing under it.
fn asks_on_click(ask: BarAsk) -> impl Fn(On<PointerClick>, ResMut<ReplayAsks>) {
    move |mut click: On<PointerClick>, mut asks: ResMut<ReplayAsks>| {
        click.propagate(false);
        asks.0.push(ask);
    }
}

/// A round transport button; the play button larger and in brass.
fn round(
    commands: &mut Commands,
    fonts: &UiFonts,
    glyph: char,
    big: bool,
    hint: &str,
    ask: BarAsk,
) -> Entity {
    let side = if big { PLAY } else { KEY };
    let (ground, ink) = if big {
        (BRASS, palette::PARCHMENT_INK)
    } else {
        (Color::NONE, palette::DOCK_INK)
    };
    let hot = if big {
        crate::ambience::lighter(BRASS, 0.18)
    } else {
        palette::PANEL_HOT
    };
    commands
        .spawn((
            UiButton,
            Node {
                width: px(side),
                height: px(side),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(side / 2.0)),
                margin: UiRect::horizontal(px(if big { 6.0 } else { 0.0 })),
                ..default()
            },
            BackgroundColor(ground),
            Feel::lifting(ground, hot, 0.08),
            crate::hud::Hint(hint.to_string()),
        ))
        .with_child((
            Text::new(glyph.to_string()),
            icon_tf(fonts, if big { 13.0 } else { 11.0 }),
            TextColor(ink),
            // The play triangle stands a hair right of its box's centre to
            // look centred.
            Node {
                margin: UiRect::left(px(if glyph == '\u{f04b}' { 2.0 } else { 0.0 })),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .observe(asks_on_click(ask))
        .id()
}

/// A speed pill: brass when chosen.
fn pill(commands: &mut Commands, fonts: &UiFonts, words: &str, on: bool, ask: BarAsk) -> Entity {
    let ground = if on { BRASS } else { Color::NONE };
    let hot = if on {
        crate::ambience::lighter(BRASS, 0.15)
    } else {
        palette::PANEL_HOT
    };
    commands
        .spawn((
            UiButton,
            Node {
                min_width: px(28.0),
                height: px(22.0),
                padding: UiRect::horizontal(px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(11.0)),
                ..default()
            },
            BackgroundColor(ground),
            Feel::lifting(ground, hot, 0.05),
        ))
        .with_child((
            Text::new(words),
            tf_bold(fonts, 11.0),
            TextColor(if on {
                palette::PARCHMENT_INK
            } else {
                palette::LEDGE_SOFT
            }),
            Pickable::IGNORE,
        ))
        .observe(asks_on_click(ask))
        .id()
}

/// What a chip on the right shows.
struct Chip<'a> {
    icon: char,
    words: &'a str,
    chevron: Option<char>,
    on: bool,
    hint: &'a str,
}

/// A chip on the right: an icon, words, a chevron; lit while its panel
/// stands open.
fn chip(commands: &mut Commands, fonts: &UiFonts, look: Chip, ask: BarAsk) -> Entity {
    let ground = if look.on {
        palette::PANEL_HOT
    } else {
        palette::DOCK_GROUND.with_alpha(0.85)
    };
    let chip = commands
        .spawn((
            UiButton,
            Node {
                height: px(26.0),
                padding: UiRect::horizontal(px(10.0)),
                column_gap: px(6.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(13.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(if look.on {
                BRASS
            } else {
                palette::DOCK_EDGE.with_alpha(0.5)
            }),
            Feel::lifting(ground, palette::PANEL_HOT, 0.05),
            crate::hud::Hint(look.hint.to_string()),
        ))
        .observe(asks_on_click(ask))
        .id();
    let mark = commands
        .spawn((
            Text::new(look.icon.to_string()),
            icon_tf(fonts, 11.0),
            TextColor(palette::DOCK_INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(chip).add_child(mark);
    if !look.words.is_empty() {
        let text = commands
            .spawn((
                Text::new(look.words),
                tf(fonts, LABEL_PT),
                TextColor(palette::DOCK_INK),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(chip).add_child(text);
    }
    if let Some(chevron) = look.chevron {
        let mark = commands
            .spawn((
                Text::new(chevron.to_string()),
                icon_tf(fonts, 8.0),
                TextColor(palette::LEDGE_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(chip).add_child(mark);
    }
    chip
}

/// The seat picker: every perspective the replay offers, a lock beside a
/// seat whose hidden cards stay hidden; over the bar's right end.
#[allow(clippy::too_many_lines)] // one panel, its rows top to bottom
fn spawn_picker(commands: &mut Commands, fonts: &UiFonts, look: &BarLook) {
    let lang = look.lang;
    let panel = commands
        .spawn((
            ReplaySheet,
            DetachedHud,
            crate::hud::hand_drawer::RidesTheHand,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(HAND_ZONE_H - LEDGE_H + BAR_H + 6.0),
                right: px(look.right + 10.0),
                min_width: px(220.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6.0)),
                row_gap: px(2.0),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(10.0)),
                ..default()
            },
            UiTransform::default(),
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            BoxShadow::from(ShadowStyle {
                color: palette::SHEET_SHADOW,
                x_offset: px(0.0),
                y_offset: px(4.0),
                spread_radius: px(0.0),
                blur_radius: px(14.0),
            }),
            GlobalZIndex(G_REPLAY_SHEET),
        ))
        .id();
    let head = commands
        .spawn((
            Text::new(ReplayPhrase::WatchFrom.text(lang)),
            tf(fonts, 11.0),
            TextColor(palette::DIALOG_SOFT),
            Node {
                margin: UiRect::new(px(8.0), px(8.0), px(2.0), px(4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(head);
    for (perspective, words, hidden) in &look.choices {
        let on = *perspective == look.perspective;
        let ground = if on { palette::DIALOG_LIT } else { Color::NONE };
        let row = commands
            .spawn((
                UiButton,
                Node {
                    height: px(30.0),
                    padding: UiRect::horizontal(px(10.0)),
                    column_gap: px(8.0),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(6.0)),
                    ..default()
                },
                BackgroundColor(ground),
                Feel::tinting_to(ground, palette::DIALOG_LIT),
            ))
            .observe(asks_on_click(BarAsk::Host(ReplayAsk::Perspective(
                *perspective,
            ))))
            .id();
        let icon = match perspective {
            Perspective::Public => '\u{f0c0}',
            Perspective::AllHands(_) => '\u{f06e}',
            Perspective::Seat(_) => '\u{f007}',
        };
        let mark = commands
            .spawn((
                Text::new(icon.to_string()),
                icon_tf(fonts, 11.0),
                TextColor(if on { BRASS } else { palette::DIALOG_SOFT }),
                Node {
                    width: px(14.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let text = commands
            .spawn((
                Text::new(words.clone()),
                tf(fonts, 13.0),
                TextColor(palette::DIALOG_INK),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_children(&[mark, text]);
        if *hidden && *perspective != Perspective::Public {
            let lock = commands
                .spawn((
                    Text::new('\u{f023}'.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(palette::DIALOG_SOFT),
                    crate::hud::Hint(ReplayPhrase::NotSharedHint.text(lang).to_string()),
                ))
                .id();
            commands.entity(row).add_child(lock);
        }
        commands.entity(panel).add_child(row);
    }
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
                bottom: px(HAND_ZONE_H + TIMELINE_H + 24.0),
                left: percent(28),
                right: percent(28),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(12.0),
                padding: UiRect::all(px(16.0)),
                border: UiRect::all(px(1.0)),
                border_radius: BorderRadius::all(px(10.0)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            BoxShadow::from(ShadowStyle {
                color: palette::SHEET_SHADOW,
                x_offset: px(0.0),
                y_offset: px(4.0),
                spread_radius: px(0.0),
                blur_radius: px(14.0),
            }),
            GlobalZIndex(G_REPLAY_SHEET),
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
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(row);
    if play_on {
        let button = sheet_button(commands, fonts, ReplayPhrase::PlayOnAnyway.text(lang), true);
        commands
            .entity(button)
            .observe(asks_on_click(BarAsk::Host(ReplayAsk::PlayOn)));
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
        let back = sheet_button(
            commands,
            fonts,
            ReplayPhrase::BackToReplays.text(lang),
            !play_on,
        );
        commands
            .entity(back)
            .observe(|mut click: On<PointerClick>, mut duel: ResMut<Duel>| {
                click.propagate(false);
                duel.leave_asked = true;
            });
        commands.entity(row).add_child(back);
    }
}

/// An answer on a sheet: the lead one in brass.
fn sheet_button(commands: &mut Commands, fonts: &UiFonts, words: &str, lead: bool) -> Entity {
    let ground = if lead { BRASS } else { palette::DIALOG_LIT };
    commands
        .spawn((
            UiButton,
            Node {
                height: px(30.0),
                padding: UiRect::horizontal(px(14.0)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(15.0)),
                ..default()
            },
            BackgroundColor(ground),
            Feel::lifting(ground, crate::ambience::lighter(ground, 0.15), 0.05),
        ))
        .with_child((
            Text::new(words),
            tf_bold(fonts, LABEL_PT),
            TextColor(if lead {
                palette::PARCHMENT_INK
            } else {
                palette::DIALOG_INK
            }),
            Pickable::IGNORE,
        ))
        .id()
}

/// Writes the playhead's place and the step's words, where they moved; and
/// starts the timeline past the players' strip, which stands on the same
/// edge at the left (#264), measured off its own layout.
#[allow(clippy::type_complexity)]
pub fn place_playhead(
    duel: Res<Duel>,
    settings: Res<crate::settings::ClientSettings>,
    mut heads: Query<&mut Node, (With<Playhead>, Without<Scrubber>)>,
    mut tracks: Query<&mut Node, (With<Scrubber>, Without<Playhead>)>,
    strips: Query<&ComputedNode, With<crate::hud::PlayersStrip>>,
    mut labels: Query<&mut Text, With<WhereLabel>>,
) {
    let Some(status) = duel.replay.as_ref() else {
        return;
    };
    let strip = strips
        .iter()
        .map(|node| node.size().x * node.inverse_scale_factor())
        .fold(0.0_f32, f32::max);
    // The bar's own padding is already between it and the window's edge.
    let margin = px(if strip > 0.0 { strip + 6.0 - 10.0 } else { 0.0 });
    for mut track in &mut tracks {
        if track.margin.left != margin {
            track.margin.left = margin;
        }
    }
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

/// The scrubber's own entity, whichever cell of it was pressed: a cell
/// lights under the pointer, so the press lands on it and bubbles up.
fn scrubber_of(
    entity: Entity,
    scrubbers: &Query<(), With<Scrubber>>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    if scrubbers.contains(entity) {
        return Some(entity);
    }
    parents
        .get(entity)
        .ok()
        .map(ChildOf::parent)
        .filter(|p| scrubbers.contains(*p))
}

#[allow(clippy::too_many_arguments)]
fn seek_on_press(
    mut press: On<PointerPress>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    scrubbers: Query<(), With<Scrubber>>,
    parents: Query<&ChildOf>,
    windows: Query<&Window>,
    duel: Res<Duel>,
    mut asks: ResMut<ReplayAsks>,
) {
    press.propagate(false);
    let Some(track) = scrubber_of(press.event_target(), &scrubbers, &parents) else {
        return;
    };
    if let Some(step) = step_under(track, press.pointer.position.x, &nodes, &windows, &duel) {
        asks.0
            .push(BarAsk::Host(ReplayAsk::Transport(Command::Seek(step))));
    }
}

fn seek_on_drag(
    drag: On<PointerDrag>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    scrubbers: Query<(), With<Scrubber>>,
    parents: Query<&ChildOf>,
    windows: Query<&Window>,
    duel: Res<Duel>,
    mut pending: ResMut<ScrubDrag>,
) {
    let Some(track) = scrubber_of(drag.event_target(), &scrubbers, &parents) else {
        return;
    };
    if let Some(step) = step_under(track, drag.pointer.position.x, &nodes, &windows, &duel) {
        pending.pending = Some(step);
    }
}

fn seek_on_release(
    _release: On<PointerDragEnd>,
    mut pending: ResMut<ScrubDrag>,
    mut asks: ResMut<ReplayAsks>,
) {
    if let Some(step) = pending.pending.take() {
        asks.0
            .push(BarAsk::Host(ReplayAsk::Transport(Command::Seek(step))));
    }
    pending.since = 0.0;
}

/// Carries the frame's asks out: the replay's to the host (a drag's seek at
/// most every 100 ms), the bar's own to [`BarUi`]; and the clock, the
/// virtual one, so dev-control's `/pause` and `/step` hold a replay too.
pub fn drive(
    time: Res<Time>,
    host: Option<ResMut<crate::InstalledHost>>,
    mut asks: ResMut<ReplayAsks>,
    mut drag: ResMut<ScrubDrag>,
    mut ui: ResMut<BarUi>,
) {
    let status = host.as_ref().and_then(|h| h.0.replay().cloned());
    let (Some(mut host), Some(status)) = (host, status) else {
        if *ui != BarUi::default() {
            *ui = BarUi::default();
        }
        if !asks.0.is_empty() {
            asks.0.clear();
        }
        return;
    };
    if !asks.0.is_empty() {
        for ask in std::mem::take(&mut asks.0) {
            match ask {
                BarAsk::Host(ask) => {
                    if matches!(ask, ReplayAsk::Perspective(_) | ReplayAsk::NextPerspective)
                        && ui.picker
                    {
                        ui.picker = false;
                    }
                    host.0.control(ask);
                }
                BarAsk::ToggleHand => {
                    let open = ui.hand_open(&status);
                    ui.hand = Some((status.perspective, !open));
                }
                BarAsk::TogglePicker => ui.picker = !ui.picker,
            }
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
/// Over the end sheet `Space` and `Esc` stay the sheet's; `I` folds the
/// hand; `Esc` shuts an open seat picker first.
#[allow(clippy::too_many_arguments)]
pub fn replay_keys(
    keys: Res<ButtonInput<KeyCode>>,
    prefs: Res<crate::prefs::Prefs>,
    duel: Res<Duel>,
    phase: Res<State<crate::DuelPhase>>,
    desk: Option<Res<crate::report::ReportDesk>>,
    ui: Res<BarUi>,
    mut asks: ResMut<ReplayAsks>,
    lobby: Option<Res<crate::LobbyState>>,
) {
    use baylee_client_core::prefs::Action;
    let Some(status) = duel.replay.as_ref() else {
        return;
    };
    // The settings over the replay have every key (`lobby::table_settings`).
    if desk.is_some_and(|desk| desk.holds_keyboard())
        || duel.browser.is_typing()
        || lobby.is_some_and(|l| l.settings_over_table())
    {
        return;
    }
    let fired = crate::keys::Fired::of(&keys, prefs.keymap());
    if fired.quiet() {
        return;
    }
    if ui.picker && fired.has(Action::Cancel) {
        asks.0.push(BarAsk::TogglePicker);
        return;
    }
    let finished = *phase.get() == crate::DuelPhase::Finished;
    let loading = status.loading.is_some();
    let found = crate::keys::replay_asks(&fired, finished, loading);
    if !found.is_empty() {
        asks.0.extend(found.into_iter().map(BarAsk::Host));
    }
    if fired.has(Action::HandDrawer) {
        asks.0.push(BarAsk::ToggleHand);
    }
}

#[cfg(test)]
mod pointer_tests;
