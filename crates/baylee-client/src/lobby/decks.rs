//! The Decks screen (the shell design, `DESIGN-v5.md` §6; WP3): the
//! player's shelf and the house decks in one centred grid of tiles, the
//! history of a deck in a sheet over it, a house deck's cards in another.
//!
//! - A tile pictures its deck with Scryfall's `art_crop` and credits the
//!   artist under it; without a known artist it shows the identity gradient
//!   alone (`shelf::DeckArt`).
//! - Its primary is **Use for next game** (S-5); the next game's own tile
//!   has none and offers Edit. `⋯` holds Duplicate · History… · Favourite ·
//!   Delete. Delete takes the deck off the shelf at once and sends nothing
//!   until its Undo toast has run out, the screen changes, the player signs
//!   out or the client closes (S-10).
//! - The grid is `minmax(320 × factor, 1fr)` over the body: four tiles a row
//!   at 1920, three at 1180, two narrower; two on a phone, where the
//!   toolbar is one 44-px row (M4-6).

use super::menus::{self, ShellMenu};
use super::orders;
use super::parts;
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::surfaces::{self, SheetWidth};
use crate::shellkit::{Frame as ShellFrame, Role, px_fixed, tokens};
use crate::tour::TourAnchor;
use baylee_client_core::tour::Anchor;
use client_core::lobby::library::{AfterCopy, Compare, Page};
use client_core::lobby::shelf::{self, Sort};

/// The screen's two tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum DecksTab {
    /// The player's own decks.
    #[default]
    Mine,
    /// The decks the project publishes.
    House,
}

/// How the Decks shelf is presented on both tabs (`Meine Decks` & `Haus-Decks`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum DecksView {
    /// Rich visual gallery cards with art banner, commander subtitle, badges, and actions.
    #[default]
    Gallery,
    /// Full-width horizontal rows with inline metadata and quick actions.
    List,
    /// High-density multi-column compact cards for large collections.
    Compact,
}

/// What the Decks screen remembers between rebuilds that the lobby's model
/// does not: the tab, the sort, a house deck being looked at, the history
/// sheet's choices. Presses write it; nothing else does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent filter toggles on the shelf
pub(crate) struct DecksUi {
    /// Which tab is up.
    pub(crate) tab: DecksTab,
    /// The shelf's order.
    pub(crate) sort: Sort,
    /// How the shelf is displayed (gallery tiles, list rows, or compact grid).
    pub(crate) view: DecksView,
    /// Active colour-identity filter letters (`W`, `U`, `B`, `R`, `G`, `C`).
    pub(crate) colors: Vec<char>,
    /// Show only starred/favourite decks (`Meine Decks`).
    pub(crate) favourites_only: bool,
    /// Show only decks with 0 unplayable/blocked cards.
    pub(crate) playable_only: bool,
    /// The house deck whose cards the preview sheet shows.
    pub(crate) preview: Option<usize>,
    /// Whether the Vorlagen preview sheet omits type grouping (`false` = grouped by type).
    pub(crate) preview_ungrouped: bool,
    /// How the Vorlagen preview sheet displays its cards (`List`, `Gallery` grid, `Compact`).
    pub(crate) preview_view: DecksView,
    /// The history sheet lists every card, not only what changed.
    pub(crate) show_all: bool,
    /// The format filter index into `DECK_FORMATS`.
    pub(crate) format: Option<u8>,
    /// On a phone, the search box stands over the tabs.
    pub(crate) searching: bool,
}

impl DecksUi {
    /// Whether any quick filter on the Decks shelf is currently active.
    pub(crate) fn has_active_filters(&self) -> bool {
        !self.colors.is_empty()
            || self.format.is_some()
            || (self.tab == DecksTab::Mine && self.favourites_only)
            || self.playable_only
    }

    /// Resets all quick filter chips on the Decks shelf.
    pub(crate) fn clear_filters(&mut self) {
        self.colors.clear();
        self.format = None;
        self.favourites_only = false;
        self.playable_only = false;
    }
}

/// An Undo the toast lane offers (S-10).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Undo {
    /// What it undoes.
    pub(crate) kind: UndoKind,
    /// Seconds left.
    pub(crate) left: f32,
}

/// What an Undo undoes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UndoKind {
    /// A deck taken off the shelf (`Lobby::staged_delete`).
    Delete,
    /// A version restored from the history sheet (`Library::restored`).
    Restore,
    /// The tours switched off by the bubble's box
    /// (`LobbyState::tours_before`, TOURS.md §1.7).
    ToursOff,
    /// Settings › Restart tours.
    ToursRestarted,
}

/// How long an Undo is offered (§2.4: a toast's 6 s).
pub(crate) const UNDO_SECS: f32 = 6.0;

/// A control of the Decks screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DecksPress {
    /// Show a tab.
    Tab(DecksTab),
    /// Order the shelf.
    Sort(Sort),
    /// The phone's search icon: the box over the tabs.
    OpenSearch,
    /// This deck is the next game's.
    #[allow(dead_code)]
    Use(usize),
    /// Open the builder on it.
    Edit(usize),
    /// A copy of it, kept on the shelf.
    Duplicate(usize),
    /// Its history, in a sheet.
    History(usize),
    /// A house deck's history, read-only (`DESIGN` §C.3, Q-C4).
    #[allow(dead_code)]
    HouseHistory(usize),
    /// Star or unstar it.
    Favourite(usize),
    /// Off the shelf, with an Undo.
    Delete(usize),
    /// The Undo toast's action.
    Undo,
    /// The builder on a new deck.
    NewDeck,
    /// The builder with the import dialog over it.
    Import,
    /// A house deck's cards, in a sheet.
    Preview(usize),
    /// A copy of a house deck, kept here (**Add**).
    Add(usize),
    /// A copy, made the next game's deck (**Add and use**).
    AddAndUse(usize),
    /// Select a format filter (`u8::MAX` clears the filter, `0..9` selects from `DECK_FORMATS`).
    Format(u8),
    /// Toggle type grouping in the Vorlagen preview sheet.
    TogglePreviewGrouped,
    /// Switch the view mode inside the Vorlagen preview sheet.
    SetPreviewView(DecksView),
    /// Close the preview or the history sheet.
    CloseSheet,
    /// The history sheet: show this version.
    Version(i32),
    /// The history sheet: every card, or only the changes.
    ShowAll,
    /// The history sheet: restore the version shown.
    Restore,
    /// The history sheet: the version shown, in the builder's export dialog.
    ExportVersion,
    /// A diff row: a stop for the ring to walk, which does nothing.
    DiffRow,
    /// Ask the library again after a failure.
    Retry,
    /// The history sheet's compare bar: `0` Previous, `1` Current, `2`
    /// Pick….
    Compare(u8),
    /// Over the builder: keep the unsaved edits, restore nothing.
    KeepEdits,
    /// Over the builder: discard the unsaved edits and restore.
    DiscardAndRestore,
    /// Switch the shelf's view mode (Gallery, List, Compact).
    SetView(DecksView),
    /// Toggle a colour in the colour-identity filter (`W`, `U`, `B`, `R`, `G`, `C`).
    ToggleColor(char),
    /// Toggle showing only favourite decks.
    ToggleFavouritesOnly,
    /// Toggle showing only 100% playable decks.
    TogglePlayableOnly,
    /// Reset all active shelf filters and search text.
    ClearFilters,
}

/// All play modes / formats selectable in the Decks format dropdown:
/// `(wire_id, icon, german_label, english_label)`.
pub(crate) const DECK_FORMATS: [(&str, char, &str, &str); 9] = [
    ("commander", '\u{f521}', "Commander", "Commander"),
    ("highlander", '\u{f3ed}', "Highlander", "Highlander"),
    ("standard", '\u{f005}', "Standard", "Standard"),
    ("pioneer", '\u{f14e}', "Pioneer", "Pioneer"),
    ("modern", '\u{f0e7}', "Modern", "Modern"),
    ("legacy", '\u{f02d}', "Legacy", "Legacy"),
    ("vintage", '\u{f51f}', "Vintage", "Vintage"),
    ("pauper", '\u{f51e}', "Pauper", "Pauper"),
    ("freeform", '\u{f534}', "Freiform", "Freeform"),
];

/// Returns the localized name and icon for a format index (`None` = Alle Formate).
fn format_filter_display(lang: Lang, format: Option<u8>) -> (char, &'static str) {
    if let Some((_, icon, de, en)) = format.and_then(|i| DECK_FORMATS.get(usize::from(i))) {
        (*icon, if lang == Lang::De { *de } else { *en })
    } else {
        (
            '\u{f0b0}',
            if lang == Lang::De {
                "Alle Formate"
            } else {
                "All formats"
            },
        )
    }
}

/// Draws the Decks screen under the header.
#[allow(clippy::too_many_lines)] // one screen, read top to bottom
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    prefs: &baylee_client_core::prefs::Preferences,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let ui = &state.decks;
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px_fixed(kit.m.body)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);
    let panel = commands
        .spawn((
            Role::Panel,
            Node {
                width: percent(100),
                max_width: px_fixed(if kit.m.frame == ShellFrame::Vast {
                    kit.m.collection_max
                } else {
                    kit.m.body_max
                }),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            super::dock::Dock(5),
        ))
        .id();
    commands.entity(body).add_child(panel);

    let (sort_anchor, mut format_anchor) = toolbar(commands, panel, state, metrics, kit);
    if kit.m.frame != ShellFrame::Phone {
        format_anchor = Some(filter_strip(commands, panel, state, prefs, kit));
    }

    // What went wrong last, in the panel: a library failure with its retry.
    if let Some(error) = lobby.library().error.as_deref() {
        let retry = controls::button(
            commands,
            kit,
            Phrase::ShellRetry.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Decks(DecksPress::Retry),
        );
        let line = crate::shellkit::states::error_line(commands, kit, error, retry);
        commands.entity(panel).add_child(line);
    }

    let scroll = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Scrollable(List::Library),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Library))),
        ))
        .id();
    commands.entity(panel).add_child(scroll);
    let grid = grid(commands, kit, ui.view);
    commands.entity(scroll).add_child(grid);

    match ui.tab {
        DecksTab::Mine => mine(commands, root, grid, scroll, state, prefs, kit),
        DecksTab::House => house(commands, root, grid, scroll, state, kit),
    }

    let hints = match ui.tab {
        DecksTab::Mine => vec![
            ("/", Phrase::ShellKeySearch.text(lang)),
            ("Enter", Phrase::ShellEdit.text(lang)),
            ("e", Phrase::ShellEdit.text(lang)),
        ],
        DecksTab::House => vec![
            ("/", Phrase::ShellKeySearch.text(lang)),
            ("Enter", Phrase::HousePreview.text(lang)),
        ],
    };
    if let Some(hint) = parts::key_hint(commands, kit, &hints) {
        commands.entity(body).add_child(hint);
        commands
            .entity(hint)
            .entry::<Node>()
            .and_modify(|mut n| n.margin = UiRect::top(px(8)));
    }

    // Open sort or format dropdown menu, anchored to its button.
    if state.menu == Some(ShellMenu::DeckSort)
        && let Some(anchor) = sort_anchor
    {
        let items: Vec<_> = Sort::ALL
            .into_iter()
            .map(|sort| {
                menus::item(
                    sort.phrase().text(lang),
                    Press::Decks(DecksPress::Sort(sort)),
                )
            })
            .collect();
        menus::draw(commands, root, kit, anchor, items);
    }
    if state.menu == Some(ShellMenu::DeckFormat)
        && let Some(anchor) = format_anchor
    {
        let all_label = if lang == Lang::De {
            "Alle Formate"
        } else {
            "All formats"
        };
        let mut items = Vec::with_capacity(DECK_FORMATS.len() + 1);
        items.push(menus::item(
            all_label,
            Press::Decks(DecksPress::Format(u8::MAX)),
        ));
        for (i, (_, _, de, en)) in DECK_FORMATS.iter().enumerate() {
            let at = u8::try_from(i).unwrap_or(0);
            let label = if lang == Lang::De { *de } else { *en };
            items.push(menus::item(label, Press::Decks(DecksPress::Format(at))));
        }
        menus::draw(commands, root, kit, anchor, items);
    }

    if ui.preview.is_some() && ui.tab == DecksTab::House {
        preview_sheet(commands, root, state, kit, scrolled);
    } else if matches!(lobby.library().page, Some(Page::History(_))) {
        super::history::sheet(commands, root, state, kit);
    }
}

