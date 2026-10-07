//! The arrangement switcher (DESIGN-v8 §2): which arrangement is in effect
//! at this table, the pill in the top-left corner that says so, the menu it
//! opens, and the two keys (`P`, `Shift+P`).
//!
//! What an arrangement *is* lives in `baylee_client_core::layout::arrangement`:
//! an arm of `TableLayout::arranged` and a camera pose. This module only
//! decides which one stands, from this game's switch, the device's memory for
//! the seat count and its default (§2.6), and draws the control that changes
//! it. A switch writes [`Duel::arrangement`] and the layout is seated again
//! through [`crate::rebuild_board`], the one door every card's `Motion`
//! target comes out of — nothing here positions a table object.

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::tableview::{Arrangement, TableFrame, TableView};
use bevy::prelude::*;

use crate::ambience::Feel;
use crate::hud::{CORNER_BUTTON, EDGE, UiFonts, btn_radius, palette, tf, tf_bold};
use crate::{Duel, DuelPhase};

/// The window and the table the arrangements are offered for, measured once
/// a frame by [`choose`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ArrangementFrame {
    /// The window's class, once a window has been seen.
    pub frame: Option<TableFrame>,
    /// Seats at the table, mine included; zero before the first view.
    pub seats: usize,
    /// Seconds left of the pill's flash after a `Shift+P` (§2.3: 1.2 s).
    pub flash: f32,
}

impl ArrangementFrame {
    /// The class, a wide window until one has been seen.
    #[must_use]
    pub fn class(&self) -> TableFrame {
        self.frame.unwrap_or(TableFrame::Wide)
    }

    /// Whether the switcher is offered at all: from three seats (a duel has
    /// one arrangement, §0).
    #[must_use]
    pub const fn switchable(&self) -> bool {
        self.seats >= 3
    }
}

/// How long the pill flashes the new name after `Shift+P` (§2.3).
pub const FLASH_SECS: f32 = 1.2;

/// Seats at the table, mine included, as the layout seats them.
#[must_use]
pub fn seat_count(duel: &Duel) -> usize {
    duel.view
        .as_ref()
        .map_or(0, |view| 1 + view.opponents_in_turn_order().len())
}

/// The arrangement chosen at a table of `seats`, before the offer is asked:
/// this game's switch, else the device's memory for the count, else its
/// default (§2.6's reading order).
#[must_use]
pub fn chosen(duel: &Duel, table: &TableView, seats: usize) -> Arrangement {
    duel.arrangement_game.unwrap_or_else(|| table.chosen(seats))
}

/// The arrangement in effect: [`chosen`], refused to the ring where it is not
/// offered, and the Turntable with rows resolved once per table and held
/// (the latch), so a resize never flips it in the middle of a game.
#[must_use]
pub fn in_effect(
    duel: &Duel,
    table: &TableView,
    seats: usize,
    frame: TableFrame,
) -> (Arrangement, Option<(Arrangement, usize, Arrangement)>) {
    let chosen = chosen(duel, table, seats);
    if chosen.offered(seats, frame).is_err() {
        return (Arrangement::Ring, None);
    }
    if let Some(latch) = duel.arrangement_latch
        && latch.0 == chosen
        && latch.1 == seats
    {
        return (latch.2, Some(latch));
    }
    let resolved = chosen.resolve(seats, frame);
    (resolved, Some((chosen, seats, resolved)))
}

/// Decides the arrangement in effect, once a frame, and seats the table
/// again when it changed. Before `track_canvas`, so a switch and a resize on
/// the same frame are one rebuild apart, not two frames.
pub fn choose(
    windows: Query<&Window>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    time: Res<Time>,
    mut duel: ResMut<Duel>,
    mut measured: ResMut<ArrangementFrame>,
) {
    let window = windows
        .single()
        .ok()
        .map(|w| Vec2::new(w.width(), w.height()));
    let frame = window.map(|w| TableFrame::of(w.x, w.y));
    let seats = seat_count(&duel);
    if measured.frame != frame || measured.seats != seats {
        measured.frame = frame;
        measured.seats = seats;
    }
    if measured.flash > 0.0 {
        measured.flash = (measured.flash - time.delta_secs()).max(0.0);
    }
    let table = settings.as_deref().map(|s| s.table).unwrap_or_default();
    let (effective, latch) = in_effect(&duel, &table, seats, measured.class());
    if duel.arrangement_latch != latch {
        duel.arrangement_latch = latch;
    }
    if duel.arrangement != effective {
        duel.arrangement = effective;
        crate::rebuild_board(&mut duel);
    }
    if std::mem::take(&mut duel.arrangement_menu_asked)
        && duel.arrangement_menu.is_none()
        && let Some(settings) = settings.as_deref()
    {
        toggle_menu(&mut duel, settings, seats);
    }
    // A menu open at a table that has become a duel has nothing to offer.
    if duel.arrangement_menu.is_some() && !measured.switchable() {
        duel.arrangement_menu = None;
    }
}

