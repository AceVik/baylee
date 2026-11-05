//! What a player sees of the card cache's fill: the block in Settings ›
//! Network & Gateway, and the small mark in the header while it runs. What it
//! says is `client_core::cachefill::words`.
//!
//! The block is drawn with the screen and its readings written in place (the
//! update card's way): a bar's width and four texts, compared first, twice a
//! second. Only a change of which buttons stand rebuilds the screen.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::settingsui::rows::Out;
use crate::shellkit::controls::{self, Kit, Weight};
use crate::shellkit::{ShellMetrics, px_fixed, tokens};
use baylee_client_core::cachefill::{
    Budget, FillChoices, Pace, Progress, Reach, Status, Steering, Why, words,
};
use baylee_client_core::settings_map::Row;

/// How often the fill's numbers are read.
const READ_EVERY: f64 = 0.5;

/// A control of the block, or the header's mark.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FillPress {
    /// The switch.
    Toggle,
    /// A pace.
    Pace(Pace),
    /// What to keep.
    Reach(Reach),
    /// How large.
    Budget(Budget),
    /// Pause, or Resume.
    Pause,
    /// Try again after a stop.
    TryAgain,
    /// Empty the cache: the first press arms, the second does it.
    Empty,
    /// The header's mark: open the block.
    Show,
}

impl FillPress {
    /// Whether this one is a choice kept in the settings file.
    pub(super) const fn on_device(self) -> bool {
        matches!(
            self,
            Self::Toggle | Self::Pace(_) | Self::Reach(_) | Self::Budget(_)
        )
    }

    /// Writes a choice, and answers whether it changed anything.
    pub(super) fn apply(self, choices: &mut FillChoices) -> bool {
        let before = *choices;
        match self {
            Self::Toggle => choices.on = !choices.on,
            Self::Pace(pace) => choices.pace = pace,
            Self::Reach(reach) => choices.reach = reach,
            Self::Budget(budget) => choices.budget = budget,
            Self::Pause | Self::TryAgain | Self::Empty | Self::Show => {}
        }
        *choices != before
    }

    /// Does what is no choice.
    pub(super) fn act(self, state: &mut LobbyState) {
        let view = &mut state.settings_view;
        match self {
            Self::Pause => {
                view.fill.held = !view.fill.held;
                view.fill.armed = false;
            }
            Self::TryAgain => {
                view.fill.armed = false;
                crate::cachefill::retry();
            }
            Self::Empty => {
                if view.fill.armed {
                    crate::cachefill::empty();
                }
                view.fill.armed = !view.fill.armed;
            }
            Self::Show => {
                state.settings = SettingsPane::Open;
                crate::settingsui::show(
                    state,
                    baylee_client_core::settings_map::of(Row::CardFill).section,
                );
            }
            Self::Toggle | Self::Pace(_) | Self::Reach(_) | Self::Budget(_) => {}
        }
    }
}

/// Which of the block's buttons stand: what changes the screen's tree, where
/// every other reading only writes into it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FillButtons {
    /// Nothing to pause.
    #[default]
    Resting,
    /// Pause.
    Going,
    /// Resume.
    Paused,
    /// Try again.
    Stopped,
}

impl FillButtons {
    const fn of(status: Status) -> Self {
        match status {
            Status::Off | Status::Finished => Self::Resting,
            Status::Paused(Why::Player) => Self::Paused,
            Status::Stopped(_) => Self::Stopped,
            Status::Paused(Why::Match)
            | Status::Checking
            | Status::Running
            | Status::Cooling(_) => Self::Going,
        }
    }
}

/// What the settings screen remembers of the block, for this run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FillView {
    /// The player pressed Pause.
    pub(crate) held: bool,
    /// Which buttons stand; a change redraws them.
    pub(crate) buttons: FillButtons,
    /// *Empty the cache* has been pressed once.
    pub(crate) armed: bool,
}

/// The fill's latest reading.
#[derive(Resource, Default)]
pub(crate) struct FillBoard {
    pub(super) progress: Progress,
    /// When to read it next, on the real clock.
    pub(super) read_at: f64,
}