/// A deck tile's minimum width at the default step, before the big-screen scale.
pub(super) const TILE_WIDTH: f32 = 304.0;

/// The container the tiles or rows flow into, shaped for `view`.
/// Uses `minmax(px(track_width), 1fr)` with `AutoFill` so tiles in a row
/// symmetrically fill the full width of the shelf while keeping single tiles
/// constrained to one column track.
fn grid(commands: &mut Commands, kit: Kit, view: DecksView) -> Entity {
    if view == DecksView::List && kit.m.frame != ShellFrame::Phone {
        return commands
            .spawn((
                Node {
                    display: Display::Flex,
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(10.0),
                    align_items: AlignItems::Stretch,
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
    }
    let track_width = match view {
        DecksView::Compact => 236.0,
        DecksView::Gallery | DecksView::List => TILE_WIDTH,
    };
    let columns = if kit.m.frame == ShellFrame::Phone {
        vec![RepeatedGridTrack::flex(2, 1.0)]
    } else {
        vec![RepeatedGridTrack::minmax(
            GridTrackRepetition::AutoFill,
            MinTrackSizingFunction::Px(kit.m.scaled(track_width)),
            MaxTrackSizingFunction::Fraction(1.0),
        )]
    };
    commands
        .spawn((
            Node {
                display: Display::Grid,
                width: percent(100),
                grid_template_columns: columns,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                align_items: AlignItems::Stretch,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// Checks whether a deck's `identity` matches the active colour filter.
fn matches_colors(identity: &str, active: &[char]) -> bool {
    if active.is_empty() {
        return true;
    }
    active.iter().all(|&want| {
        if want == 'C' {
            identity.trim().is_empty()
        } else {
            identity.chars().any(|c| c.eq_ignore_ascii_case(&want))
        }
    })
}

/// The toolbar: tabs, search, view switcher, sort, New deck, Import. Answers
/// `(sort_anchor, format_anchor)`.
#[allow(clippy::too_many_lines)] // one row of controls, read left to right
fn toolbar(
    commands: &mut Commands,
    panel: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
) -> (Option<Entity>, Option<Entity>) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let ui = &state.decks;
    let phone = kit.m.frame == ShellFrame::Phone;
    let bar = parts::row(commands, kit, true);
    commands
        .entity(bar)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.width = percent(100);
            n.min_height = px_fixed(if phone { 44.0 } else { kit.m.control });
            n.flex_shrink = 0.0;
        });
    commands.entity(panel).add_child(bar);
    let mine = lobby
        .decks()
        .len()
        .saturating_sub(usize::from(lobby.staged_delete().is_some()));
    let house_count = lobby.library().house.len();
    let typing = lobby.focus() == Field::DeckSearch && lobby.typing_here();
    if !(phone && (ui.searching || typing)) {
        let tabs = controls::tabs(
            commands,
            kit,
            &[
                (
                    Phrase::ShellMyDecks.text(lang),
                    Some(u32::try_from(mine).unwrap_or(u32::MAX)),
                ),
                (
                    Phrase::ShellHouseDecks.text(lang),
                    (house_count > 0).then(|| u32::try_from(house_count).unwrap_or(u32::MAX)),
                ),
            ],
            usize::from(ui.tab == DecksTab::House),
            |i| {
                Press::Decks(DecksPress::Tab(if i == 0 {
                    DecksTab::Mine
                } else {
                    DecksTab::House
                }))
            },
        );
        orders::items_of(commands, tabs, &orders::DECKS, "tabs");
        commands.entity(tabs).insert(TourAnchor(Anchor::DecksTabs));
        commands.queue(move |world: &mut World| {
            let house = world
                .get::<Children>(tabs)
                .and_then(|children| children.get(1).copied());
            if let Some(house) = house {
                world
                    .entity_mut(house)
                    .insert(TourAnchor(Anchor::DecksHouseTab));
            }
        });
        commands.entity(bar).add_child(tabs);
    }
    let gap = parts::grow(commands);
    commands.entity(bar).add_child(gap);
    if phone && !ui.searching && !typing {
        let glass = parts::icon_button(
            commands,
            kit,
            crate::hud::glyph::MAGNIFIER,
            Press::Decks(DecksPress::OpenSearch),
        );
        orders::stop(commands, glass, &orders::DECKS, "search");
        commands.entity(bar).add_child(glass);
    } else {
        let hunt = commands
            .spawn((
                Node {
                    width: kit.m.px(if phone { 240.0 } else { 280.0 }),
                    flex_shrink: 1.0,
                    min_width: px(120),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let field = text_field(
            commands,
            kit.fonts,
            metrics,
            "",
            &FieldLook {
                buffer: lobby.buffer(Field::DeckSearch),
                focused: typing,
                mask: None,
                press: Press::Shared(SharedPress::Focus(Field::DeckSearch)),
                lead: Some(crate::hud::glyph::MAGNIFIER),
                hint: Some(Phrase::DecksSearchHint.text(lang)),
                tail: None,
            },
        );
        orders::field(commands, field, &orders::DECKS, "search");
        commands.entity(hunt).add_child(field);
        commands.entity(bar).add_child(hunt);
    }
    if !phone {
        let views = view_switcher(commands, kit, lang, ui.view);
        commands.entity(bar).add_child(views);
    }
    let mut sort_anchor = None;
    let mut format_anchor = None;
    if phone {
        let (_, fmt_label) = format_filter_display(lang, ui.format);
        let fmt_btn = parts::menu_button(
            commands,
            kit,
            fmt_label,
            Weight::Ghost,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::DeckFormat)),
        );
        orders::item(commands, fmt_btn, &orders::DECKS, "formats", 0);
        format_anchor = Some(fmt_btn);
        commands.entity(bar).add_child(fmt_btn);
    }
    if ui.tab == DecksTab::Mine {
        let label = Phrase::DecksSortBy.fill(lang, &[ui.sort.phrase().text(lang)]);
        let sort = if phone {
            parts::icon_button(
                commands,
                kit,
                parts::ELLIPSIS,
                Press::Shared(SharedPress::OpenMenu(ShellMenu::DeckSort)),
            )
        } else {
            parts::menu_button(
                commands,
                kit,
                &label,
                Weight::Ghost,
                Press::Shared(SharedPress::OpenMenu(ShellMenu::DeckSort)),
            )
        };
        orders::stop(commands, sort, &orders::DECKS, "sort");
        sort_anchor = Some(sort);
        commands.entity(bar).add_child(sort);
        let new = controls::button_with_icon(
            commands,
            kit,
            '\u{f067}',
            Phrase::NewDeck.text(lang),
            Weight::Primary,
            Live::Yes,
            Some("n"),
            Press::Decks(DecksPress::NewDeck),
        );
        let import = controls::button_with_icon(
            commands,
            kit,
            '\u{f56f}',
            Phrase::ImportDeck.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Decks(DecksPress::Import),
        );
        orders::stop(commands, new, &orders::DECKS, "new");
        orders::stop(commands, import, &orders::DECKS, "import");
        commands
            .entity(import)
            .insert(TourAnchor(Anchor::DecksImport));
        commands.entity(bar).add_children(&[new, import]);
    }
    (sort_anchor, format_anchor)
}

/// Unified 3-option segmented view-mode switcher (`Gallery`, `List`, `Compact`).
fn view_switcher(commands: &mut Commands, kit: Kit, lang: Lang, current: DecksView) -> Entity {
    let modes = [DecksView::Gallery, DecksView::List, DecksView::Compact];
    let active = modes.iter().position(|&m| m == current).unwrap_or(0);
    controls::segmented_icons(
        commands,
        kit,
        &[
            (
                '\u{f009}',
                if lang == Lang::De {
                    "Kachel-Ansicht"
                } else {
                    "Gallery view"
                },
            ),
            (
                '\u{f03a}',
                if lang == Lang::De {
                    "Listen-Ansicht"
                } else {
                    "List view"
                },
            ),
            (
                '\u{f00a}',
                if lang == Lang::De {
                    "Kompakt-Raster"
                } else {
                    "Compact grid"
                },
            ),
        ],
        active,
        |i| Press::Decks(DecksPress::SetView(modes[i])),
    )
}

/// Secondary `QoL` filter & status strip on desktop: colour-identity filter pips,
/// unified format dropdown with icon, Favourites / Playable toggles, filter reset,
/// and collection count summary. Answers the format dropdown's anchor entity.
#[allow(clippy::too_many_lines)]
fn filter_strip(
    commands: &mut Commands,
    panel: Entity,
    state: &LobbyState,
    prefs: &baylee_client_core::prefs::Preferences,
    kit: Kit,
) -> Entity {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let ui = &state.decks;
    let strip = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: kit.m.px(36.0),
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(4.0)),
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.0,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.38)),
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(strip);

    // WUBRG + C colour-identity filter pips (tightly grouped)
    let pip_row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for c in ['W', 'U', 'B', 'R', 'G', 'C'] {
        let active = ui.colors.contains(&c);
        let (fill, ink) = crate::shellkit::surfaces::identity_colours(c);
        let pip_face = commands
            .spawn((
                Role::Button,
                Node {
                    width: kit.m.px(26.0),
                    height: kit.m.px(26.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px_fixed(if active { 2.0 } else { 1.0 })),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(if active {
                    fill.with_alpha(0.92)
                } else {
                    fill.with_alpha(0.22)
                }),
                BorderColor::all(if active {
                    tokens::ACCENT
                } else {
                    tokens::BORDER
                }),
                UiTransform::default(),
                crate::ambience::Feel::tinting_to(
                    if active {
                        fill.with_alpha(0.92)
                    } else {
                        fill.with_alpha(0.22)
                    },
                    fill.with_alpha(0.55),
                ),
                Pickable::IGNORE,
            ))
            .id();
        let symbol = crate::manaui::spawn_identity(
            commands,
            kit.fonts,
            &c.to_string(),
            kit.m.scaled(14.0).max(13.0),
            px_fixed(0.0),
        );
        let _ = ink;
        commands.entity(pip_face).add_child(symbol);
        let pip_btn = controls::hit(
            commands,
            kit,
            pip_face,
            Press::Decks(DecksPress::ToggleColor(c)),
        );
        commands
            .entity(pip_btn)
            .entry::<Node>()
            .and_modify(move |mut n| {
                n.min_width = kit.m.px(26.0);
                n.min_height = kit.m.px(26.0);
            });
        commands.entity(pip_row).add_child(pip_btn);
    }
    commands.entity(strip).add_child(pip_row);

    // Unified Format filter dropdown button with icon + caret
    let (fmt_icon, fmt_label) = format_filter_display(lang, ui.format);
    let format_btn = format_dropdown_pill(
        commands,
        kit,
        fmt_icon,
        fmt_label,
        ui.format.is_some(),
        Press::Shared(SharedPress::OpenMenu(ShellMenu::DeckFormat)),
    );
    orders::item(commands, format_btn, &orders::DECKS, "formats", 0);
    commands.entity(strip).add_child(format_btn);

    if ui.tab == DecksTab::Mine {
        let fav_label = if lang == Lang::De {
            "Favoriten"
        } else {
            "Favourites"
        };
        let fav_chip = quick_filter_pill(
            commands,
            kit,
            Some(parts::STAR),
            fav_label,
            ui.favourites_only,
            Press::Decks(DecksPress::ToggleFavouritesOnly),
        );
        commands.entity(strip).add_child(fav_chip);
    }

    let playable_label = if lang == Lang::De {
        "100% spielbar"
    } else {
        "100% playable"
    };
    let playable_chip = quick_filter_pill(
        commands,
        kit,
        Some('\u{f00c}'),
        playable_label,
        ui.playable_only,
        Press::Decks(DecksPress::TogglePlayableOnly),
    );
    commands.entity(strip).add_child(playable_chip);

    let has_filters = !lobby.field(Field::DeckSearch).trim().is_empty()
        || !ui.colors.is_empty()
        || ui.format.is_some()
        || (ui.tab == DecksTab::Mine && ui.favourites_only)
        || ui.playable_only;
    if has_filters {
        let reset_label = if lang == Lang::De {
            "Filter zurücksetzen"
        } else {
            "Reset filters"
        };
        let reset = quick_filter_pill(
            commands,
            kit,
            Some('\u{f00d}'),
            reset_label,
            false,
            Press::Decks(DecksPress::ClearFilters),
        );
        commands.entity(strip).add_child(reset);
    }

    let gap = parts::grow(commands);
    commands.entity(strip).add_child(gap);

    // Collection summary on the right (no "Dein nächstes Spiel" badge)
    if ui.tab == DecksTab::Mine {
        let total = lobby
            .decks()
            .len()
            .saturating_sub(usize::from(lobby.staged_delete().is_some()));
        let shown = filtered_mine_indices(state, prefs).len();
        let summary_text = if shown == total {
            format!("{total} Decks")
        } else if lang == Lang::De {
            format!("{shown} von {total} Decks")
        } else {
            format!("{shown} of {total} decks")
        };
        let summary = parts::line(commands, kit, &summary_text, kit.m.small, tokens::MUTED);
        commands.entity(strip).add_child(summary);
    } else {
        let total = lobby.library().house.len();
        let shown = filtered_house_indices(state).len();
        let summary_text = if shown == total {
            if lang == Lang::De {
                format!("{total} Vorlagen")
            } else {
                format!("{total} templates")
            }
        } else if lang == Lang::De {
            format!("{shown} von {total} Vorlagen")
        } else {
            format!("{shown} of {total} templates")
        };
        let summary = parts::line(commands, kit, &summary_text, kit.m.small, tokens::MUTED);
        commands.entity(strip).add_child(summary);
    }
    format_btn
}

/// Compact dropdown pill with leading icon, label, and trailing caret.
fn format_dropdown_pill(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    label: &str,
    active: bool,
    press: Press,
) -> Entity {
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: kit.m.px(26.0),
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(3.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if active {
                tokens::SELECTED
            } else {
                tokens::CONTROL.with_alpha(0.55)
            }),
            BorderColor::all(if active {
                tokens::ACCENT.with_alpha(0.65)
            } else {
                tokens::BORDER
            }),
            UiTransform::default(),
            crate::ambience::Feel::tinting_to(
                if active {
                    tokens::SELECTED
                } else {
                    tokens::CONTROL.with_alpha(0.55)
                },
                tokens::HOVER,
            ),
            Pickable::IGNORE,
        ))
        .id();
    let lead = commands
        .spawn((
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.85),
            TextColor(if active {
                tokens::ACCENT
            } else {
                tokens::MUTED
            }),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(label),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.92),
            TextColor(if active { tokens::INK } else { tokens::MUTED }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let caret = commands
        .spawn((
            Text::new(parts::CARET.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.8),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[lead, words, caret]);
    let btn = controls::hit(commands, kit, face, press);
    commands
        .entity(btn)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.min_height = kit.m.px(28.0);
        });
    btn
}

/// Compact pill toggle used in the Decks `QoL` filter strip.
fn quick_filter_pill(
    commands: &mut Commands,
    kit: Kit,
    icon: Option<char>,
    label: &str,
    active: bool,
    press: Press,
) -> Entity {
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: kit.m.px(26.0),
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(3.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if active {
                tokens::SELECTED
            } else {
                tokens::CONTROL.with_alpha(0.55)
            }),
            BorderColor::all(if active {
                tokens::ACCENT.with_alpha(0.65)
            } else {
                tokens::BORDER
            }),
            UiTransform::default(),
            crate::ambience::Feel::tinting_to(
                if active {
                    tokens::SELECTED
                } else {
                    tokens::CONTROL.with_alpha(0.55)
                },
                tokens::HOVER,
            ),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(glyph) = icon {
        let g = commands
            .spawn((
                Text::new(glyph.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.85),
                TextColor(if active { tokens::GOLD } else { tokens::MUTED }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_child(g);
    }
    let words = commands
        .spawn((
            Text::new(label),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.92),
            TextColor(if active { tokens::INK } else { tokens::MUTED }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(words);
    let btn = controls::hit(commands, kit, face, press);
    commands
        .entity(btn)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.min_height = kit.m.px(28.0);
        });
    btn
}

/// Computes the ordered & filtered indices for `Meine Decks`.
fn filtered_mine_indices(
    state: &LobbyState,
    prefs: &baylee_client_core::prefs::Preferences,
) -> Vec<usize> {
    let lobby = &state.lobby;
    let ui = &state.decks;
    let format = ui
        .format
        .and_then(|i| DECK_FORMATS.get(usize::from(i)))
        .map(|(f, ..)| *f);
    shelf::order(
        lobby.decks(),
        lobby.field(Field::DeckSearch),
        ui.sort,
        lobby.staged_delete(),
    )
    .into_iter()
    .filter(|&index| {
        let deck = &lobby.decks()[index];
        if !matches_colors(&deck.identity, &ui.colors) {
            return false;
        }
        if let Some(f) = format {
            let matches_primary = deck.format.eq_ignore_ascii_case(f);
            let matches_variant = prefs
                .variants_for_deck(&deck.id)
                .iter()
                .any(|v| v.format.eq_ignore_ascii_case(f));
            if !matches_primary && !matches_variant {
                return false;
            }
        }
        if ui.favourites_only && !prefs.favourite(&deck.id) {
            return false;
        }
        if ui.playable_only && (deck.unplayable > 0 || !deck.blocked.is_empty()) {
            return false;
        }
        true
    })
    .collect()
}

/// Computes the filtered indices for `Vorlagen` (`Haus-Decks`).
fn filtered_house_indices(state: &LobbyState) -> Vec<usize> {
    let lobby = &state.lobby;
    let ui = &state.decks;
    let query = shelf::fold(lobby.field(Field::DeckSearch).trim());
    let format = ui
        .format
        .and_then(|i| DECK_FORMATS.get(usize::from(i)))
        .map(|(f, ..)| *f);
    lobby
        .library()
        .house
        .iter()
        .enumerate()
        .filter(|(_, deck)| {
            if !query.is_empty()
                && !shelf::fold(&deck.name).contains(&query)
                && !deck
                    .commanders
                    .iter()
                    .any(|c| shelf::fold(c).contains(&query))
            {
                return false;
            }
            if format.is_some_and(|f| !deck.format.eq_ignore_ascii_case(f)) {
                return false;
            }
            if !matches_colors(&deck.identity, &ui.colors) {
                return false;
            }
            if ui.playable_only && (deck.unplayable > 0 || !deck.blocked.is_empty()) {
                return false;
            }
            true
        })
        .map(|(i, _)| i)
        .collect()
}

/// Accent ink for a deck's earned achievement rank tier (`0..=11`).
fn rank_tier_ink(tier: u8) -> Color {
    match tier {
        0 => tokens::MUTED,
        1 => Color::srgb(0.76, 0.52, 0.34),  // Bronze
        2 => Color::srgb(0.68, 0.72, 0.78),  // Iron
        3 => Color::srgb(0.84, 0.88, 0.95),  // Silver
        4 => Color::srgb(0.95, 0.78, 0.30),  // Gold
        5 => Color::srgb(0.30, 0.86, 0.56),  // Emerald
        6 => Color::srgb(0.36, 0.68, 0.98),  // Sapphire
        7 => Color::srgb(0.74, 0.44, 0.98),  // Amethyst
        8 => Color::srgb(0.96, 0.34, 0.44),  // Ruby
        9 => Color::srgb(0.45, 0.90, 1.00),  // Diamond
        10 => Color::srgb(1.00, 0.62, 0.24), // Mythic
        _ => Color::srgb(0.95, 0.75, 1.00),  // Aether-Lotus
    }
}

/// The player's own decks: tiles, then the New deck tile; or the empty
/// shelf.
#[allow(clippy::too_many_lines)] // one shelf, read in order
fn mine(
    commands: &mut Commands,
    root: Entity,
    grid: Entity,
    scroll: Entity,
    state: &LobbyState,
    prefs: &baylee_client_core::prefs::Preferences,
    kit: Kit,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let order = filtered_mine_indices(state, prefs);
    let any_filter =
        state.decks.has_active_filters() || !lobby.field(Field::DeckSearch).trim().is_empty();
    if lobby.decks().is_empty() || (order.is_empty() && !any_filter) {
        let empty = empty_shelf(commands, state, kit);
        commands.entity(scroll).add_child(empty);
        return;
    }
    if order.is_empty() {
        let query = lobby.field(Field::DeckSearch).trim();
        let said = if query.is_empty() {
            match lang {
                Lang::De => "Keine Decks entsprechen den aktiven Filtern.".to_string(),
                Lang::En => "No decks match the active filters.".to_string(),
            }
        } else {
            Phrase::DecksNoMatch.fill(lang, &[query])
        };
        let none = parts::line(commands, kit, &said, kit.m.text, tokens::MUTED);
        commands.entity(scroll).add_child(none);
    }
    let now = parts::now_secs();
    let view = state.decks.view;
    let german = lang == Lang::De;
    let mut walked = 0;
    for index in order {
        let deck = &lobby.decks()[index];
        let favourite = prefs.favourite(&deck.id);
        let actions: Vec<Entity> = Vec::new();
        let more = spawn_vertical_more(
            commands,
            kit,
            lang,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(index))),
        );
        let said = shelf::badges(lang, deck, false);
        let blocked_at = (!deck.blocked.is_empty()).then(|| said.len() - 1);
        let mut badges: Vec<(String, Color)> = said
            .into_iter()
            .enumerate()
            .map(|(i, said)| {
                let ink = match i {
                    _ if Some(i) == blocked_at => tokens::DANGER,
                    0 => tokens::ACCENT,
                    _ => tokens::GOLD,
                };
                (said, ink)
            })
            .collect();
        let variants = prefs.variants_for_deck(&deck.id);
        if !variants.is_empty() {
            let v_label = if german {
                format!("+{} Formate", variants.len())
            } else {
                format!("+{} formats", variants.len())
            };
            badges.push((v_label, tokens::ACCENT));
        }
        let stats = prefs.stats_for_deck(&deck.id);
        let rank = stats.rank_tier();
        let rank_tier = rank.level();
        let rank_ink = rank_tier_ink(rank_tier);
        let step_lbl = if german {
            format!("Stufe {rank_tier}/11")
        } else {
            format!("Tier {rank_tier}/11")
        };
        let rank_badge = Some((
            format!("{step_lbl} · {}", rank.title(lang)),
            rank_ink,
            rank.tooltip(stats, lang),
        ));
        let stats_line = if stats.games_played > 0 {
            let rate = stats.win_rate_pct().unwrap_or(0);
            let streak = if stats.best_streak >= 2 {
                if german {
                    format!(" · Serie {}", stats.best_streak)
                } else {
                    format!(" · Streak {}", stats.best_streak)
                }
            } else {
                String::new()
            };
            let text = if german {
                format!(
                    "\u{f091} {} S / {} Sp. ({rate}%){streak}",
                    stats.wins, stats.games_played
                )
            } else {
                format!(
                    "\u{f091} {}W / {}G ({rate}%){streak}",
                    stats.wins, stats.games_played
                )
            };
            Some((
                text,
                if rank_tier >= 4 {
                    tokens::GOLD
                } else {
                    tokens::INK.with_alpha(0.85)
                },
            ))
        } else {
            let text = if german {
                "Noch keine Spiele gespielt".to_string()
            } else {
                "No games played yet".to_string()
            };
            Some((text, tokens::MUTED))
        };
        let subtitle = if deck.commanders.is_empty() {
            deck.signature.as_ref().map(|sig| sig.name.clone())
        } else {
            Some(format!("\u{f521} {}", deck.commanders.join(" & ")))
        };
        let look = TileLook {
            name: &deck.name,
            subtitle,
            identity: &deck.identity,
            art: deck.art(),
            meta: shelf::meta_line(lang, deck, now),
            badges,
            star: favourite,
            star_action: Some(Press::Decks(DecksPress::Favourite(index))),
            blurb: None,
            rank_tier,
            rank_badge,
            stats_line,
        };
        let tile = tile(
            commands,
            kit,
            lang,
            view,
            &look,
            &actions,
            more,
            Some(Press::Decks(DecksPress::Edit(index))),
        );
        orders::item(commands, tile, &orders::DECKS, "tiles", walked);
        if walked == 0 {
            commands
                .entity(tile)
                .insert(TourAnchor(Anchor::DecksFirstTile));
            commands
                .entity(more)
                .insert(TourAnchor(Anchor::DecksTileActions));
        }
        walked += 1;
        commands.entity(tile).insert((
            super::focusing::Primary(Press::Decks(DecksPress::Edit(index))),
            super::focusing::TileOf(index),
            crate::shellkit::focus::Current(false),
        ));
        commands.entity(grid).add_child(tile);
        if state.menu == Some(ShellMenu::Deck(index)) {
            let mut items = vec![
                menus::item(
                    Phrase::ShellEdit.text(lang),
                    Press::Decks(DecksPress::Edit(index)),
                ),
                menus::item(
                    Phrase::DecksDuplicate.text(lang),
                    Press::Decks(DecksPress::Duplicate(index)),
                ),
            ];
            if !lobby.offline() {
                items.push(menus::item(
                    Phrase::DecksHistory.text(lang),
                    Press::Decks(DecksPress::History(index)),
                ));
            }
            items.extend([
                menus::item(
                    if favourite {
                        Phrase::DecksUnfavourite
                    } else {
                        Phrase::DecksFavourite
                    }
                    .text(lang),
                    Press::Decks(DecksPress::Favourite(index)),
                ),
                menus::danger(
                    Phrase::Delete.text(lang),
                    Press::Decks(DecksPress::Delete(index)),
                ),
            ]);
            menus::draw(commands, root, kit, more, items);
        }
    }
    let new = new_tile(commands, kit, lang, view);
    commands.entity(new).insert(TourAnchor(Anchor::DecksNew));
    orders::item(commands, new, &orders::DECKS, "tiles", walked);
    commands.entity(grid).add_child(new);
}

/// Polished empty state when no decks exist yet: hero icon badge, inviting
/// headline and subtitle, primary New Deck & Import actions, and quick-start
/// Vorlagen mini-tiles.
#[allow(clippy::too_many_lines)] // hero badge + title + subtitle + primary actions + Vorlagen mini-tiles
fn empty_shelf(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let german = lang == Lang::De;
    let card = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(kit.m.px(28.0), kit.m.px(32.0)),
                row_gap: kit.m.px(16.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.28)),
            BorderColor::all(tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    let icon_disc = commands
        .spawn((
            Node {
                width: kit.m.px(56.0),
                height: kit.m.px(56.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT.with_alpha(0.14)),
            BorderColor::all(tokens::ACCENT.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(crate::hud::glyph::LIBRARY.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.head * 1.15),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(icon_disc).add_child(icon);

    let title = commands
        .spawn((
            Text::new(Phrase::DecksEmptyTitle.text(lang)),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let sub_text = if german {
        "Erstelle dein erstes eigenes Deck im Deckbuilder, importiere eine bestehende Liste oder starte direkt mit einer fertigen Vorlage."
    } else {
        "Create your first deck in the deckbuilder, import an existing list, or start right away from a template."
    };
    let subtitle = commands
        .spawn((
            Text::new(sub_text),
            crate::hud::tf(kit.fonts, kit.m.text),
            TextColor(tokens::MUTED),
            TextLayout::justify(Justify::Center),
            Node {
                max_width: kit.m.px(540.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let new = controls::button_with_icon(
        commands,
        kit,
        '\u{f067}',
        Phrase::NewDeck.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::NewDeck),
    );
    let import = controls::button_with_icon(
        commands,
        kit,
        '\u{f56f}',
        Phrase::ImportDeck.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::Import),
    );
    let actions = parts::row(commands, kit, true);
    commands.entity(new).insert(TourAnchor(Anchor::DecksNew));
    commands
        .entity(import)
        .insert(TourAnchor(Anchor::DecksImport));
    commands.entity(actions).add_children(&[new, import]);

    let minis = super::play::mini_tiles(commands, state, kit, &orders::DECKS, "tiles");
    commands
        .entity(card)
        .add_children(&[icon_disc, title, subtitle, actions, minis]);
    card
}

/// The New deck tile: clean, prominent card last on the shelf with an accent
/// `+` badge, title, and subtitle.
#[allow(clippy::too_many_lines)] // Gallery/Compact card and List-row layout in one tile constructor
fn new_tile(commands: &mut Commands, kit: Kit, lang: Lang, view: DecksView) -> Entity {
    let is_list = view == DecksView::List;
    let min_h = match view {
        DecksView::Gallery => kit.m.tile,
        DecksView::List => kit.m.scaled(74.0),
        DecksView::Compact => kit.m.scaled(132.0),
    };
    let rest_bg = tokens::CONTROL.with_alpha(0.24);
    let rest_border = tokens::ACCENT.with_alpha(0.38);
    let face = commands
        .spawn((
            Role::Tile,
            Node {
                min_height: px_fixed(min_h),
                flex_direction: if is_list {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: kit.m.px(14.0),
                row_gap: kit.m.px(10.0),
                padding: UiRect::all(kit.m.px(18.0)),
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BorderColor::all(rest_border),
            BackgroundColor(rest_bg),
            UiTransform::default(),
            Press::Decks(DecksPress::NewDeck),
            crate::ambience::Feel::new(rest_bg),
            crate::ambience::FeelBorder {
                rest: rest_border,
                hot: tokens::ACCENT,
                press: tokens::ACCENT,
            },
        ))
        .id();
    let plus_disc = commands
        .spawn((
            Node {
                width: kit.m.px(if is_list { 36.0 } else { 44.0 }),
                height: kit.m.px(if is_list { 36.0 } else { 44.0 }),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::ACCENT.with_alpha(0.16)),
            BorderColor::all(tokens::ACCENT.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .id();
    let plus_glyph = commands
        .spawn((
            Text::new("\u{f067}"),
            crate::hud::icon_tf(kit.fonts, kit.m.text * 1.1),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(plus_disc).add_child(plus_glyph);

    let text_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: if is_list {
                    AlignItems::Start
                } else {
                    AlignItems::Center
                },
                row_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(Phrase::NewDeck.text(lang)),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.08),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let hint = parts::line(
        commands,
        kit,
        if kit.m.touch() || cfg!(target_arch = "wasm32") {
            Phrase::DecksNewTileTouch.text(lang)
        } else {
            Phrase::DecksNewTileHint.text(lang)
        },
        kit.m.small,
        tokens::MUTED,
    );
    if !is_list {
        commands
            .entity(hint)
            .insert(TextLayout::justify(Justify::Center));
    }
    commands.entity(text_col).add_children(&[title, hint]);
    commands.entity(face).add_children(&[plus_disc, text_col]);
    face
}

/// The Vorlagen (house decks) shelf: tiles with artwork, commander/signature,
/// blurb, **Ansehen** (`Preview`) and **Hinzufügen** (`Add`). No `AddAndUse`
/// and no `HouseHistory`.
#[allow(clippy::too_many_lines)] // one shelf, read in order
fn house(
    commands: &mut Commands,
    root: Entity,
    grid: Entity,
    scroll: Entity,
    state: &LobbyState,
    kit: Kit,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let lib = lobby.library();
    if lib.house.is_empty() {
        let said = if lib.loading {
            Phrase::LibraryLoading.text(lang)
        } else {
            Phrase::LibraryEmpty.text(lang)
        };
        if lib.loading {
            let skeleton = crate::shellkit::states::skeleton(commands, kit, 3);
            commands.entity(scroll).add_child(skeleton);
        } else if lib.error.is_none() {
            let line = parts::line(commands, kit, said, kit.m.text, tokens::MUTED);
            commands.entity(scroll).add_child(line);
        }
        return;
    }
    let order = filtered_house_indices(state);
    if order.is_empty() {
        let query = lobby.field(Field::DeckSearch).trim();
        let said = if query.is_empty() {
            match lang {
                Lang::De => "Keine Vorlagen entsprechen den aktiven Filtern.".to_string(),
                Lang::En => "No templates match the active filters.".to_string(),
            }
        } else {
            Phrase::DecksNoMatch.fill(lang, &[query])
        };
        let none = parts::line(commands, kit, &said, kit.m.text, tokens::MUTED);
        commands.entity(scroll).add_child(none);
        return;
    }
    let view = state.decks.view;
    let tiny = kit.m.frame == ShellFrame::Phone;
    for (walked, index) in order.into_iter().enumerate() {
        let deck = &lib.house[index];
        let mut actions = Vec::new();
        if !tiny {
            actions.push(controls::button_compact(
                commands,
                kit,
                Some('\u{f06e}'),
                Phrase::HousePreview.text(lang),
                Weight::Secondary,
                Live::Yes,
                Press::Decks(DecksPress::Preview(index)),
            ));
        }
        let add = controls::button_compact(
            commands,
            kit,
            Some('\u{f067}'),
            Phrase::HouseAdd.text(lang),
            Weight::Primary,
            if lib.loading {
                Live::No(Phrase::LibraryLoading.text(lang))
            } else {
                Live::Yes
            },
            Press::Decks(DecksPress::Add(index)),
        );
        actions.push(add);
        let more = spawn_vertical_more(
            commands,
            kit,
            lang,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::House(index))),
        );
        let copies = if deck.copies > 0 {
            deck.copies.to_string()
        } else {
            deck.cards.to_string()
        };
        let mut badges = vec![(shelf::format_label(lang, &deck.format), tokens::ACCENT)];
        if deck.unplayable > 0 {
            badges.push((
                Phrase::DecksUnplayable.fill_count(
                    lang,
                    usize::try_from(deck.unplayable).unwrap_or(usize::MAX),
                    &[],
                ),
                tokens::GOLD,
            ));
        }
        if let Some(said) = shelf::blocked_badge(lang, &deck.blocked) {
            badges.push((said, tokens::DANGER));
        }
        let subtitle = if deck.commanders.is_empty() {
            deck.signature.as_ref().map(|sig| sig.name.clone())
        } else {
            Some(format!("\u{f521} {}", deck.commanders.join(" & ")))
        };
        let look = TileLook {
            name: &deck.name,
            subtitle,
            identity: &deck.identity,
            art: deck.art(),
            meta: Phrase::DeckMeta.fill(lang, &[&copies, &deck.sideboard.to_string()]),
            badges,
            star: false,
            star_action: None,
            blurb: (!deck.description.is_empty()).then_some(deck.description.as_str()),
            rank_tier: 0,
            rank_badge: None,
            stats_line: None,
        };
        let tile = tile(
            commands,
            kit,
            lang,
            view,
            &look,
            &actions,
            more,
            Some(Press::Decks(DecksPress::Preview(index))),
        );
        orders::item(commands, tile, &orders::DECKS, "tiles", walked);
        commands
            .entity(tile)
            .insert(super::focusing::Primary(Press::Decks(DecksPress::Preview(
                index,
            ))));
        commands.entity(grid).add_child(tile);
        if state.menu == Some(ShellMenu::House(index)) {
            let items = vec![
                menus::item(
                    Phrase::HousePreview.text(lang),
                    Press::Decks(DecksPress::Preview(index)),
                ),
                menus::item(
                    Phrase::HouseAdd.text(lang),
                    Press::Decks(DecksPress::Add(index)),
                ),
            ];
            menus::draw(commands, root, kit, more, items);
        }
    }
}

/// What a tile shows.
pub(super) struct TileLook<'a> {
    pub(super) name: &'a str,
    pub(super) subtitle: Option<String>,
    pub(super) identity: &'a str,
    pub(super) art: Option<shelf::DeckArt>,
    pub(super) meta: String,
    pub(super) badges: Vec<(String, Color)>,
    pub(super) star: bool,
    pub(super) star_action: Option<Press>,
    pub(super) blurb: Option<&'a str>,
    /// Achievement rank tier (`0..=11`) earned by playing and winning with this deck.
    pub(super) rank_tier: u8,
    /// Optional `(badge_text, color, tooltip)` for the deck's achievement rank.
    pub(super) rank_badge: Option<(String, Color, String)>,
    /// Optional `(stats_text, color)` summarizing Central play/win statistics.
    pub(super) stats_line: Option<(String, Color)>,
}

/// Spawns an interactive favourite star button when `star_action` is present,
/// or a static star glyph when `star` is true.
fn spawn_star(commands: &mut Commands, kit: Kit, lang: Lang, look: &TileLook) -> Option<Entity> {
    if let Some(press) = look.star_action {
        let tip = if look.star {
            Phrase::DecksUnfavourite.text(lang)
        } else {
            Phrase::DecksFavourite.text(lang)
        };
        let btn = commands
            .spawn((
                Role::Button,
                Node {
                    width: kit.m.px(28.0),
                    height: kit.m.px(28.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(if look.star {
                    tokens::GOLD.with_alpha(0.14)
                } else {
                    Color::NONE
                }),
                UiTransform::default(),
                crate::ambience::Feel::rising_to(
                    if look.star {
                        tokens::GOLD.with_alpha(0.14)
                    } else {
                        Color::NONE
                    },
                    tokens::HOVER,
                ),
                super::hint::HoverHint(tip.to_string()),
                press,
            ))
            .id();
        let icon = commands
            .spawn((
                Text::new(parts::STAR.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.text * 0.95),
                TextColor(if look.star {
                    tokens::GOLD
                } else {
                    tokens::MUTED.with_alpha(0.55)
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(btn).add_child(icon);
        Some(btn)
    } else if look.star {
        Some(
            commands
                .spawn((
                    Text::new(parts::STAR.to_string()),
                    crate::hud::icon_tf(kit.fonts, kit.m.text),
                    TextColor(tokens::GOLD),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id(),
        )
    } else {
        None
    }
}

/// Spawns a compact 28×28 vertical `⋮` menu button placed right beside the
/// favourite star in the top-right header of a deck or template tile.
fn spawn_vertical_more(commands: &mut Commands, kit: Kit, lang: Lang, press: Press) -> Entity {
    let tip = match lang {
        Lang::De => "Weitere Aktionen",
        Lang::En => "More actions",
    };
    let btn = commands
        .spawn((
            Role::Button,
            Node {
                width: kit.m.px(28.0),
                height: kit.m.px(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            super::hint::HoverHint(tip.to_string()),
            press,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(parts::ELLIPSIS.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text * 0.92),
            TextColor(tokens::MUTED.with_alpha(0.85)),
            UiTransform {
                rotation: Rot2::radians(std::f32::consts::FRAC_PI_2),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(btn).add_child(icon);
    btn
}

/// Spawns a badge with an optional hover tooltip (`HoverHint`) for the deck's achievement rank.
fn spawn_rank_badge(commands: &mut Commands, kit: Kit, look: &TileLook) -> Option<Entity> {
    let (label, ink, tooltip) = look.rank_badge.as_ref()?;
    let pill = parts::badge_with(commands, kit, '\u{f091}', label, *ink);
    commands
        .entity(pill)
        .insert((Pickable::default(), super::hint::HoverHint(tooltip.clone())));
    Some(pill)
}

/// Configures hover/press feel, golden favourite border, click-to-open action,
/// and shader-animated achievement frame on a deck tile's root entity.
fn decorate_tile_card(commands: &mut Commands, card: Entity, look: &TileLook, open: Option<Press>) {
    let (rest_border, hover_border) = if look.star {
        (tokens::GOLD.with_alpha(0.72), tokens::GOLD)
    } else if look.rank_tier >= 4 {
        let ink = rank_tier_ink(look.rank_tier);
        (ink.with_alpha(0.50), ink.with_alpha(0.92))
    } else {
        (tokens::BORDER, tokens::ACCENT.with_alpha(0.78))
    };
    commands.entity(card).insert((
        BorderColor::all(rest_border),
        UiTransform::default(),
        crate::ambience::Feel::new(tokens::PANEL),
        crate::ambience::FeelBorder {
            rest: rest_border,
            hot: hover_border,
            press: tokens::ACCENT,
        },
    ));
    if let Some(press) = open {
        commands.entity(card).insert(press);
    }
    super::deck_frame::attach(commands, card, card, look.rank_tier, look.star);
}

/// A deck tile (§2.4 Tile), rendered in the active `DecksView` (`Gallery`,
/// `List`, or `Compact`).
#[allow(clippy::too_many_arguments)]
pub(super) fn tile(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    view: DecksView,
    look: &TileLook,
    actions: &[Entity],
    more: Entity,
    open: Option<Press>,
) -> Entity {
    match view {
        DecksView::Gallery => tile_gallery(commands, kit, lang, look, actions, more, open),
        DecksView::List => tile_list(commands, kit, lang, look, actions, more, open),
        DecksView::Compact => tile_compact(commands, kit, lang, look, actions, more, open),
    }
}

/// Gallery view tile: balanced header band with artwork + artist credit, title + star,
/// commander/signature subtitle, mana identity discs, stats & meta row, achievement
/// rank badge, format badges, and footer actions.
#[allow(clippy::too_many_lines)]
fn tile_gallery(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    look: &TileLook,
    actions: &[Entity],
    more: Entity,
    open: Option<Press>,
) -> Entity {
    let phone = kit.m.frame == ShellFrame::Phone;
    let border_px = if look.star || look.rank_tier >= 4 {
        1.5
    } else {
        1.0
    };
    let card = commands
        .spawn((
            Role::Tile,
            Node {
                flex_direction: FlexDirection::Column,
                min_height: px_fixed(kit.m.tile),
                min_width: px(0),
                border: UiRect::all(px_fixed(border_px)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
        ))
        .id();
    decorate_tile_card(commands, card, look, open);

    let pad = kit.m.scaled(if phone { 8.0 } else { 12.0 });
    let band = commands
        .spawn((
            Node {
                column_gap: kit.m.px(12.0),
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(px_fixed(pad)),
                border: UiRect::bottom(px_fixed(1.0)),
                border_radius: BorderRadius::top(px_fixed(tokens::RADIUS_PANEL - 1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER.with_alpha(0.55)),
            parts::identity_gradient(look.identity),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(art) = &look.art {
        let picture = parts::art_with_credit(commands, kit, lang, art, 90.0);
        commands.entity(band).add_child(picture);
    }
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(5.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title_row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(look.name),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.12),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(title_row).add_child(name);
    let tools = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(2.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(star) = spawn_star(commands, kit, lang, look) {
        commands.entity(tools).add_child(star);
    }
    commands.entity(tools).add_child(more);
    commands.entity(title_row).add_child(tools);
    commands.entity(words).add_child(title_row);

    if let Some(sub) = &look.subtitle {
        let sub_text = commands
            .spawn((
                Text::new(shortened(sub, 44)),
                crate::hud::tf(kit.fonts, kit.m.small * 0.96),
                TextColor(tokens::INK.with_alpha(0.82)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(words).add_child(sub_text);
    }
    if !phone && !look.identity.is_empty() {
        let discs = parts::identity_discs(commands, kit, look.identity);
        commands.entity(words).add_child(discs);
    }
    commands.entity(band).add_child(words);

    let inside = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: kit.m.px(if phone { 5.0 } else { 8.0 }),
                padding: UiRect::all(px_fixed(pad)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(blurb) = look.blurb {
        let said = commands
            .spawn((
                Text::new(shortened(blurb, 84)),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(inside).add_child(said);
    }

    // Meta + Central Statistics line
    let meta_row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(3.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let meta = parts::line(commands, kit, &look.meta, kit.m.small, tokens::MUTED);
    commands.entity(meta_row).add_child(meta);
    if let Some((stats_text, stats_ink)) = &look.stats_line {
        let stat_lbl = parts::line(commands, kit, stats_text, kit.m.small * 0.94, *stats_ink);
        commands.entity(meta_row).add_child(stat_lbl);
    }
    commands.entity(inside).add_child(meta_row);

    let badges = parts::row(commands, kit, true);
    if phone && !look.identity.is_empty() {
        let letters = parts::badge(commands, kit, look.identity, tokens::INK);
        commands.entity(badges).add_child(letters);
    }
    for (said, ink) in &look.badges {
        let b = parts::badge(commands, kit, said, *ink);
        commands.entity(badges).add_child(b);
    }
    if let Some(rank_pill) = spawn_rank_badge(commands, kit, look) {
        commands.entity(badges).add_child(rank_pill);
    }
    commands.entity(inside).add_child(badges);

    if !actions.is_empty() {
        let row = commands
            .spawn((
                Node {
                    margin: UiRect::top(Val::Auto),
                    padding: UiRect::top(kit.m.px(4.0)),
                    column_gap: kit.m.px(6.0),
                    row_gap: kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::NoWrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_children(actions);
        commands.entity(inside).add_child(row);
    }
    commands.entity(card).add_children(&[band, inside]);
    card
}

/// List view tile: sleek horizontal row with artwork thumbnail on the left,
/// deck name + commander/blurb + identity discs in the middle, badges & stats,
/// and action buttons on the right.
#[allow(clippy::too_many_lines)]
fn tile_list(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    look: &TileLook,
    actions: &[Entity],
    more: Entity,
    open: Option<Press>,
) -> Entity {
    let phone = kit.m.frame == ShellFrame::Phone;
    let border_px = if look.star || look.rank_tier >= 4 {
        1.5
    } else {
        1.0
    };
    let card = commands
        .spawn((
            Role::Tile,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(14.0),
                row_gap: kit.m.px(8.0),
                flex_wrap: if phone {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                padding: UiRect::axes(kit.m.px(14.0), kit.m.px(10.0)),
                border: UiRect::all(px_fixed(border_px)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            parts::identity_gradient(look.identity),
        ))
        .id();
    decorate_tile_card(commands, card, look, open);

    let lead = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(12.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: kit.m.px(200.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(art) = &look.art {
        let picture = parts::art_with_credit(commands, kit, lang, art, 74.0);
        commands.entity(lead).add_child(picture);
    }
    let info = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(look.name),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.08),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(title_row).add_child(name);
    if !look.identity.is_empty() {
        let discs = parts::identity_discs(commands, kit, look.identity);
        commands.entity(title_row).add_child(discs);
    }
    for (said, ink) in &look.badges {
        let b = parts::badge(commands, kit, said, *ink);
        commands.entity(title_row).add_child(b);
    }
    if let Some(rank_pill) = spawn_rank_badge(commands, kit, look) {
        commands.entity(title_row).add_child(rank_pill);
    }
    commands.entity(info).add_child(title_row);

    let mut sub_parts = Vec::new();
    if let Some(sub) = &look.subtitle {
        sub_parts.push(shortened(sub, 48));
    }
    sub_parts.push(look.meta.clone());
    if let Some((stats_text, _)) = &look.stats_line {
        sub_parts.push(stats_text.clone());
    }
    if let Some(blurb) = look.blurb {
        sub_parts.push(shortened(blurb, 64));
    }
    let meta = parts::line(
        commands,
        kit,
        &sub_parts.join("  ·  "),
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(info).add_child(meta);
    commands.entity(lead).add_child(info);
    commands.entity(card).add_child(lead);

    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(actions);
    if let Some(star) = spawn_star(commands, kit, lang, look) {
        commands.entity(row).add_child(star);
    }
    commands.entity(row).add_child(more);
    commands.entity(card).add_child(row);
    card
}

/// Compact view tile: dense mini-card for large deck collections, showing
/// identity gradient, artwork thumbnail, deck name, badges, stats, and actions
/// in a tight footprint.
#[allow(clippy::too_many_lines)]
fn tile_compact(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    look: &TileLook,
    actions: &[Entity],
    more: Entity,
    open: Option<Press>,
) -> Entity {
    let border_px = if look.star || look.rank_tier >= 4 {
        1.5
    } else {
        1.0
    };
    let card = commands
        .spawn((
            Role::Tile,
            Node {
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                row_gap: kit.m.px(6.0),
                padding: UiRect::all(kit.m.px(10.0)),
                border: UiRect::all(px_fixed(border_px)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            parts::identity_gradient(look.identity),
        ))
        .id();
    decorate_tile_card(commands, card, look, open);

    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Start,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(art) = &look.art {
        let picture = parts::art_with_credit(commands, kit, lang, art, 58.0);
        commands.entity(head).add_child(picture);
    }
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = commands
        .spawn((
            Text::new(shortened(look.name, 28)),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(words).add_child(name);
    if !look.identity.is_empty() {
        let discs = parts::identity_discs(commands, kit, look.identity);
        commands.entity(words).add_child(discs);
    }
    let meta = parts::line(
        commands,
        kit,
        &shortened(&look.meta, 32),
        kit.m.small * 0.94,
        tokens::MUTED,
    );
    commands.entity(words).add_child(meta);
    if let Some((stats_text, stats_ink)) = &look.stats_line {
        let stat_lbl = parts::line(
            commands,
            kit,
            &shortened(stats_text, 28),
            kit.m.small * 0.90,
            *stats_ink,
        );
        commands.entity(words).add_child(stat_lbl);
    }
    commands.entity(head).add_child(words);
    let tools = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(2.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(star) = spawn_star(commands, kit, lang, look) {
        commands.entity(tools).add_child(star);
    }
    commands.entity(tools).add_child(more);
    commands.entity(head).add_child(tools);
    commands.entity(card).add_child(head);

    let badges = parts::row(commands, kit, true);
    for (said, ink) in &look.badges {
        let b = parts::badge(commands, kit, said, *ink);
        commands.entity(badges).add_child(b);
    }
    if let Some(rank_pill) = spawn_rank_badge(commands, kit, look) {
        commands.entity(badges).add_child(rank_pill);
    }
    commands.entity(card).add_child(badges);

    if !actions.is_empty() {
        let row = commands
            .spawn((
                Node {
                    margin: UiRect::top(Val::Auto),
                    column_gap: kit.m.px(6.0),
                    row_gap: kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_children(actions);
        commands.entity(card).add_child(row);
    }
    card
}

/// `text` cut to at most `most` characters at a word, with an ellipsis
/// when anything was cut.
fn shortened(text: &str, most: usize) -> String {
    if text.chars().count() <= most {
        return text.to_string();
    }
    let cut: String = text.chars().take(most).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!(
        "{}\u{2026}",
        cut[..at].trim_end_matches([',', ';', ':', '.', ' '])
    )
}

/// Looks up a card in the compiled pool by its English or localized name.
fn lookup_pool_card(name: &str) -> Option<&'static baylee_cards::pool::PoolCard> {
    let trimmed = name.trim();
    if let Some(idx) = baylee_cards::decks::by_name(trimmed) {
        let raw = idx.get();
        let rows = baylee_cards::pool::rows();
        if let Ok(pos) = rows.binary_search_by_key(&raw, |c| c.index) {
            return rows.get(pos);
        }
    }
    let front = trimmed.split(" // ").next().unwrap_or(trimmed).trim();
    baylee_cards::pool::rows().iter().find(|c| {
        c.english_name.eq_ignore_ascii_case(trimmed)
            || c.name.eq_ignore_ascii_case(trimmed)
            || c.english_name.eq_ignore_ascii_case(front)
            || c.name.eq_ignore_ascii_case(front)
    })
}

/// Colour of the rarity dot for a `Rarity` value.
pub(crate) fn rarity_dot_color(rarity: baylee_client_core::deckbuilder::Rarity) -> Color {
    match rarity {
        baylee_client_core::deckbuilder::Rarity::Common => Color::srgb(0.60, 0.64, 0.70),
        baylee_client_core::deckbuilder::Rarity::Uncommon => Color::srgb(0.58, 0.76, 0.90),
        baylee_client_core::deckbuilder::Rarity::Rare => Color::srgb(0.95, 0.78, 0.30),
        baylee_client_core::deckbuilder::Rarity::Mythic => Color::srgb(0.96, 0.42, 0.22),
        baylee_client_core::deckbuilder::Rarity::Special
        | baylee_client_core::deckbuilder::Rarity::Bonus => Color::srgb(0.76, 0.52, 0.96),
    }
}

/// Returns a card's verified reference rarity for display when `baylee_cards::pool::PoolCard` is on hand.
fn preview_card_rarity(
    card: &baylee_cards::pool::PoolCard,
) -> baylee_client_core::deckbuilder::Rarity {
    baylee_cards::sides::rarity(baylee_core::ids::CardIndex::new(card.index))
        .unwrap_or(baylee_client_core::deckbuilder::Rarity::Common)
}

/// Categorizes a card's `kinds` into a display section key (`0..=6`).
fn preview_type_section(kinds: &[&str], lang: Lang) -> (u8, &'static str) {
    let de = lang == Lang::De;
    if kinds.contains(&"Creature") {
        (1, if de { "Kreaturen" } else { "Creatures" })
    } else if kinds.contains(&"Planeswalker") {
        (2, "Planeswalker")
    } else if kinds.contains(&"Instant") || kinds.contains(&"Sorcery") {
        (3, if de { "Zaubersprüche" } else { "Spells" })
    } else if kinds.contains(&"Artifact") {
        (4, if de { "Artefakte" } else { "Artifacts" })
    } else if kinds.contains(&"Enchantment") || kinds.contains(&"Battle") {
        (5, if de { "Verzauberungen" } else { "Enchantments" })
    } else if kinds.contains(&"Land") {
        (6, if de { "Länder" } else { "Lands" })
    } else {
        (5, if de { "Weitere Karten" } else { "Other cards" })
    }
}

/// Builds a `HoverCard` for a compiled `baylee_cards::pool::PoolCard`, honouring
/// any explicit printing (`scryfall_id`, `lang`) and `finish` (`Foil`, `Etched`, etc.)
/// specified by the deck row.
fn hover_for_pool_card(
    card: &baylee_cards::pool::PoolCard,
    print: Option<&baylee_core::deckrow::PrintChoice>,
) -> super::preview::HoverCard {
    let scryfall_id = print
        .and_then(|p| p.scryfall_id.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| card.scryfall_id.to_owned());
    let lang = print
        .and_then(|p| p.lang.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "en".to_owned());
    let core_finish = print
        .and_then(|p| p.finish)
        .unwrap_or(baylee_core::preset::Finish::Normal);
    let view_finish = match core_finish {
        baylee_core::preset::Finish::Normal => baylee_view::Finish::Normal,
        baylee_core::preset::Finish::Foil => baylee_view::Finish::Foil,
        baylee_core::preset::Finish::Etched => baylee_view::Finish::Etched,
        baylee_core::preset::Finish::Holographic => baylee_view::Finish::Holographic,
        baylee_core::preset::Finish::Glitter => baylee_view::Finish::Glitter,
        baylee_core::preset::Finish::Galaxy => baylee_view::Finish::Galaxy,
    };
    let entry = baylee_view::PrintEntry {
        scryfall_id: scryfall_id.clone(),
        lang,
        finish: view_finish,
    };
    let url = (!scryfall_id.is_empty())
        .then(|| {
            baylee_client_core::images::image_url(
                &entry,
                baylee_client_core::images::Face::Front,
                baylee_client_core::images::ArtSize::Normal,
            )
        })
        .flatten();
    let back_url = (card.has_back_image && !scryfall_id.is_empty())
        .then(|| {
            baylee_client_core::images::image_url(
                &entry,
                baylee_client_core::images::Face::Back,
                baylee_client_core::images::ArtSize::Normal,
            )
        })
        .flatten();
    super::preview::HoverCard {
        url,
        back_url,
        finish: baylee_client_core::images::FinishTreatment::from(view_finish),
        index: Some(card.index),
    }
}

/// Renders one card entry inside the `Vorlagen -> Ansehen` preview sheet.
#[allow(clippy::too_many_lines)] // Gallery card, Compact card, and List row modes for a preview card
fn spawn_preview_card_entry(
    commands: &mut Commands,
    kit: Kit,
    row_line: &str,
    view: DecksView,
) -> Entity {
    let parsed = baylee_core::deckrow::parse(row_line).ok();
    let count = parsed.as_ref().map_or(1, |r| r.count);
    let card_name = parsed
        .as_ref()
        .map_or_else(|| row_line.trim().to_string(), |r| r.name.clone());
    let print = parsed.as_ref().map(|r| &r.print);
    let pool_card = lookup_pool_card(&card_name);
    let hover = pool_card.map(|pc| hover_for_pool_card(pc, print));

    match view {
        DecksView::Gallery => {
            let tile = commands
                .spawn((
                    Node {
                        width: kit.m.px(136.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: kit.m.px(4.0),
                        padding: UiRect::all(kit.m.px(6.0)),
                        border: UiRect::all(px_fixed(1.0)),
                        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                        ..default()
                    },
                    BackgroundColor(tokens::CONTROL.with_alpha(0.45)),
                    BorderColor::all(tokens::BORDER),
                    Pickable::default(),
                ))
                .id();
            if let Some(h) = &hover {
                let pic = super::thumbnails::spawn_sized(
                    commands,
                    h,
                    kit.m.px(124.0),
                    kit.m.px(173.0),
                    px_fixed(6.0),
                );
                commands.entity(tile).insert(h.clone()).add_child(pic);
            }
            let foot = commands
                .spawn((
                    Node {
                        width: percent(100),
                        align_items: AlignItems::Center,
                        column_gap: kit.m.px(5.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            if let Some(pc) = pool_card {
                let dot = commands
                    .spawn((
                        Node {
                            width: kit.m.px(7.0),
                            height: kit.m.px(7.0),
                            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(rarity_dot_color(preview_card_rarity(pc))),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(foot).add_child(dot);
            }
            let lbl = commands
                .spawn((
                    Text::new(format!("{count}× {}", shortened(&card_name, 16))),
                    crate::hud::tf_bold(kit.fonts, kit.m.small * 0.90),
                    TextColor(tokens::INK),
                    TextLayout::no_wrap(),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(foot).add_child(lbl);
            commands.entity(tile).add_child(foot);
            tile
        }
        DecksView::List | DecksView::Compact => {
            let compact = view == DecksView::Compact;
            let item = commands
                .spawn((
                    Node {
                        width: percent(100),
                        min_height: kit.m.px(if compact { 28.0 } else { 38.0 }),
                        padding: UiRect::axes(
                            kit.m.px(10.0),
                            kit.m.px(if compact { 3.0 } else { 5.0 }),
                        ),
                        column_gap: kit.m.px(8.0),
                        align_items: AlignItems::Center,
                        border: UiRect::all(px_fixed(1.0)),
                        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                        ..default()
                    },
                    BackgroundColor(tokens::CONTROL.with_alpha(0.38)),
                    BorderColor::all(tokens::BORDER.with_alpha(0.65)),
                    Pickable::default(),
                ))
                .id();
            if let Some(h) = &hover {
                commands.entity(item).insert(h.clone());
                if !compact {
                    let thumb = super::thumbnails::spawn_sized(
                        commands,
                        h,
                        kit.m.px(26.0),
                        kit.m.px(36.0),
                        px_fixed(4.0),
                    );
                    commands.entity(item).add_child(thumb);
                }
            }
            let qty = commands
                .spawn((
                    Text::new(format!("{count}×")),
                    crate::hud::tf_bold(kit.fonts, kit.m.small),
                    TextColor(tokens::ACCENT),
                    Node {
                        min_width: kit.m.px(24.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(item).add_child(qty);
            if let Some(pc) = pool_card {
                let dot = commands
                    .spawn((
                        Node {
                            width: kit.m.px(8.0),
                            height: kit.m.px(8.0),
                            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(rarity_dot_color(preview_card_rarity(pc))),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(item).add_child(dot);
            }
            let name_node = commands
                .spawn((
                    Text::new(card_name),
                    crate::hud::tf_bold(kit.fonts, kit.m.small * 0.98),
                    TextColor(tokens::INK),
                    Node {
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        min_width: px(0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(item).add_child(name_node);
            if let Some(pc) = pool_card {
                if !compact && !pc.type_line.is_empty() {
                    let t_lbl = commands
                        .spawn((
                            Text::new(shortened(&pc.type_line, 26)),
                            crate::hud::tf(kit.fonts, kit.m.small * 0.88),
                            TextColor(tokens::MUTED),
                            TextLayout::no_wrap(),
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(item).add_child(t_lbl);
                }
                if !pc.mana_cost.is_empty()
                    && let Some(cost) = crate::manaui::spawn_cost(
                        commands,
                        kit.fonts,
                        &pc.mana_cost,
                        kit.m.scaled(14.0).max(13.0),
                    )
                {
                    commands.entity(item).add_child(cost);
                }
            }
            item
        }
    }
}

/// Spawns a group of preview card entries (either in a vertical list, a 2-column
/// compact grid, or a wrapping gallery grid).
fn spawn_preview_section_container(
    commands: &mut Commands,
    kit: Kit,
    rows: &[String],
    view: DecksView,
) -> Entity {
    let container = match view {
        DecksView::Gallery => commands
            .spawn((
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: kit.m.px(8.0),
                    row_gap: kit.m.px(8.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id(),
        DecksView::Compact => commands
            .spawn((
                Node {
                    display: Display::Grid,
                    width: percent(100),
                    grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.0)],
                    column_gap: kit.m.px(8.0),
                    row_gap: kit.m.px(5.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id(),
        DecksView::List => commands
            .spawn((
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(5.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id(),
    };
    for line in rows {
        let entry = spawn_preview_card_entry(commands, kit, line, view);
        commands.entity(container).add_child(entry);
    }
    container
}

/// The Vorlagen (`Haus-Decks`) preview sheet (`Ansehen`): header summary card,
/// type-grouping & view-mode toolbar, rich card entries with mana pips, rarity dots,
/// thumbnails & hover previews, and a proper scrollbar (`sheet_box_listed`).
#[allow(clippy::too_many_lines)]
fn preview_sheet(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let german = lang == Lang::De;
    let lib = lobby.library();
    let Some(index) = state.decks.preview else {
        return;
    };
    let Some(deck) = lib.house.get(index) else {
        return;
    };
    let mut body = Vec::new();

    // 1. Summary header card inside the sheet
    let hero = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: kit.m.px(14.0),
                padding: UiRect::all(kit.m.px(12.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            parts::identity_gradient(&deck.identity),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(art) = deck.art() {
        let pic = parts::art_with_credit(commands, kit, lang, &art, 96.0);
        if let Some(cmd_name) = deck
            .commanders
            .first()
            .map(String::as_str)
            .or_else(|| deck.signature.as_ref().map(|s| s.name.as_str()))
            && let Some(pc) = lookup_pool_card(cmd_name)
        {
            commands
                .entity(pic)
                .insert((Pickable::default(), hover_for_pool_card(pc, None)));
        }
        commands.entity(hero).add_child(pic);
    }
    let hero_info = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if !deck.commanders.is_empty() {
        let cmd_lbl = parts::line(
            commands,
            kit,
            &format!("\u{f521} {}", deck.commanders.join(" & ")),
            kit.m.text,
            tokens::GOLD,
        );
        commands.entity(hero_info).add_child(cmd_lbl);
    } else if let Some(sig) = &deck.signature {
        let sig_lbl = parts::line(
            commands,
            kit,
            &sig.name,
            kit.m.small,
            tokens::INK.with_alpha(0.85),
        );
        commands.entity(hero_info).add_child(sig_lbl);
    }
    let meta_badges = parts::row(commands, kit, true);
    if !deck.identity.is_empty() {
        let discs = parts::identity_discs(commands, kit, &deck.identity);
        commands.entity(meta_badges).add_child(discs);
    }
    let fmt_badge = parts::badge(
        commands,
        kit,
        &shelf::format_label(lang, &deck.format),
        tokens::ACCENT,
    );
    commands.entity(meta_badges).add_child(fmt_badge);
    let copies = if deck.copies > 0 {
        deck.copies
    } else {
        u32::try_from(deck.cards).unwrap_or(0)
    };
    let count_badge = parts::badge(
        commands,
        kit,
        &Phrase::DeckMeta.fill(lang, &[&copies.to_string(), &deck.sideboard.to_string()]),
        tokens::INK,
    );
    commands.entity(meta_badges).add_child(count_badge);
    commands.entity(hero_info).add_child(meta_badges);

    if !deck.description.is_empty() {
        let desc = parts::line(commands, kit, &deck.description, kit.m.small, tokens::MUTED);
        commands.entity(hero_info).add_child(desc);
    }
    commands.entity(hero).add_child(hero_info);
    body.push(hero);

    // 2. Preview sheet toolbar: Type grouping toggle + 3-option view switcher
    let bar = parts::row(commands, kit, true);
    commands.entity(bar).entry::<Node>().and_modify(|mut n| {
        n.width = percent(100);
        n.justify_content = JustifyContent::SpaceBetween;
    });
    let grouped = !state.decks.preview_ungrouped;
    let group_pill = quick_filter_pill(
        commands,
        kit,
        Some('\u{f0ca}'),
        if german {
            "Nach Typ gruppieren"
        } else {
            "Group by type"
        },
        grouped,
        Press::Decks(DecksPress::TogglePreviewGrouped),
    );
    let modes = [DecksView::List, DecksView::Gallery, DecksView::Compact];
    let p_view = state.decks.preview_view;
    let active_v = modes.iter().position(|&m| m == p_view).unwrap_or(0);
    let view_seg = controls::segmented_icons(
        commands,
        kit,
        &[
            (
                '\u{f03a}',
                if german {
                    "Listen-Ansicht"
                } else {
                    "List view"
                },
            ),
            (
                '\u{f009}',
                if german { "Karten-Raster" } else { "Card grid" },
            ),
            (
                '\u{f00a}',
                if german {
                    "Kompakt-Ansicht"
                } else {
                    "Compact view"
                },
            ),
        ],
        active_v,
        |i| Press::Decks(DecksPress::SetPreviewView(modes[i])),
    );
    commands.entity(bar).add_children(&[group_pill, view_seg]);
    body.push(bar);

    // 3. Card sections
    match &lib.preview {
        Some((id, snapshot)) if *id == deck.id => {
            if !snapshot.commanders.is_empty() {
                body.push(parts::caption(
                    commands,
                    kit,
                    &format!(
                        "{} ({})",
                        Phrase::LibraryCommanders.text(lang),
                        snapshot.commanders.len()
                    ),
                ));
                body.push(spawn_preview_section_container(
                    commands,
                    kit,
                    &snapshot.commanders,
                    p_view,
                ));
            }
            if !snapshot.cards.is_empty() {
                if grouped {
                    let mut buckets: std::collections::BTreeMap<(u8, &'static str), Vec<String>> =
                        std::collections::BTreeMap::new();
                    for line in &snapshot.cards {
                        let row = baylee_core::deckrow::parse(line).ok();
                        let name = row.as_ref().map_or(line.as_str(), |r| r.name.as_str());
                        let key = lookup_pool_card(name).map_or(
                            (
                                5,
                                if german {
                                    "Weitere Karten"
                                } else {
                                    "Other cards"
                                },
                            ),
                            |pc| preview_type_section(&pc.kinds, lang),
                        );
                        buckets.entry(key).or_default().push(line.clone());
                    }
                    for ((_, label), rows) in buckets {
                        let section_copies: usize = rows
                            .iter()
                            .map(|l| {
                                baylee_core::deckrow::parse(l)
                                    .map_or(1, |r| usize::try_from(r.count).unwrap_or(1))
                            })
                            .sum();
                        body.push(parts::caption(
                            commands,
                            kit,
                            &format!("{label} ({section_copies})"),
                        ));
                        body.push(spawn_preview_section_container(
                            commands, kit, &rows, p_view,
                        ));
                    }
                } else {
                    body.push(parts::caption(
                        commands,
                        kit,
                        Phrase::LibraryMain.text(lang),
                    ));
                    body.push(spawn_preview_section_container(
                        commands,
                        kit,
                        &snapshot.cards,
                        p_view,
                    ));
                }
            }
            if !snapshot.sideboard.is_empty() {
                body.push(parts::caption(
                    commands,
                    kit,
                    Phrase::LibrarySide.text(lang),
                ));
                body.push(spawn_preview_section_container(
                    commands,
                    kit,
                    &snapshot.sideboard,
                    p_view,
                ));
            }
        }
        _ => body.push(crate::shellkit::states::skeleton(commands, kit, 6)),
    }

    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Ghost,
        Live::Yes,
        None,
        Press::Decks(DecksPress::CloseSheet),
    );
    let add = controls::button_with_icon(
        commands,
        kit,
        '\u{f067}',
        Phrase::HouseAdd.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::Add(index)),
    );
    orders::stop(commands, close, &orders::PREVIEW, "close");
    orders::stop(commands, add, &orders::PREVIEW, "add");
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Large,
        &deck.name,
        &body,
        &[close, add],
        (
            Scrollable(List::RoomCards),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::RoomCards))),
        ),
    );
    let close_x = parts::icon_button(
        commands,
        kit,
        crate::hud::glyph::CLOSE,
        Press::Decks(DecksPress::CloseSheet),
    );
    let close_inset = px_fixed(kit.m.pad * 0.55);
    commands
        .entity(close_x)
        .insert(super::hint::HoverHint(
            Phrase::ShellClose.text(lang).to_string(),
        ))
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.position_type = PositionType::Absolute;
            n.top = close_inset;
            n.right = close_inset;
        });
    commands.entity(surface).add_child(close_x);
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Decks(DecksPress::CloseSheet));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}

impl DecksPress {
    /// What a press on this control does.
    #[allow(clippy::too_many_lines)] // one flat match, read top to bottom
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        } = cx;
        match self {
            DecksPress::Tab(tab) => {
                if state.decks.tab != tab {
                    state.decks.tab = tab;
                    state.decks.preview = None;
                    scrolled.set(List::Library, 0.0);
                }
                if tab == DecksTab::House && state.lobby.library().page != Some(Page::House) {
                    let request = state.lobby.browse_house();
                    dispatch(state, mailbox, request);
                }
            }
            DecksPress::Sort(sort) => {
                if state.decks.sort != sort {
                    state.decks.sort = sort;
                    scrolled.set(List::Library, 0.0);
                }
            }
            DecksPress::SetView(view) => {
                if state.decks.view != view {
                    state.decks.view = view;
                    scrolled.set(List::Library, 0.0);
                }
            }
            DecksPress::ToggleColor(c) => {
                if let Some(pos) = state.decks.colors.iter().position(|&x| x == c) {
                    state.decks.colors.remove(pos);
                } else {
                    state.decks.colors.push(c);
                }
                scrolled.set(List::Library, 0.0);
            }
            DecksPress::ToggleFavouritesOnly => {
                state.decks.favourites_only = !state.decks.favourites_only;
                scrolled.set(List::Library, 0.0);
            }
            DecksPress::TogglePlayableOnly => {
                state.decks.playable_only = !state.decks.playable_only;
                scrolled.set(List::Library, 0.0);
            }
            DecksPress::ClearFilters => {
                state.decks.clear_filters();
                state.lobby.set_field(Field::DeckSearch, "");
                scrolled.set(List::Library, 0.0);
            }
            DecksPress::OpenSearch => {
                state.decks.searching = true;
                state.lobby.focus_on(Field::DeckSearch);
            }
            DecksPress::Use(index) => state.lobby.select_deck(index),
            DecksPress::Edit(index) => {
                let request = state.lobby.flush_delete();
                dispatch(state, mailbox, request);
                state.commander_pick = None;
                state.build = crate::buildui::BuildUi::opened();
                let request = state.lobby.edit_deck(index);
                dispatch(state, mailbox, request);
            }
            DecksPress::Duplicate(index) => {
                let request = state.lobby.duplicate_deck(index);
                dispatch(state, mailbox, request);
            }
            DecksPress::History(index) => {
                if let Some(id) = state.lobby.decks().get(index).map(|d| d.id.clone()) {
                    state.decks.show_all = false;
                    let request = state.lobby.browse_deck_history(&id);
                    dispatch(state, mailbox, request);
                }
            }
            DecksPress::HouseHistory(index) => {
                let id = state.lobby.library().house.get(index).map(|d| d.id.clone());
                if let Some(id) = id {
                    state.decks.show_all = false;
                    state.menu = None;
                    let request = state.lobby.browse_deck_history(&id);
                    dispatch(state, mailbox, request);
                }
            }
            DecksPress::Favourite(index) => {
                if let Some(id) = state.lobby.decks().get(index).map(|d| d.id.clone()) {
                    let mut edit = prefs.edit();
                    edit.toggle_favourite(&id);
                    let kept = shelf::kept_favourites(&edit.favourites, state.lobby.decks());
                    if kept != edit.favourites {
                        edit.favourites = kept;
                    }
                }
            }
            DecksPress::Delete(index) => {
                let earlier = state.lobby.stage_delete(index);
                dispatch(state, mailbox, earlier);
                state.undo = Some(Undo {
                    kind: UndoKind::Delete,
                    left: UNDO_SECS,
                });
            }
            DecksPress::Undo => match state.undo.take().map(|u| u.kind) {
                Some(UndoKind::Delete) => state.lobby.undo_delete(),
                Some(UndoKind::Restore) => {
                    let request = state.lobby.undo_restore();
                    dispatch(state, mailbox, request);
                }
                Some(UndoKind::ToursOff | UndoKind::ToursRestarted) => {
                    if let (Some(before), Some(settings)) =
                        (state.tours_before.take(), settings.as_mut())
                    {
                        settings.tours = before;
                        settings.save();
                    }
                }
                None => {}
            },
            DecksPress::NewDeck => {
                let request = state.lobby.flush_delete();
                dispatch(state, mailbox, request);
                state.commander_pick = None;
                state.build = crate::buildui::BuildUi::opened();
                state.build.naming_new = true;
                state.build.nav = crate::buildui::Nav::Field;
                state.build.nav_epoch = state.build.nav_epoch.wrapping_add(1);
                let request = state.lobby.build_deck();
                dispatch(state, mailbox, request);
                if matches!(state.lobby.screen(), Screen::Build) {
                    let lang = state.lobby.lang();
                    let default_name = Phrase::BuildUntitled.text(lang).to_string();
                    let builder = state.lobby.builder_mut();
                    builder.set_default_name(&default_name);
                    builder.focus_on(baylee_client_core::deckbuilder::BuildField::Name);
                    builder.edit_buffer(
                        baylee_client_core::deckbuilder::BuildField::Name,
                        crate::buildui::select_all,
                    );
                }
            }
            DecksPress::Import => {
                let request = state.lobby.flush_delete();
                dispatch(state, mailbox, request);
                open_import(state, scrolled, mailbox, None);
            }
            DecksPress::Preview(index) => {
                state.decks.preview = Some(index);
                scrolled.set(List::RoomCards, 0.0);
                let choice = state
                    .lobby
                    .library()
                    .house
                    .get(index)
                    .map(|d| (d.id.clone(), d.version));
                if let Some((id, version)) = choice {
                    let request = state.lobby.preview_version(&id, version);
                    dispatch(state, mailbox, request);
                }
            }
            DecksPress::TogglePreviewGrouped => {
                state.decks.preview_ungrouped = !state.decks.preview_ungrouped;
            }
            DecksPress::SetPreviewView(view) => {
                state.decks.preview_view = view;
                scrolled.set(List::RoomCards, 0.0);
            }
            DecksPress::Add(index) => {
                state.decks.preview = None;
                let request = state.lobby.copy_house_then(index, AfterCopy::Keep);
                dispatch(state, mailbox, request);
            }
            DecksPress::AddAndUse(index) => {
                state.decks.preview = None;
                let request = state.lobby.copy_house_then(index, AfterCopy::Use);
                dispatch(state, mailbox, request);
            }
            DecksPress::Format(at) => {
                state.decks.format = if at == u8::MAX || state.decks.format == Some(at) {
                    None
                } else {
                    Some(at)
                };
                scrolled.set(List::Library, 0.0);
            }
            DecksPress::CloseSheet => {
                state.decks.preview = None;
                state.confirm_restore = false;
                if matches!(state.lobby.library().page, Some(Page::History(_))) {
                    state.lobby.close_library();
                }
            }
            DecksPress::Compare(at) => {
                let compare = match at {
                    0 => Compare::Previous,
                    1 => Compare::Current,
                    _ => Compare::Pick(None),
                };
                let request = state.lobby.compare_with(compare);
                dispatch(state, mailbox, request);
            }
            DecksPress::KeepEdits => state.confirm_restore = false,
            DecksPress::DiscardAndRestore => {
                state.confirm_restore = false;
                let request = state.lobby.restore_version();
                dispatch(state, mailbox, request);
            }
            DecksPress::Version(version) => {
                if let Some(Page::History(id)) = state.lobby.library().page.clone() {
                    let request = state.lobby.preview_version(&id, version);
                    dispatch(state, mailbox, request);
                }
            }
            DecksPress::ShowAll => state.decks.show_all = !state.decks.show_all,
            DecksPress::DiffRow => {}
            DecksPress::ExportVersion => {
                if let Some(version) = super::history::version_rows(state) {
                    state.lobby.builder_mut().open_version_export(version);
                }
            }
            DecksPress::Restore => {
                if matches!(state.lobby.screen(), Screen::Build) && state.lobby.builder().dirty() {
                    state.confirm_restore = true;
                    return;
                }
                let request = state.lobby.restore_version();
                dispatch(state, mailbox, request);
            }
            DecksPress::Retry => {
                let request = match state.lobby.library().page.clone() {
                    Some(Page::History(id)) => state.lobby.browse_deck_history(&id),
                    _ => state.lobby.browse_house(),
                };
                dispatch(state, mailbox, request);
            }
        }
    }
}

/// Opens the builder on a new deck with the import dialog over it, the
/// pasted or dropped list in its field when there is one (N-2).
pub(super) fn open_import(
    state: &mut LobbyState,
    scrolled: &mut Scrolled,
    mailbox: &Mailbox,
    text: Option<&str>,
) {
    state.commander_pick = None;
    state.build = crate::buildui::BuildUi::opened();
    let request = state.lobby.build_deck();
    dispatch(state, mailbox, request);
    if matches!(state.lobby.screen(), Screen::Build) {
        scrolled.set(List::Transfer, 0.0);
        let builder = state.lobby.builder_mut();
        builder.open_import();
        if let Some(text) = text {
            builder.import_paste(text);
        }
    }
}

/// Whether the Decks screen is up with nothing over it and no field typing:
/// where a paste or a dropped file means "import this" (N-2).
#[cfg(not(target_arch = "wasm32"))]
fn decks_bare(state: &LobbyState) -> bool {
    !state.settings_open()
        && matches!(state.lobby.screen(), Screen::Table)
        && (state.lobby.awaiting().is_none() || state.room_away)
        && state.hub == Hub::Decks
        && state.decks.tab == DecksTab::Mine
        && state.decks.preview.is_none()
        && state.menu.is_none()
        && state.confirmation.is_none()
        && !matches!(state.lobby.library().page, Some(Page::History(_)))
        && !(state.lobby.focus() == Field::DeckSearch && state.lobby.typing_here())
}

/// `Ctrl/Cmd+V` with a deck list on the clipboard, or a `.txt`, `.json`,
/// `.yaml` file dropped on the window, opens Import with the list in it
/// (N-2) — natively only: a browser's canvas sees neither (S4-5), and there
/// the Import sheet's own paste field is the door.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn paste_or_drop(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut drops: MessageReader<bevy::window::FileDragAndDrop>,
    mut state: ResMut<LobbyState>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    mut pending: Local<Option<bevy::clipboard::ClipboardRead>>,
) {
    let bare = decks_bare(&state);
    let mut text = None;
    for drop in drops.read() {
        if let bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } = drop
            && bare
            && path_buf
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| {
                    matches!(
                        e.to_ascii_lowercase().as_str(),
                        "txt" | "json" | "yaml" | "yml" | "dek" | "dck"
                    )
                })
        {
            text = std::fs::read_to_string(path_buf).ok();
        }
    }
    if let Some(read) = pending.as_mut()
        && let Some(answer) = read.poll_result()
    {
        *pending = None;
        text = answer.ok();
    }
    let command = keys.as_deref().is_some_and(|keys| {
        if crate::shellkit::keys::mac() {
            keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)
        } else {
            keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
        }
    });
    if bare
        && command
        && keys
            .as_deref()
            .is_some_and(|keys| keys.just_pressed(KeyCode::KeyV))
        && let Some(mut clipboard) = clipboard
    {
        let mut read = clipboard.fetch_text();
        match read.poll_result() {
            Some(answer) => text = answer.ok(),
            None => *pending = Some(read),
        }
    }
    if let Some(text) = text.filter(|t| !t.trim().is_empty()) {
        let request = state.lobby.flush_delete();
        dispatch(&mut state, &mailbox, request);
        open_import(&mut state, &mut scrolled, &mailbox, Some(&text));
    }
}

/// Puts on the clipboard what a press asked to (Copy invite).
pub(super) fn write_clipboard(
    mut state: ResMut<LobbyState>,
    clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    if state.clipboard_out.is_none() {
        return;
    }
    let Some(text) = state.clipboard_out.take() else {
        return;
    };
    if let Some(mut clipboard) = clipboard {
        let _ = clipboard.set_text(text);
    }
}

/// Counts the Undo down, and sends what it was holding back when it runs
/// out or the player leaves the Decks screen (S-10). The countdown itself
/// marks nothing changed: a rebuild a frame would be a flicker a second.
pub(super) fn undo_clock(time: Res<Time>, mut state: ResMut<LobbyState>, mailbox: Res<Mailbox>) {
    // A restore that landed offers its Undo.
    if state.undo.is_none() && state.lobby.library().restored.is_some() {
        state.undo = Some(Undo {
            kind: UndoKind::Restore,
            left: UNDO_SECS,
        });
    }
    let Some(undo) = state.undo else {
        return;
    };
    let on_decks = !state.settings_open()
        && matches!(state.lobby.screen(), Screen::Table)
        && state.lobby.awaiting().is_none()
        && state.hub == Hub::Decks;
    let left = undo.left - time.delta_secs();
    let over = left <= 0.0 || (undo.kind == UndoKind::Delete && !on_decks);
    if !over {
        if let Some(u) = state.bypass_change_detection().undo.as_mut() {
            u.left = left;
        }
        return;
    }
    state.undo = None;
    match undo.kind {
        UndoKind::Delete => {
            let request = state.lobby.flush_delete();
            dispatch(&mut state, &mailbox, request);
        }
        UndoKind::Restore => state.lobby.forget_restore(),
        UndoKind::ToursOff | UndoKind::ToursRestarted => state.tours_before = None,
    }
}

/// The client is closing: a deletion still waiting for its Undo goes out
/// now, on this thread, so a deck the player saw go does not come back
/// (S-10). Natively only: a browser tab that closes takes its requests with
/// it, and the deck stays.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn flush_on_exit(mut exits: MessageReader<AppExit>, mut state: ResMut<LobbyState>) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(request) = state.lobby.flush_delete() else {
        return;
    };
    let token = state.lobby.token().map(str::to_string);
    let (http, _) = super::http::build(&state.gateway, token.as_deref(), &state.lang, request);
    let outcome = crate::transport::fetch_blocking(&http);
    bevy::log::info!(
        "a deletion waiting for its Undo was sent on the way out: {}",
        outcome.map_or_else(|e| e, |r| r.status.to_string())
    );
}