/// Seats a layout arrangement again whenever the seat of interest has moved
/// since the layout was solved, whoever moved it: a strip chip, `F`, `H`,
/// `Esc`, my turn beginning, a combat question (DESIGN-v8 §0's "one state,
/// two meanings"). A camera arrangement never comes here — its visit moves
/// the camera and no card.
pub fn lay_the_interest(mut duel: ResMut<Duel>) {
    if duel.arrangement.moves_cards() && duel.visiting != duel.interest_laid {
        crate::rebuild_board(&mut duel);
    }
}

/// Switches to `arrangement`: into the device's memory for this seat count
/// when *remember* is ticked, else for this game only (§2.6). Not offered
/// here: nothing happens.
pub fn pick(
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    measured: &ArrangementFrame,
    arrangement: Arrangement,
) -> bool {
    let seats = measured.seats;
    if arrangement.offered(seats, measured.class()).is_err() {
        return false;
    }
    if duel.arrangement_remember {
        settings
            .table
            .arrangement_by_seats
            .set(seats, Some(arrangement));
        settings.save();
        duel.arrangement_game = None;
    } else {
        duel.arrangement_game = Some(arrangement);
    }
    true
}

/// `Shift+P`: the next offered arrangement after the one in effect, written
/// where a pick would write it, and the pill flashes the new name (§2.3).
/// At a table offering one, the flash alone.
pub fn cycle(
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    measured: &mut ArrangementFrame,
) {
    if !measured.switchable() {
        return;
    }
    let seats = measured.seats;
    let current = chosen(duel, &settings.table, seats);
    let next = current.next_offered(seats, measured.class());
    let remember = duel.arrangement_remember;
    duel.arrangement_remember = settings.table.arrangement_by_seats.get(seats).is_some();
    pick(duel, settings, measured, next);
    duel.arrangement_remember = remember;
    measured.flash = FLASH_SECS;
}

/// Opens the menu on the row in effect, or shuts it.
pub fn toggle_menu(duel: &mut Duel, settings: &crate::settings::ClientSettings, seats: usize) {
    if duel.arrangement_menu.is_some() {
        duel.arrangement_menu = None;
        return;
    }
    if seats < 3 {
        return;
    }
    let current = chosen(duel, &settings.table, seats);
    duel.arrangement_menu = Some(current.index());
    duel.arrangement_remember = settings.table.arrangement_by_seats.get(seats).is_some();
}

/// The menu's rows below the eight arrangements: *remember for this seat
/// count*.
pub const REMEMBER_ROW: usize = Arrangement::ALL.len();

/// The keys while the menu stands, and the two that open and cycle it
/// (§2.3): `↑↓` (and `W`/`S`) move, `Enter`/`Space` choose, a digit `1`–`8`
/// chooses its row, `Esc` shuts. The menu holds the keyboard: everything
/// else is swallowed while it is up. Returns whether the keys were the
/// menu's.
pub fn keys(
    fired: crate::keys::Fired,
    digits: &[u32],
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    measured: &mut ArrangementFrame,
) -> bool {
    use baylee_client_core::prefs::Action;
    let Some(focus) = duel.arrangement_menu else {
        if fired.has(Action::ArrangementMenu) {
            toggle_menu(duel, settings, measured.seats);
            return true;
        }
        if fired.has(Action::NextArrangement) {
            cycle(duel, settings, measured);
            return true;
        }
        return false;
    };
    let rows = REMEMBER_ROW + 1;
    if fired.has(Action::Cancel) || fired.has(Action::ArrangementMenu) {
        duel.arrangement_menu = None;
    } else if fired.has(Action::NumberUp) || fired.has(Action::CursorUp) {
        duel.arrangement_menu = Some((focus + rows - 1) % rows);
    } else if fired.has(Action::NumberDown) || fired.has(Action::CursorDown) {
        duel.arrangement_menu = Some((focus + 1) % rows);
    } else if fired.has(Action::Primary) || fired.has(Action::Confirm) {
        press_row(duel, settings, measured, focus);
    } else if let Some(&digit) = digits.first()
        && (1..=8).contains(&digit)
    {
        let row = usize::try_from(digit - 1).unwrap_or(0);
        duel.arrangement_menu = Some(row);
        press_row(duel, settings, measured, row);
    }
    true
}