/// What the client wants of the fill. Any phase but `Closed` is a table on
/// screen, a replay's among them, opening or showing its result: that frame
/// time and that bandwidth are the game's. Who is signed in has no say: the
/// fill asks Scryfall, and what it files serves every art base.
pub(super) fn steering(
    state: &LobbyState,
    choices: Option<FillChoices>,
    phase: DuelPhase,
) -> Steering {
    Steering {
        choices: choices.unwrap_or(FillChoices {
            on: false,
            ..FillChoices::default()
        }),
        playing: phase != DuelPhase::Closed,
        held: state.settings_view.fill.held,
    }
}

/// Tells the fill what the client wants. In every phase: the table is what
/// it has to rest for.
pub(super) fn steer(
    state: Res<LobbyState>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    phase: Res<State<DuelPhase>>,
) {
    let settings_moved = settings.as_ref().is_some_and(DetectChanges::is_changed);
    if !state.is_changed() && !phase.is_changed() && !settings_moved {
        return;
    }
    crate::cachefill::steer(steering(
        &state,
        settings.map(|s| s.card_fill),
        *phase.get(),
    ));
}

/// Reads the fill's numbers, and moves the buttons when they change. Not read
/// during a game, which shows none of this.
pub(super) fn read(
    time: Res<Time<bevy::time::Real>>,
    mut board: ResMut<FillBoard>,
    mut state: ResMut<LobbyState>,
) {
    let now = time.elapsed_secs_f64();
    if now < board.read_at {
        return;
    }
    board.read_at = now + READ_EVERY;
    let progress = crate::cachefill::read();
    if board.progress != progress {
        board.progress = progress;
    }
    let buttons = FillButtons::of(progress.status);
    if state.settings_view.fill.buttons != buttons {
        state.settings_view.fill.buttons = buttons;
    }
}

/// A text of the block that follows the numbers.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Readout {
    /// What it is doing.
    Status,
    /// The share.
    Percent,
    /// How far the round is.
    Cards,
    /// The numbers of the run.
    Figures,
    /// How full the cache is.
    Cache,
    /// Downloaded / total card count (`4.210 / 10.754`).
    CountPair,
    /// Current download speed (`1,2 MB/s`).
    Speed,
    /// Downloaded size so far (`1,1 GB`).
    DownloadedSize,
    /// Estimated total size (`~ 2,4 GB`).
    EstimatedTotalSize,
    /// Estimated remaining time (`~ 12 min`).
    RemainingTime,
    /// Short round description (`Vorschaubilder · Runde 1/2`).
    RoundShort,
}

impl Readout {
    fn said(self, lang: Lang, progress: &Progress) -> String {
        use baylee_client_core::sizewords::{rate, size};
        match self {
            Self::Status => words::status(lang, progress.status),
            Self::Percent if progress.pictures_total > 0 => words::percent_text(lang, progress),
            Self::Percent => String::new(),
            Self::Cards => words::cards(lang, progress),
            Self::Figures => words::figures(lang, progress),
            Self::Cache => words::cache(lang, progress),
            Self::CountPair => {
                if progress.cards_total > 0 {
                    format!(
                        "{} / {}",
                        words::grouped(lang, progress.cards_done),
                        words::grouped(lang, progress.cards_total)
                    )
                } else if progress.pictures_total > 0 {
                    format!(
                        "{} / {}",
                        words::grouped(lang, progress.pictures_done),
                        words::grouped(lang, progress.pictures_total)
                    )
                } else {
                    "—".to_owned()
                }
            }
            Self::Speed => progress
                .speed
                .map_or_else(|| "—".to_owned(), |s| rate(lang, s)),
            Self::DownloadedSize => {
                let bytes = progress.cached.unwrap_or(progress.bytes);
                if bytes > 0 {
                    size(lang, bytes)
                } else {
                    size(lang, 0)
                }
            }
            Self::EstimatedTotalSize => {
                let done_bytes = progress.cached.unwrap_or(progress.bytes);
                let share = f64::from(words::share(progress));
                if share > 0.01 && done_bytes > 0 {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        clippy::cast_precision_loss
                    )]
                    let est = ((done_bytes as f64) / share).round() as u64;
                    format!("~ {}", size(lang, est.max(done_bytes)))
                } else if progress.budget > 0 {
                    format!("≤ {}", size(lang, progress.budget))
                } else {
                    "—".to_owned()
                }
            }
            Self::RemainingTime => progress.left.map_or_else(
                || "—".to_owned(),
                |left| format!("~ {}", words::span(lang, left)),
            ),
            Self::RoundShort => progress.round.map_or_else(String::new, |round| {
                let which = match round {
                    baylee_client_core::cachefill::Round::Thumbnails => {
                        Phrase::CardFillRoundThumbnails
                    }
                    baylee_client_core::cachefill::Round::Large => Phrase::CardFillRoundLarge,
                };
                which.fill(
                    lang,
                    &[
                        &(round.index() + 1).to_string(),
                        &progress.rounds.to_string(),
                    ],
                )
            }),
        }
    }
}

