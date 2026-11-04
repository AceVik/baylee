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
}

impl Readout {
    fn said(self, lang: Lang, progress: &Progress) -> String {
        match self {
            Self::Status => words::status(lang, progress.status),
            Self::Percent if progress.pictures_total > 0 => words::percent_text(lang, progress),
            Self::Percent => String::new(),
            Self::Cards => words::cards(lang, progress),
            Self::Figures => words::figures(lang, progress),
            Self::Cache => words::cache(lang, progress),
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
/// header's other tools, opening the block.
pub(crate) fn mark(commands: &mut Commands, kit: Kit) -> Entity {
    let m: ShellMetrics = kit.m;
    let face = commands
        .spawn((
            Node {
                min_height: m.px(32.0),
                padding: UiRect::axes(m.px(6.0), m.px(4.0)),
                align_items: AlignItems::Center,
                column_gap: m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(crate::hud::glyph::DOWNLOAD.to_string()),
            crate::hud::icon_tf(kit.fonts, m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let share = commands
        .spawn((
            FillMarkShare,
            Text::new(""),
            crate::hud::tf(kit.fonts, m.small),
            TextColor(tokens::MUTED),
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
            Press::Settings(SettingsPress::Fill(FillPress::Show)),
            hint::HoverHint(String::new()),
        ),
    );
    commands.entity(hit).entry::<Node>().and_modify(|mut node| {
        node.display = Display::None;
    });
    hit
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