/// A row pressed, by the pointer or the keyboard: an arrangement offered
/// here is chosen and the menu shuts; a greyed one does nothing; the last
/// row ticks or unticks *remember*.
pub fn press_row(
    duel: &mut Duel,
    settings: &mut crate::settings::ClientSettings,
    measured: &ArrangementFrame,
    row: usize,
) {
    if row == REMEMBER_ROW {
        duel.arrangement_remember = !duel.arrangement_remember;
        // Ticking it writes what stands now, unticking forgets it: the box
        // says whether this count has a memory.
        let seats = measured.seats;
        let current = chosen(duel, &settings.table, seats);
        if duel.arrangement_remember {
            settings
                .table
                .arrangement_by_seats
                .set(seats, Some(current));
            duel.arrangement_game = None;
        } else {
            settings.table.arrangement_by_seats.set(seats, None);
            duel.arrangement_game = Some(current);
        }
        settings.save();
        return;
    }
    if let Some(&arrangement) = Arrangement::ALL.get(row)
        && pick(duel, settings, measured, arrangement)
    {
        duel.arrangement_menu = None;
    }
}

/// The per-count memory stepped by one: *Default* (`None`), then the built
/// arrangements in the menu's order, round (DESIGN-v8 §2.6).
#[must_use]
pub fn step_remembered(
    now: Option<baylee_client_core::tableview::Arrangement>,
    step: i8,
) -> Option<baylee_client_core::tableview::Arrangement> {
    use baylee_client_core::tableview::Arrangement;
    let cycle: Vec<Option<Arrangement>> = std::iter::once(None)
        .chain(Arrangement::ALL.into_iter().filter(|a| a.built()).map(Some))
        .collect();
    let at = cycle.iter().position(|c| *c == now).unwrap_or(0);
    let len = cycle.len();
    let next = if step < 0 {
        (at + len - 1) % len
    } else {
        (at + 1) % len
    };
    cycle[next]
}

// --------------------------------------------------------------- drawing

/// The pill's root: top-left, `(EDGE, EDGE)`, [`CORNER_BUTTON`] tall.
#[derive(Component)]
pub struct ArrangementPill;

/// The pill's name.
#[derive(Component)]
pub struct PillName;

/// The pill's letter disc.
#[derive(Component)]
pub struct PillLetter;

/// The veil behind an open menu: a press on it shuts the menu and reaches
/// nothing under it.
#[derive(Component)]
pub struct ArrangementVeil;

/// The open menu's panel.
#[derive(Component)]
pub struct ArrangementPanel;

/// One row of the menu.
#[derive(Component, Clone, Copy)]
pub struct ArrangementRow(pub usize);

/// The pill's and the menu's root rung: over the table's HUD (0) and the
/// end screen (1), under the report button's (900) and its form.
pub const G_SWITCHER: i32 = 800;

/// The pill's text size.
const PILL_PT: f32 = 13.0;
/// The disc's side.
const DISC: f32 = 18.0;
/// The menu's width on a window that is not a phone's.
pub const MENU_W: f32 = 460.0;
/// A row's least height: a touch target (§2.3: 44).
const ROW_H: f32 = 44.0;

/// The pill's padding either side, and the gap between its three parts.
const PILL_PAD: f32 = 6.0;