/// The bar's fill.
#[derive(Component)]
pub(crate) struct FillBar;

/// A bar as wide as the share that is on disk.
fn bar_width(progress: &Progress) -> Val {
    Val::Percent((words::share(progress) * 1000.0).round() / 10.0)
}

/// Writes the numbers into the drawn block, each only when it differs.
pub(super) fn show(
    board: Res<FillBoard>,
    state: Res<LobbyState>,
    fresh: Query<(), Added<Readout>>,
    mut texts: Query<(&Readout, &mut Text)>,
    mut bars: Query<&mut Node, With<FillBar>>,
) {
    if texts.is_empty() || (!board.is_changed() && fresh.is_empty()) {
        return;
    }
    let lang = state.lobby.lang();
    for (readout, mut text) in &mut texts {
        let said = readout.said(lang, &board.progress);
        if text.0 != said {
            text.0 = said;
        }
    }
    let width = bar_width(&board.progress);
    for mut node in &mut bars {
        if node.width != width {
            node.width = width;
        }
    }
}

/// A line of the block.
fn line(out: &mut Out, readout: Readout, ink: Color, size: f32) -> Entity {
    let font = crate::hud::tf(out.kit.fonts, size);
    out.commands
        .spawn((
            readout,
            Text::new(""),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px_fixed(0.0),
                ..default()
            },
            font,
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

/// The status, the bar, the numbers and the buttons.
fn block(out: &mut Out, state: &LobbyState) -> Entity {
    let m = out.kit.m;
    let head = out
        .commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                column_gap: m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let status = line(out, Readout::Status, tokens::INK, m.text);
    let share = line(out, Readout::Percent, tokens::INK, m.small);
    out.commands.entity(head).add_children(&[status, share]);
    let track = out
        .commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: m.px(4.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::TRACK_OFF),
            Pickable::IGNORE,
        ))
        .id();
    let filled = out
        .commands
        .spawn((
            FillBar,
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    out.commands.entity(track).add_child(filled);
    let cards = line(out, Readout::Cards, tokens::MUTED, m.small);
    let figures = line(out, Readout::Figures, tokens::MUTED, m.small);
    let cache = line(out, Readout::Cache, tokens::MUTED, m.small);
    let holder = out
        .commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    out.commands
        .entity(holder)
        .add_children(&[head, track, cards, figures, cache]);
    buttons(out, holder, state.settings_view.fill);
    holder
}

/// The block's buttons: Pause, Resume or Try again, whichever stands, and
/// Empty the cache, which asks first.
fn buttons(out: &mut Out, holder: Entity, view: FillView) {
    let lang = out.lang;
    let press = |press| Press::Settings(SettingsPress::Fill(press));
    let mut buttons = Vec::new();
    let going = match view.buttons {
        FillButtons::Going => Some((Phrase::CardFillPause, FillPress::Pause)),
        FillButtons::Paused => Some((Phrase::CardFillResume, FillPress::Pause)),
        FillButtons::Stopped => Some((Phrase::CardFillTryAgain, FillPress::TryAgain)),
        FillButtons::Resting => None,
    };
    if let Some((words, action)) = going {
        buttons.push(out.button(
            "card-fill-go",
            words.text(lang),
            Weight::Secondary,
            press(action),
        ));
    }
    let (empty, weight) = if view.armed {
        (Phrase::CardFillEmptySure, Weight::Danger)
    } else {
        (Phrase::CardFillEmpty, Weight::Secondary)
    };
    buttons.push(out.button(
        "card-fill-empty",
        empty.text(lang),
        weight,
        press(FillPress::Empty),
    ));
    let row = out.wrap(&buttons);
    out.commands.entity(holder).add_child(row);
    if view.armed {
        let warning = out.words(Phrase::CardFillEmptyAsk.text(lang), true);
        out.commands.entity(holder).add_child(warning);
    }
}

/// A segmented row of choices.
fn choice<T: Copy + PartialEq>(
    out: &mut Out,
    row: Row,
    id: &'static str,
    all: &[T],
    now: T,
    names: &[String],
    press: impl Fn(T) -> FillPress,
) {
    let labels: Vec<&str> = names.iter().map(String::as_str).collect();
    let at = all.iter().position(|value| *value == now);
    let control = out.seg(id, &labels, at, |i| {
        Press::Settings(SettingsPress::Fill(press(all[i])))
    });
    out.row(row, control);
}

/// The Network section's rows of the fill.
pub(crate) fn rows(out: &mut Out, view: &crate::settingsui::View) {
    let lang = out.lang;
    let choices = view.settings.map(|s| s.card_fill).unwrap_or_default();
    let toggle = out.toggle(
        "card-fill",
        choices.on,
        Press::Settings(SettingsPress::Fill(FillPress::Toggle)),
    );
    out.row(Row::CardFill, toggle);
    let holder = block(out, view.state);
    out.commands.entity(out.column).add_child(holder);
    let names: Vec<String> = Pace::ALL
        .iter()
        .map(|pace| pace.name().text(lang).to_string())
        .collect();
    choice(
        out,
        Row::CardFillPace,
        "card-fill-pace",
        &Pace::ALL,
        choices.pace,
        &names,
        FillPress::Pace,
    );
    let names: Vec<String> = Reach::ALL
        .iter()
        .map(|reach| reach.name().text(lang).to_string())
        .collect();
    choice(
        out,
        Row::CardFillReach,
        "card-fill-reach",
        &Reach::ALL,
        choices.reach,
        &names,
        FillPress::Reach,
    );
    let names: Vec<String> = Budget::ALL
        .iter()
        .map(|budget| {
            Phrase::CardFillBudget.fill(lang, &[&(budget.bytes() / 1_000_000_000).to_string()])
        })
        .collect();
    choice(
        out,
        Row::CardFillBudget,
        "card-fill-budget",
        &Budget::ALL,
        choices.budget,
        &names,
        FillPress::Budget,
    );
}

/// The header's mark: a glyph and the share, while pictures are on their way.
#[derive(Component)]
pub(crate) struct FillMark;

/// The share beside the mark's glyph.
#[derive(Component)]
pub(crate) struct FillMarkShare;

/// The mark, hidden until [`show_mark`] says otherwise. A hit area like the
/// header's other tools, opening the card-fill details popover.
pub(crate) fn mark(commands: &mut Commands, kit: Kit) -> Entity {
    let m: ShellMetrics = kit.m;
    let face = commands
        .spawn((
            crate::shellkit::Role::Button,
            Node {
                min_height: m.px(32.0),
                padding: UiRect::axes(m.px(10.0), m.px(4.0)),
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.75)),
            BorderColor::all(tokens::BORDER),
            UiTransform::default(),
            crate::ambience::Feel::new(tokens::CONTROL.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(crate::hud::glyph::DOWNLOAD.to_string()),
            crate::hud::icon_tf(kit.fonts, m.small * 0.92),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let share = commands
        .spawn((
            FillMarkShare,
            Text::new(""),
            crate::hud::tf_bold(kit.fonts, m.small * 0.92),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[glyph, share]);
    let hit = controls::hit(
        commands,
        kit,
        face,
        (
            FillMark,
            crate::shellkit::header::HeaderTrigger::CardFill,
            Press::Header(header::HeaderPress::CardFill),
            hint::HoverHint(String::new()),
        ),
    );
    commands.entity(hit).entry::<Node>().and_modify(|mut node| {
        node.display = Display::None;
    });
    hit
}

fn popover_readout_text(
    commands: &mut Commands,
    kit: Kit,
    readout: Readout,
    ink: Color,
    size: f32,
    bold: bool,
) -> Entity {
    let font = if bold {
        crate::hud::tf_bold(kit.fonts, size)
    } else {
        crate::hud::tf(kit.fonts, size)
    };
    commands
        .spawn((
            readout,
            Text::new(""),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px_fixed(0.0),
                ..default()
            },
            font,
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

fn metric_tile(
    commands: &mut Commands,
    kit: Kit,
    icon_glyph: char,
    title: &str,
    readout: Readout,
) -> Entity {
    let m = kit.m;
    let tile = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Percent(46.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(3.0),
                padding: UiRect::axes(m.px(10.0), m.px(8.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.62)),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let label_row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: m.px(5.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(icon_glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, m.small * 0.82),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let cap = commands
        .spawn((
            Text::new(title.to_owned()),
            crate::hud::tf(kit.fonts, m.small * 0.84),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(label_row).add_children(&[icon, cap]);
    let val = popover_readout_text(commands, kit, readout, tokens::INK, m.small * 0.98, true);
    commands.entity(tile).add_children(&[label_row, val]);
    tile
}

/// Rich dropdown popover opened when clicking the header's cache-warmup indicator.
#[allow(clippy::too_many_lines)]
pub(crate) fn popover(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let m = kit.m;
    let lang = state.lobby.lang();
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    // Header row: status + percentage badge
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let left_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: m.px(2.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let status = popover_readout_text(commands, kit, Readout::Status, tokens::INK, m.text, true);
    let round_line = popover_readout_text(
        commands,
        kit,
        Readout::RoundShort,
        tokens::MUTED,
        m.small * 0.86,
        false,
    );
    commands
        .entity(left_col)
        .add_children(&[status, round_line]);

    let badge = commands
        .spawn((
            Node {
                padding: UiRect::axes(m.px(8.0), m.px(3.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT.with_alpha(0.14)),
            BorderColor::all(tokens::ACCENT.with_alpha(0.42)),
            Pickable::IGNORE,
        ))
        .id();
    let pct = popover_readout_text(
        commands,
        kit,
        Readout::Percent,
        tokens::ACCENT,
        m.small * 0.94,
        true,
    );
    commands.entity(badge).add_child(pct);
    commands.entity(head).add_children(&[left_col, badge]);

    // Progress bar
    let track = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: m.px(7.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::TRACK_OFF),
            Pickable::IGNORE,
        ))
        .id();
    let filled = commands
        .spawn((
            FillBar,
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(filled);

    // 2x3 grid of detailed metrics
    let grid = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: m.px(6.0),
                row_gap: m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let t_cards = metric_tile(
        commands,
        kit,
        crate::hud::glyph::LIBRARY,
        Phrase::CardFillTileCards.text(lang),
        Readout::CountPair,
    );
    let t_speed = metric_tile(
        commands,
        kit,
        crate::hud::glyph::ENERGY,
        Phrase::CardFillTileSpeed.text(lang),
        Readout::Speed,
    );
    let t_done = metric_tile(
        commands,
        kit,
        crate::hud::glyph::DOWNLOAD,
        Phrase::CardFillTileDownloaded.text(lang),
        Readout::DownloadedSize,
    );
    let t_total = metric_tile(
        commands,
        kit,
        crate::hud::glyph::ZONES,
        Phrase::CardFillTileTotal.text(lang),
        Readout::EstimatedTotalSize,
    );
    let t_left = metric_tile(
        commands,
        kit,
        crate::hud::glyph::CLOCK,
        Phrase::CardFillTileRemaining.text(lang),
        Readout::RemainingTime,
    );
    commands
        .entity(grid)
        .add_children(&[t_cards, t_speed, t_done, t_total, t_left]);

    // Footer actions: Pause/Resume + Open Network Settings
    let actions_row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: m.px(8.0),
                padding: UiRect::top(m.px(2.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let view = state.settings_view.fill;
    let going = match view.buttons {
        FillButtons::Going => Some((Phrase::CardFillPause, FillPress::Pause)),
        FillButtons::Paused => Some((Phrase::CardFillResume, FillPress::Pause)),
        FillButtons::Stopped => Some((Phrase::CardFillTryAgain, FillPress::TryAgain)),
        FillButtons::Resting => None,
    };
    if let Some((words, action)) = going {
        let pause_btn = controls::button(
            commands,
            kit,
            words.text(lang),
            Weight::Secondary,
            controls::Live::Yes,
            None,
            Press::Settings(SettingsPress::Fill(action)),
        );
        commands
            .entity(pause_btn)
            .entry::<Node>()
            .and_modify(move |mut n| {
                n.flex_grow = 1.0;
                n.flex_basis = px_fixed(0.0);
                n.min_height = m.px(32.0);
            });
        commands.entity(actions_row).add_child(pause_btn);
    }

    let settings_btn = controls::button(
        commands,
        kit,
        Phrase::SettingsTitle.text(lang),
        Weight::Secondary,
        controls::Live::Yes,
        None,
        Press::Settings(SettingsPress::Fill(FillPress::Show)),
    );
    commands
        .entity(settings_btn)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.flex_grow = 1.0;
            n.flex_basis = px_fixed(0.0);
            n.min_height = m.px(32.0);
        });
    commands.entity(actions_row).add_child(settings_btn);

    commands
        .entity(root)
        .add_children(&[head, track, grid, actions_row]);
    root
}

/// Stands the mark while pictures are on their way, and keeps its words true.
pub(super) fn show_mark(
    board: Res<FillBoard>,
    state: Res<LobbyState>,
    fresh: Query<(), Added<FillMark>>,
    mut marks: Query<(&mut Node, &mut hint::HoverHint), With<FillMark>>,
    mut shares: Query<&mut Text, With<FillMarkShare>>,
) {
    if marks.is_empty() || (!board.is_changed() && fresh.is_empty()) {
        return;
    }
    let lang = state.lobby.lang();
    let progress = &board.progress;
    let display = if words::shows_indicator(progress.status) {
        Display::Flex
    } else {
        Display::None
    };
    let said = words::hint(lang, progress);
    for (mut node, mut hint) in &mut marks {
        if node.display != display {
            node.display = display;
        }
        if hint.0 != said {
            hint.0.clone_from(&said);
        }
    }
    let share = words::percent_text(lang, progress);
    for mut text in &mut shares {
        if text.0 != share {
            text.0.clone_from(&share);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A choice writes the file only when it changes, and what is no choice
    /// writes nothing.
    #[test]
    fn a_press_writes_only_what_it_changes() {
        let mut choices = FillChoices::default();
        assert!(FillPress::Pace(Pace::Brisk).apply(&mut choices));
        assert_eq!(choices.pace, Pace::Brisk);
        assert!(!FillPress::Pace(Pace::Brisk).apply(&mut choices));
        assert!(FillPress::Toggle.apply(&mut choices));
        assert!(!choices.on);
        assert!(FillPress::Reach(Reach::Thumbnails).apply(&mut choices));
        assert!(FillPress::Budget(Budget::Five).apply(&mut choices));
        for press in [
            FillPress::Pause,
            FillPress::TryAgain,
            FillPress::Empty,
            FillPress::Show,
        ] {
            assert!(!press.on_device());
            assert!(!press.apply(&mut choices));
        }
    }

    /// Which buttons stand follows the state, and only a change of those
    /// rebuilds the screen.
    #[test]
    fn the_buttons_follow_the_state() {
        use baylee_client_core::cachefill::Stop;
        use std::time::Duration;
        assert_eq!(FillButtons::of(Status::Off), FillButtons::Resting);
        assert_eq!(FillButtons::of(Status::Finished), FillButtons::Resting);
        assert_eq!(
            FillButtons::of(Status::Paused(Why::Player)),
            FillButtons::Paused
        );
        assert_eq!(
            FillButtons::of(Status::Stopped(Stop::Budget)),
            FillButtons::Stopped
        );
        assert_eq!(FillButtons::of(Status::Running), FillButtons::Going);
        assert_eq!(
            FillButtons::of(Status::Cooling(Duration::from_secs(1))),
            FillButtons::of(Status::Cooling(Duration::from_secs(2))),
            "a count-down rebuilds nothing"
        );
    }

    /// Every control's words fit their control, in both languages.
    #[test]
    fn the_buttons_fit_their_budgets() {
        use crate::shellkit::tokens::{BUTTON_BUDGET, CHIP_BUDGET};
        for lang in Lang::ALL {
            let german = lang == Lang::De;
            for phrase in [
                Phrase::CardFillPause,
                Phrase::CardFillResume,
                Phrase::CardFillTryAgain,
                Phrase::CardFillEmpty,
                Phrase::CardFillEmptySure,
            ] {
                let said = phrase.text(lang);
                assert!(
                    said.chars().count() <= BUTTON_BUDGET.of(german),
                    "{lang:?}: {said}"
                );
            }
            for phrase in Pace::ALL
                .map(Pace::name)
                .into_iter()
                .chain(Reach::ALL.map(Reach::name))
            {
                let said = phrase.text(lang);
                assert!(
                    said.chars().count() <= CHIP_BUDGET.of(german),
                    "{lang:?}: {said}"
                );
            }
        }
    }
}