/// How wide the pill is drawn when it names `arrangement`: its disc, the
/// name at its widest in either language (the HUD's estimate, which is
/// generous — `hud::text_width`), the caret, padding and border. Below a
/// wide window the name is left out: the design leaves it out on a compact
/// one (§2.2), and at 800 × 600 the ring's left flank reaches the corner a
/// named pill would take — the pictogram alone keeps the ring's shot as v7
/// measured it rather than pushing its top down (the fallback §2.2 names). The node is **set** to this
/// width, so the corner the camera tests hold clear is the corner it stands
/// in (`camera_tests::the_arrangement_pill_lies_on_no_seat_s_place`).
#[must_use]
pub fn pill_width(arrangement: Arrangement, compact: bool) -> f32 {
    let size = PILL_PT * crate::hud::UI_SCALE;
    let name = if compact {
        0.0
    } else {
        Lang::ALL
            .iter()
            .map(|lang| crate::hud::text_width(arrangement.name().text(*lang), size, true))
            .fold(0.0, f32::max)
            + PILL_PAD
    };
    let caret = size * 0.6;
    2.0 * PILL_PAD + DISC + PILL_PAD + name + caret + 2.0
}

/// Where the pill stands when it names `arrangement` in a window `window`
/// big: what the camera tests hold that arrangement's seats clear of (§2.2).
#[must_use]
pub fn pill_corner(window: Vec2, arrangement: Arrangement) -> Rect {
    let compact = pictogram_alone(TableFrame::of(window.x, window.y));
    Rect::new(
        EDGE,
        EDGE,
        EDGE + pill_width(arrangement, compact),
        EDGE + CORNER_BUTTON,
    )
}

/// Whether the pill shows its pictogram without the name: below a wide
/// window ([`pill_width`]).
#[must_use]
pub fn pictogram_alone(frame: TableFrame) -> bool {
    matches!(frame, TableFrame::Compact | TableFrame::Narrow)
}

/// The arrangement the pill names: the one chosen where it is offered (the
/// Turntable with rows by that name, not what it resolved to), else the ring
/// the table fell back to.
#[must_use]
pub fn pill_names(duel: &Duel, table: &TableView, measured: &ArrangementFrame) -> Arrangement {
    let chosen = chosen(duel, table, measured.seats);
    if chosen.offered(measured.seats, measured.class()).is_ok() {
        chosen
    } else {
        Arrangement::Ring
    }
}

/// The pill's ground: the dialog's, at .88 (§2.1).
fn pill_ground() -> Color {
    palette::DIALOG.with_alpha(0.88)
}

/// What the pill and the menu were last drawn from.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct SwitcherRevision {
    shown: bool,
    effective: Option<Arrangement>,
    compact: bool,
    menu: Option<usize>,
    remember: bool,
    seats: usize,
    frame: Option<TableFrame>,
    lang: Option<Lang>,
    resolved_to: Option<Arrangement>,
}

/// Whether the pill stands: from three seats, not on a phone (its switcher
/// is the game menu's row, §2.2), not under the end screen.
fn pill_shown(duel: &Duel, measured: &ArrangementFrame) -> bool {
    measured.switchable() && measured.class() != TableFrame::Phone && duel.ending().is_none()
}

/// The pill, the menu's panel and its veil: what a redraw takes down.
type SwitcherParts = Or<(
    With<ArrangementPill>,
    With<ArrangementPanel>,
    With<ArrangementVeil>,
)>;

/// Builds and redraws the pill and the menu when what they say changed, and
/// takes both down off the table.
#[allow(clippy::too_many_arguments)] // one switcher, its two surfaces
pub fn sync_switcher(
    mut commands: Commands,
    phase: Option<Res<State<DuelPhase>>>,
    fonts: Option<Res<UiFonts>>,
    duel: Res<Duel>,
    measured: Res<ArrangementFrame>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut revision: ResMut<SwitcherRevision>,
    standing: Query<Entity, SwitcherParts>,
) {
    let up = phase.is_some_and(|p| matches!(p.get(), DuelPhase::Playing | DuelPhase::Finished));
    let Some(fonts) = fonts.filter(|_| up) else {
        for e in &standing {
            commands.entity(e).despawn();
        }
        *revision = SwitcherRevision::default();
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let table = settings.as_ref().map(|s| s.table).unwrap_or_default();
    let resolved_to = (chosen(&duel, &table, measured.seats) == Arrangement::TurntableRows)
        .then_some(duel.arrangement);
    let next = SwitcherRevision {
        shown: pill_shown(&duel, &measured),
        effective: Some(duel.arrangement),
        compact: pictogram_alone(measured.class()),
        menu: duel.arrangement_menu.filter(|_| duel.ending().is_none()),
        remember: duel.arrangement_remember,
        seats: measured.seats,
        frame: measured.frame,
        lang: Some(lang),
        resolved_to,
    };
    if *revision == next && (standing.iter().next().is_some() || !next.shown) {
        return;
    }
    revision.clone_from(&next);
    for e in &standing {
        commands.entity(e).despawn();
    }
    let chosen_now = chosen(&duel, &table, measured.seats);
    if next.shown {
        let named = pill_names(&duel, &table, &measured);
        spawn_pill(&mut commands, &fonts, lang, named, next.compact);
    }
    if let Some(focus) = next.menu {
        let phone = measured.class() == TableFrame::Phone;
        spawn_menu(
            &mut commands,
            &fonts,
            lang,
            MenuLook {
                chosen: chosen_now,
                resolved_to,
                focus,
                remember: next.remember,
                seats: measured.seats,
                frame: measured.class(),
                phone,
            },
        );
    }
}

/// The pill: the arrangement's disc, its name (not on a compact window),
/// and a caret.
fn spawn_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    arrangement: Arrangement,
    compact: bool,
) {
    let ground = pill_ground();
    let pill = commands
        .spawn((
            ArrangementPill,
            Node {
                position_type: PositionType::Absolute,
                left: px(EDGE),
                top: px(EDGE),
                height: px(CORNER_BUTTON),
                width: px(pill_width(arrangement, compact)),
                overflow: Overflow::clip(),
                padding: UiRect::horizontal(px(PILL_PAD)),
                column_gap: px(PILL_PAD),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(palette::DIALOG_LINE),
            Button,
            Feel::new(ground),
            GlobalZIndex(G_SWITCHER),
        ))
        .observe(pill_pressed)
        .id();
    let disc = letter_disc(commands, fonts, arrangement, palette::DIALOG_SOFT);
    commands.entity(disc).insert(PillLetter);
    commands.entity(pill).add_child(disc);
    if !compact {
        let name = commands
            .spawn((
                PillName,
                Text::new(arrangement.name().text(lang)),
                tf_bold(fonts, PILL_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pill).add_child(name);
    }
    let caret = commands
        .spawn((
            Text::new("\u{25be}"),
            tf(fonts, PILL_PT),
            TextColor(palette::DIALOG_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_child(caret);
}

/// A placeholder pictogram: the arrangement's letter in a ring (§7.6).
fn letter_disc(
    commands: &mut Commands,
    fonts: &UiFonts,
    arrangement: Arrangement,
    ink: Color,
) -> Entity {
    commands
        .spawn((
            Node {
                width: px(DISC),
                height: px(DISC),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(DISC / 2.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(ink),
            Pickable::IGNORE,
            children![(
                Text::new(arrangement.letter().to_string()),
                tf_bold(fonts, 11.0),
                TextColor(ink),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// What one drawing of the menu shows.
#[derive(Clone, Copy)]
struct MenuLook {
    chosen: Arrangement,
    resolved_to: Option<Arrangement>,
    focus: usize,
    remember: bool,
    seats: usize,
    frame: TableFrame,
    phone: bool,
}

/// The menu: one row per arrangement — disc, name, what it does, the reason
/// it is greyed or its *experimentell* tag, its digit — then *remember for
/// this seat count* under a rule, and the note on the letters (§2.1). On a
/// phone it is a sheet over the whole window.
fn spawn_menu(commands: &mut Commands, fonts: &UiFonts, lang: Lang, look: MenuLook) {
    commands
        .spawn((
            ArrangementVeil,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(G_SWITCHER - 1),
        ))
        .observe(veil_pressed);
    let node = if look.phone {
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12.0)),
            row_gap: px(2.0),
            overflow: Overflow::scroll_y(),
            ..default()
        }
    } else {
        Node {
            position_type: PositionType::Absolute,
            left: px(EDGE),
            top: px(EDGE + CORNER_BUTTON + 6.0),
            width: px(MENU_W),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(8.0)),
            row_gap: px(2.0),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8.0)),
            ..default()
        }
    };
    let panel = commands
        .spawn((
            ArrangementPanel,
            node,
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            GlobalZIndex(G_SWITCHER),
            Pickable::default(),
        ))
        .id();
    for (row, arrangement) in Arrangement::ALL.into_iter().enumerate() {
        let entity = arrangement_row(commands, fonts, lang, look, row, arrangement);
        commands.entity(panel).add_child(entity);
    }
    let rule = commands
        .spawn((
            Node {
                height: px(1),
                margin: UiRect::vertical(px(4.0)),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(rule);
    let remember = remember_row(commands, fonts, lang, look);
    commands.entity(panel).add_child(remember);
    let note = commands
        .spawn((
            Text::new(Phrase::ArrLettersNote.text(lang)),
            tf(fonts, 11.0),
            TextColor(palette::DIALOG_SOFT),
            Node {
                margin: UiRect::top(px(4.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(note);
    if !look.phone {
        let hint = commands
            .spawn((
                Text::new(Phrase::ArrKeysHint.text(lang)),
                tf(fonts, 11.0),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(panel).add_child(hint);
    }
}

/// One arrangement's row.
#[allow(clippy::too_many_lines)] // one row: disc, three lines of words, a digit
fn arrangement_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    look: MenuLook,
    row: usize,
    arrangement: Arrangement,
) -> Entity {
    let offered = arrangement.offered(look.seats, look.frame);
    let live = offered.is_ok();
    let current = arrangement == look.chosen;
    let focused = row == look.focus;
    let ground = if focused {
        palette::DIALOG_LIT
    } else {
        palette::DIALOG
    };
    let (ink, soft) = if live {
        (palette::DIALOG_INK, palette::DIALOG_SOFT)
    } else {
        (
            palette::DIALOG_SOFT.with_alpha(0.55),
            palette::DIALOG_SOFT.with_alpha(0.55),
        )
    };
    let mut line = commands.spawn((
        ArrangementRow(row),
        Node {
            min_height: px(ROW_H),
            padding: UiRect::axes(px(8.0), px(4.0)),
            column_gap: px(10.0),
            align_items: AlignItems::Center,
            border: UiRect::left(px(2.0)),
            border_radius: BorderRadius::all(px(4.0)),
            ..default()
        },
        BackgroundColor(ground),
        BorderColor::all(if current {
            palette::DIALOG_INK
        } else {
            Color::NONE
        }),
        Button,
    ));
    if live {
        line.insert(Feel::new(ground));
    }
    let line = line.observe(row_pressed).id();
    let disc = letter_disc(commands, fonts, arrangement, ink);
    let mut words: Vec<Entity> = Vec::new();
    words.push(
        commands
            .spawn((
                Text::new(arrangement.name().text(lang)),
                tf_bold(fonts, 13.0),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id(),
    );
    let blurb = match (arrangement, look.resolved_to) {
        (Arrangement::TurntableRows, Some(to)) if current => format!(
            "{} {}",
            arrangement.blurb().text(lang),
            Phrase::ArrResolvedTo.fill(lang, &[to.name().text(lang)])
        ),
        _ => arrangement.blurb().text(lang).to_string(),
    };
    words.push(
        commands
            .spawn((
                Text::new(blurb),
                tf(fonts, 11.0),
                TextColor(soft),
                Pickable::IGNORE,
            ))
            .id(),
    );
    let tag = match offered {
        Err(Phrase::ArrComing) => Some(Phrase::ArrComing.fill(lang, &[arrangement.package()])),
        Err(reason) => Some(reason.text(lang).to_string()),
        Ok(()) if arrangement.experimental() => {
            Some(Phrase::ArrExperimental.text(lang).to_string())
        }
        Ok(()) => None,
    };
    if let Some(tag) = tag {
        words.push(
            commands
                .spawn((
                    Text::new(tag),
                    crate::hud::tf_italic(fonts, 11.0),
                    TextColor(soft),
                    Pickable::IGNORE,
                ))
                .id(),
        );
    }
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&words)
        .id();
    let cap = commands
        .spawn((
            Text::new((row + 1).to_string()),
            tf(fonts, 11.0),
            TextColor(soft),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_children(&[disc, column, cap]);
    line
}

/// The last row: *remember for N seats*, a tick box.
fn remember_row(commands: &mut Commands, fonts: &UiFonts, lang: Lang, look: MenuLook) -> Entity {
    let focused = look.focus == REMEMBER_ROW;
    let ground = if focused {
        palette::DIALOG_LIT
    } else {
        palette::DIALOG
    };
    let tick = if look.remember {
        "\u{2611}"
    } else {
        "\u{2610}"
    };
    commands
        .spawn((
            ArrangementRow(REMEMBER_ROW),
            Node {
                min_height: px(ROW_H),
                padding: UiRect::axes(px(8.0), px(4.0)),
                column_gap: px(10.0),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(4.0)),
                ..default()
            },
            BackgroundColor(ground),
            Button,
            Feel::new(ground),
            children![
                (
                    Text::new(tick),
                    tf(fonts, 15.0),
                    TextColor(palette::DIALOG_INK),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(Phrase::ArrRememberForSeats.fill(lang, &[&look.seats.to_string()])),
                    tf(fonts, 13.0),
                    TextColor(palette::DIALOG_INK),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .observe(row_pressed)
        .id()
}

/// The pill pressed: the menu opens, or shuts.
fn pill_pressed(
    mut click: On<Pointer<Click>>,
    mut duel: ResMut<Duel>,
    settings: Res<crate::settings::ClientSettings>,
    measured: Res<ArrangementFrame>,
) {
    click.propagate(false);
    toggle_menu(&mut duel, &settings, measured.seats);
}

/// A row pressed.
fn row_pressed(
    mut click: On<Pointer<Click>>,
    rows: Query<&ArrangementRow>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<crate::settings::ClientSettings>,
    measured: Res<ArrangementFrame>,
) {
    click.propagate(false);
    if let Ok(row) = rows.get(click.entity) {
        duel.arrangement_menu = Some(row.0);
        press_row(&mut duel, &mut settings, &measured, row.0);
    }
}

/// A press beside the menu shuts it, and goes no further.
fn veil_pressed(mut click: On<Pointer<Click>>, mut duel: ResMut<Duel>) {
    click.propagate(false);
    duel.arrangement_menu = None;
}

/// The pill's flash after `Shift+P`: its name and disc go from the quiet
/// ink to the full one and back over [`FLASH_SECS`]. Writes only while a
/// flash runs, and once more as it ends.
pub fn flash_the_pill(
    measured: Res<ArrangementFrame>,
    mut was: Local<bool>,
    mut names: Query<&mut TextColor, With<PillName>>,
) {
    let running = measured.flash > 0.0;
    if !running && !*was {
        return;
    }
    *was = running;
    let t = 1.0 - measured.flash / FLASH_SECS;
    // Up in the first third, held, down in the last third.
    let lift = (3.0 * t.min(1.0 - t)).clamp(0.0, 1.0);
    let ink = palette::DIALOG_SOFT.mix(&palette::DIALOG_INK, lift);
    for mut colour in &mut names {
        if colour.0 != ink {
            colour.0 = ink;
        }
    }
}

/// Registers the switcher's systems. Its own call, because the present
/// tuple is at bevy's twenty.
pub fn plugin(app: &mut App) {
    app.init_resource::<ArrangementFrame>()
        .init_resource::<crate::table::GlideReport>()
        .init_resource::<SwitcherRevision>();
    app.add_systems(
        Update,
        (
            choose.before(crate::table::track_canvas),
            lay_the_interest
                .after(choose)
                .before(crate::table::sync_scene),
        )
            .in_set(crate::DuelSet::Present)
            .run_if(not(in_state(DuelPhase::Closed))),
    );
    app.add_systems(
        Update,
        (sync_switcher, flash_the_pill.after(sync_switcher))
            .in_set(crate::DuelSet::Present)
            .run_if(not(in_state(DuelPhase::Closed))),
    );
}

#[cfg(test)]
#[path = "arrangement_tests.rs"]
mod tests;
