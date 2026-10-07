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
use super::parts;
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::surfaces::{self, SheetWidth};
use crate::shellkit::{Frame as ShellFrame, Role, px_fixed, tokens};
use client_core::lobby::library::{AfterCopy, Page, Snapshot, row_changes};
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

/// What the Decks screen remembers between rebuilds that the lobby's model
/// does not: the tab, the sort, a house deck being looked at, the history
/// sheet's choices. Presses write it; nothing else does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DecksUi {
    /// Which tab is up.
    pub(crate) tab: DecksTab,
    /// The shelf's order.
    pub(crate) sort: Sort,
    /// The house deck whose cards the preview sheet shows.
    pub(crate) preview: Option<usize>,
    /// The history sheet lists every card, not only what changed.
    pub(crate) show_all: bool,
    /// The house tab's format chip: `0` Commander, `1` Freeform.
    pub(crate) format: Option<u8>,
    /// On a phone, the search box stands over the tabs.
    pub(crate) searching: bool,
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
}

/// How long an Undo is offered (§2.4: a toast's 6 s).
pub(super) const UNDO_SECS: f32 = 6.0;

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
    Use(usize),
    /// Open the builder on it.
    Edit(usize),
    /// A copy of it, kept on the shelf.
    Duplicate(usize),
    /// Its history, in a sheet.
    History(usize),
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
    /// The house tab's format chip.
    Format(u8),
    /// Close the preview or the history sheet.
    CloseSheet,
    /// The history sheet: show this version.
    Version(i32),
    /// The history sheet: every card, or only the changes.
    ShowAll,
    /// The history sheet: restore the version shown.
    Restore,
    /// Ask the library again after a failure.
    Retry,
}

/// The house formats the chips filter by, in their order.
const HOUSE_FORMATS: [(&str, Phrase); 2] = [
    ("commander", Phrase::FormatCommander),
    ("freeform", Phrase::FormatFreeform),
];

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
                // A form-wide body on Wide (four tiles a row at 1920, three at
                // 1180), the collection's width on Vast (up to six, S-14).
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

    let sort_anchor = toolbar(commands, panel, state, metrics, kit);

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
    let grid = grid(commands, kit);
    commands.entity(scroll).add_child(grid);

    match ui.tab {
        DecksTab::Mine => mine(commands, root, grid, scroll, state, prefs, metrics, kit),
        DecksTab::House => house(commands, root, grid, scroll, state, kit),
    }

    let hints = match ui.tab {
        DecksTab::Mine => vec![
            ("/", Phrase::ShellKeySearch.text(lang)),
            ("Enter", Phrase::ShellUseForNextGame.text(lang)),
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

    // The open sort menu, at the top of the tree (a tile's `⋯` menu is
    // drawn by the tile, which holds its anchor).
    if state.menu == Some(ShellMenu::DeckSort) {
        {
            if let Some(anchor) = sort_anchor {
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
        }
    }

    if ui.preview.is_some() && ui.tab == DecksTab::House {
        preview_sheet(commands, root, state, kit);
    } else if matches!(lobby.library().page, Some(Page::History(_))) && ui.tab == DecksTab::Mine {
        history_sheet(commands, root, state, kit);
    }
}

/// The centred grid the tiles flow into: `minmax(320 × factor, 1fr)`,
/// two on a phone.
fn grid(commands: &mut Commands, kit: Kit) -> Entity {
    let columns = if kit.m.frame == ShellFrame::Phone {
        vec![RepeatedGridTrack::flex(2, 1.0)]
    } else {
        vec![RepeatedGridTrack::minmax(
            GridTrackRepetition::AutoFill,
            MinTrackSizingFunction::Px(kit.m.scaled(320.0)),
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

/// The toolbar: tabs, search, sort, New deck, Import. Answers the Sort
/// button's entity, which its menu hangs from.
#[allow(clippy::too_many_lines)] // one row of controls, read left to right
fn toolbar(
    commands: &mut Commands,
    panel: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
) -> Option<Entity> {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let ui = &state.decks;
    let phone = kit.m.frame == ShellFrame::Phone;
    let bar = parts::row(commands, kit, !phone);
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
        commands.entity(bar).add_child(glass);
    } else {
        let hunt = commands
            .spawn((
                Node {
                    width: kit.m.px(if phone { 260.0 } else { 300.0 }),
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
        commands.entity(hunt).add_child(field);
        commands.entity(bar).add_child(hunt);
    }
    let mut sort_anchor = None;
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
        sort_anchor = Some(sort);
        commands.entity(bar).add_child(sort);
        let new = controls::button(
            commands,
            kit,
            &format!("+ {}", Phrase::NewDeck.text(lang)),
            Weight::Primary,
            Live::Yes,
            Some("n"),
            Press::Decks(DecksPress::NewDeck),
        );
        let import = controls::button(
            commands,
            kit,
            Phrase::ImportDeck.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Decks(DecksPress::Import),
        );
        commands.entity(bar).add_children(&[new, import]);
    } else {
        // The house tab filters by format; it has no sort (its order is the
        // gateway's).
        for (i, (_, phrase)) in HOUSE_FORMATS.iter().enumerate() {
            let at = u8::try_from(i).unwrap_or(0);
            let chip = controls::chip(
                commands,
                kit,
                phrase.text(lang),
                ui.format == Some(at),
                None,
                false,
                Press::Decks(DecksPress::Format(at)),
            );
            commands.entity(bar).add_child(chip);
        }
    }
    sort_anchor
}

/// The player's own decks: tiles, then the New deck tile; or the empty
/// shelf.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the screen's inputs; one shelf, read in order
fn mine(
    commands: &mut Commands,
    root: Entity,
    grid: Entity,
    scroll: Entity,
    state: &LobbyState,
    prefs: &baylee_client_core::prefs::Preferences,
    metrics: Metrics,
    kit: Kit,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let order = shelf::order(
        lobby.decks(),
        lobby.field(Field::DeckSearch),
        state.decks.sort,
        lobby.staged_delete(),
    );
    if lobby.decks().is_empty()
        || (order.is_empty() && lobby.field(Field::DeckSearch).trim().is_empty())
    {
        let empty = empty_shelf(commands, state, kit);
        commands.entity(scroll).add_child(empty);
        return;
    }
    if order.is_empty() {
        let none = parts::line(
            commands,
            kit,
            &Phrase::DecksNoMatch.fill(lang, &[lobby.field(Field::DeckSearch).trim()]),
            kit.m.text,
            tokens::MUTED,
        );
        commands.entity(scroll).add_child(none);
    }
    let now = parts::now_secs();
    let tiny = kit.m.frame == ShellFrame::Phone && metrics.frame == Frame::Compact;
    for index in order {
        let deck = &lobby.decks()[index];
        let next = lobby.selected() == Some(index);
        let favourite = prefs.favourite(&deck.id);
        let mut actions = Vec::new();
        if !next {
            actions.push(controls::button(
                commands,
                kit,
                Phrase::ShellUseForNextGame.text(lang),
                Weight::Primary,
                Live::Yes,
                None,
                Press::Decks(DecksPress::Use(index)),
            ));
        }
        if next || !tiny {
            actions.push(controls::button(
                commands,
                kit,
                Phrase::ShellEdit.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                Press::Decks(DecksPress::Edit(index)),
            ));
        }
        let more = parts::icon_button(
            commands,
            kit,
            parts::ELLIPSIS,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::Deck(index))),
        );
        let badges: Vec<(String, Color)> = shelf::badges(lang, deck, next)
            .into_iter()
            .enumerate()
            .map(|(i, said)| {
                let ink = match i {
                    0 => tokens::ACCENT,
                    _ if next && i == 1 => tokens::ACCENT,
                    _ => tokens::GOLD,
                };
                (said, ink)
            })
            .collect();
        let look = TileLook {
            name: &deck.name,
            identity: &deck.identity,
            art: deck.art(),
            meta: shelf::meta_line(lang, deck, now),
            badges,
            star: favourite,
            blurb: None,
            next,
        };
        let tile = tile(commands, kit, lang, &look, &actions, more, None);
        commands.entity(grid).add_child(tile);
        if state.menu == Some(ShellMenu::Deck(index)) {
            let mut items = Vec::new();
            if tiny && !next {
                items.push(menus::item(
                    Phrase::ShellEdit.text(lang),
                    Press::Decks(DecksPress::Edit(index)),
                ));
            }
            items.extend([
                menus::item(
                    Phrase::DecksDuplicate.text(lang),
                    Press::Decks(DecksPress::Duplicate(index)),
                ),
                menus::item(
                    Phrase::DecksHistory.text(lang),
                    Press::Decks(DecksPress::History(index)),
                ),
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
    // The dashed New deck tile closes the shelf.
    let new = new_tile(commands, kit, lang);
    commands.entity(grid).add_child(new);
}

/// The empty shelf: a sentence, three house decks to start from, Import.
fn empty_shelf(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let import = controls::button(
        commands,
        kit,
        Phrase::ImportDeck.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::Import),
    );
    let new = controls::button(
        commands,
        kit,
        &format!("+ {}", Phrase::NewDeck.text(lang)),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::NewDeck),
    );
    let actions = parts::row(commands, kit, true);
    commands.entity(actions).add_children(&[new, import]);
    let column = crate::shellkit::states::empty(
        commands,
        kit,
        crate::hud::glyph::LIBRARY,
        Phrase::DecksEmptyTitle.text(lang),
        None,
    );
    let minis = super::play::mini_tiles(commands, state, kit);
    commands.entity(column).add_children(&[minis, actions]);
    column
}

/// The New deck tile: dashed, last on the shelf.
fn new_tile(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let face = commands
        .spawn((
            Role::Tile,
            Node {
                min_height: px_fixed(kit.m.tile),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: kit.m.px(6.0),
                padding: UiRect::all(kit.m.px(12.0)),
                border: UiRect::all(px_fixed(2.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            BackgroundColor(Color::NONE),
            Press::Decks(DecksPress::NewDeck),
            crate::ambience::Feel::new(Color::NONE),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(format!("+ {}", Phrase::NewDeck.text(lang))),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
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
    commands
        .entity(hint)
        .insert(TextLayout::justify(Justify::Center));
    commands.entity(face).add_children(&[title, hint]);
    face
}

/// The house decks: tiles with a blurb, **Add and use** and **Add**.
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
    let query = shelf::fold(lobby.field(Field::DeckSearch).trim());
    let format = state
        .decks
        .format
        .and_then(|i| HOUSE_FORMATS.get(usize::from(i)))
        .map(|(f, _)| *f);
    for (index, deck) in lib.house.iter().enumerate() {
        if !query.is_empty()
            && !shelf::fold(&deck.name).contains(&query)
            && !deck
                .commanders
                .iter()
                .any(|c| shelf::fold(c).contains(&query))
        {
            continue;
        }
        if format.is_some_and(|f| deck.format != f) {
            continue;
        }
        let add_use = controls::button(
            commands,
            kit,
            Phrase::ShellAddAndUse.text(lang),
            Weight::Primary,
            if lib.loading {
                Live::No(Phrase::LibraryLoading.text(lang))
            } else {
                Live::Yes
            },
            None,
            Press::Decks(DecksPress::AddAndUse(index)),
        );
        let add = controls::button(
            commands,
            kit,
            Phrase::HouseAdd.text(lang),
            Weight::Secondary,
            if lib.loading {
                Live::No(Phrase::LibraryLoading.text(lang))
            } else {
                Live::Yes
            },
            None,
            Press::Decks(DecksPress::Add(index)),
        );
        let more = parts::icon_button(
            commands,
            kit,
            parts::ELLIPSIS,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::House(index))),
        );
        let copies = if deck.copies > 0 {
            deck.copies.to_string()
        } else {
            deck.cards.to_string()
        };
        let mut badges = vec![(shelf::format_label(lang, &deck.format), tokens::ACCENT)];
        // A playable badge only on a deck under 100 % (§6).
        if deck.unplayable > 0 {
            let n = deck.unplayable.to_string();
            badges.push((
                Phrase::counted(
                    usize::try_from(deck.unplayable).unwrap_or(usize::MAX),
                    Phrase::DecksUnplayableOne,
                    Phrase::DecksUnplayableMany,
                )
                .fill(lang, &[&n]),
                tokens::GOLD,
            ));
        }
        let look = TileLook {
            name: &deck.name,
            identity: &deck.identity,
            art: deck.art(),
            meta: Phrase::DeckMeta.fill(lang, &[&copies, &deck.sideboard.to_string()]),
            badges,
            star: false,
            blurb: (!deck.description.is_empty()).then_some(deck.description.as_str()),
            next: false,
        };
        let tile = tile(
            commands,
            kit,
            lang,
            &look,
            &[add_use, add],
            more,
            Some(Press::Decks(DecksPress::Preview(index))),
        );
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
                menus::item(
                    Phrase::ShellAddAndUse.text(lang),
                    Press::Decks(DecksPress::AddAndUse(index)),
                ),
            ];
            menus::draw(commands, root, kit, more, items);
        }
    }
}

/// What a tile shows.
pub(super) struct TileLook<'a> {
    pub(super) name: &'a str,
    pub(super) identity: &'a str,
    pub(super) art: Option<shelf::DeckArt>,
    pub(super) meta: String,
    pub(super) badges: Vec<(String, Color)>,
    pub(super) star: bool,
    pub(super) blurb: Option<&'a str>,
    /// The next game's deck: ringed.
    pub(super) next: bool,
}

/// A deck tile (§2.4 Tile): the band (art and credit, name, identity), the
/// meta line, the badge line, and the action row with its `⋯` at the end.
/// `open` makes the band a press (a house deck's preview).
#[allow(clippy::too_many_lines)] // one tile, band to action row
pub(super) fn tile(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    look: &TileLook,
    actions: &[Entity],
    more: Entity,
    open: Option<Press>,
) -> Entity {
    let phone = kit.m.frame == ShellFrame::Phone;
    let card = commands
        .spawn((
            Role::Tile,
            Node {
                flex_direction: FlexDirection::Column,
                min_height: px_fixed(kit.m.tile),
                min_width: px(0),
                border: UiRect::all(px_fixed(if look.next { 2.0 } else { 1.0 })),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(if look.next {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
        ))
        .id();
    let band = commands
        .spawn((
            Node {
                column_gap: kit.m.px(12.0),
                padding: UiRect::all(kit.m.px(if phone { 8.0 } else { 12.0 })),
                border_radius: BorderRadius::top(px_fixed(tokens::RADIUS_PANEL - 1.0)),
                ..default()
            },
            parts::identity_gradient(look.identity),
        ))
        .id();
    match open {
        Some(press) => {
            commands.entity(band).insert(press);
        }
        None => {
            commands.entity(band).insert(Pickable::IGNORE);
        }
    }
    if let Some(art) = &look.art {
        let picture = parts::art_with_credit(commands, kit, lang, art, 82.0);
        commands.entity(band).add_child(picture);
    }
    let words = commands
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
    let name = commands
        .spawn((
            Text::new(look.name),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.1),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(words).add_child(name);
    if !phone && !look.identity.is_empty() {
        let discs = parts::identity_discs(commands, kit, look.identity);
        commands.entity(words).add_child(discs);
    }
    commands.entity(band).add_child(words);
    if look.star {
        let star = commands
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
            .id();
        commands.entity(band).add_child(star);
    }
    let inside = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: kit.m.px(6.0),
                padding: UiRect::all(kit.m.px(if phone { 8.0 } else { 12.0 })),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(blurb) = look.blurb {
        // The blurb's opening (§6: "a one-line blurb"), cut at a word so it
        // keeps to two short lines at the narrowest tile; the preview
        // sheet says it whole.
        let said = commands
            .spawn((
                Text::new(shortened(blurb, 72)),
                crate::hud::tf(kit.fonts, kit.m.small),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(inside).add_child(said);
    }
    let meta = parts::line(commands, kit, &look.meta, kit.m.small, tokens::MUTED);
    commands.entity(inside).add_child(meta);
    let badges = parts::row(commands, kit, true);
    if phone && !look.identity.is_empty() {
        let letters = parts::badge(commands, kit, look.identity, tokens::INK);
        commands.entity(badges).add_child(letters);
    }
    for (said, ink) in &look.badges {
        let b = parts::badge(commands, kit, said, *ink);
        commands.entity(badges).add_child(b);
    }
    commands.entity(inside).add_child(badges);
    let row = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Auto),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(actions);
    let gap = parts::grow(commands);
    commands.entity(row).add_children(&[gap, more]);
    commands.entity(inside).add_child(row);
    commands.entity(card).add_children(&[band, inside]);
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

/// The history sheet (960 × factor): versions left, the changes right.
#[allow(clippy::too_many_lines)] // a sheet, read top to bottom
fn history_sheet(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let lib = lobby.library();
    let ui = &state.decks;
    let name = match &lib.page {
        Some(Page::History(id)) => lobby
            .decks()
            .iter()
            .find(|d| &d.id == id)
            .map_or_else(String::new, |d| d.name.clone()),
        _ => String::new(),
    };
    let mut body = Vec::new();
    let columns = commands
        .spawn((
            Node {
                column_gap: px_fixed(kit.m.gap * 1.5),
                row_gap: px_fixed(kit.m.gap),
                flex_direction: if kit.m.frame == ShellFrame::Phone {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                align_items: AlignItems::Start,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    body.push(columns);
    let versions = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(4.0),
                width: kit.m.px(240.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let detail = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(6.0),
                flex_grow: 1.0,
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(columns).add_children(&[versions, detail]);
    let shown = lib.preview.as_ref().map(|(_, s)| s.version);
    if lib.loading && lib.history.is_none() {
        let skeleton = crate::shellkit::states::skeleton(commands, kit, 4);
        commands.entity(versions).add_child(skeleton);
    }
    if let Some(history) = &lib.history {
        let rows = std::iter::once((history.version, history.updated_at))
            .chain(history.past.iter().map(|v| (v.version, v.superseded_at)));
        for (version, at) in rows {
            let when = when(at);
            let label = if version == history.version {
                Phrase::HistoryCurrentRow.fill(lang, &[&version.to_string()])
            } else {
                Phrase::HistoryRow.fill(lang, &[&version.to_string()])
            };
            let on = shown == Some(version);
            let item = surfaces::list_row(
                commands,
                kit,
                &label,
                &when,
                &[],
                Press::Decks(DecksPress::Version(version)),
            );
            commands.entity(item).insert(BackgroundColor(if on {
                tokens::SELECTED
            } else {
                Color::NONE
            }));
            commands.entity(versions).add_child(item);
        }
        if history.past.is_empty() {
            let none = parts::line(
                commands,
                kit,
                Phrase::NoPastVersions.text(lang),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(versions).add_child(none);
        }
    }
    if let (Some((_, snapshot)), Some(history)) = (&lib.preview, &lib.history) {
        // What this version holds against the deck as it is now: the one
        // comparison the history answers without another request (a
        // version's own changes need the one before it, which is not read;
        // principle 5 — no control that would not work).
        let compare = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: kit.m.px(8.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let label = parts::line(
            commands,
            kit,
            if snapshot.version == history.version {
                Phrase::HistoryIsCurrent.text(lang)
            } else {
                Phrase::HistoryAgainstCurrent.text(lang)
            },
            kit.m.small,
            tokens::MUTED,
        );
        let gap = parts::grow(commands);
        let all = controls::chip(
            commands,
            kit,
            Phrase::HistoryShowAll.text(lang),
            ui.show_all,
            None,
            false,
            Press::Decks(DecksPress::ShowAll),
        );
        commands.entity(compare).add_children(&[label, gap, all]);
        commands.entity(detail).add_child(compare);
        let base: &Snapshot = lib.current.as_ref().unwrap_or(snapshot);
        diff(commands, detail, kit, lang, base, snapshot);
        if ui.show_all {
            for (label, rows) in [
                (Phrase::LibraryCommanders, &snapshot.commanders),
                (Phrase::LibraryMain, &snapshot.cards),
                (Phrase::LibrarySide, &snapshot.sideboard),
            ] {
                if rows.is_empty() {
                    continue;
                }
                let title = parts::caption(commands, kit, label.text(lang));
                let text = parts::line(commands, kit, &rows.join("\n"), kit.m.small, tokens::INK);
                commands.entity(detail).add_children(&[title, text]);
            }
        }
    }
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::CloseSheet),
    );
    let current = lib
        .preview
        .as_ref()
        .zip(lib.history.as_ref())
        .is_none_or(|((_, s), h)| s.version == h.version);
    let restore = controls::button(
        commands,
        kit,
        Phrase::HistoryRestore.text(lang),
        Weight::Primary,
        if current {
            Live::No(Phrase::HistoryRestoreCurrent.text(lang))
        } else if lib.loading {
            Live::No(Phrase::LibraryLoading.text(lang))
        } else {
            Live::Yes
        },
        None,
        Press::Decks(DecksPress::Restore),
    );
    let title = Phrase::HistoryTitle.fill(lang, &[&name]);
    let surface = surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Large,
        &title,
        &body,
        &[close, restore],
    );
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Decks(DecksPress::CloseSheet));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}

/// A unix time as "2026-10-07 14:02" on the device's clock.
fn when(at: i64) -> String {
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(at) else {
        return String::new();
    };
    let at = time::UtcOffset::local_offset_at(utc).map_or(utc, |o| utc.to_offset(o));
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute()
    )
}

/// The changes from `before` to `after`, by zone: `+3 Island` in accent,
/// `−2 Forest` in danger ink; "No changes" where a zone has none.
fn diff(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    before: &Snapshot,
    after: &Snapshot,
) {
    for (label, from, to) in [
        (Phrase::LibraryMain, &before.cards, &after.cards),
        (Phrase::LibrarySide, &before.sideboard, &after.sideboard),
        (
            Phrase::LibraryCommanders,
            &before.commanders,
            &after.commanders,
        ),
    ] {
        let changes = row_changes(from, to);
        if changes.is_empty() && label != Phrase::LibraryMain {
            continue;
        }
        let title = parts::caption(commands, kit, label.text(lang));
        commands.entity(parent).add_child(title);
        if changes.is_empty() {
            let same = parts::line(
                commands,
                kit,
                Phrase::NoChanges.text(lang),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(parent).add_child(same);
        }
        for (name, delta) in changes {
            let sign = if delta > 0 { "+" } else { "\u{2212}" };
            let change = parts::line(
                commands,
                kit,
                &format!("{sign}{}  {name}", delta.unsigned_abs()),
                kit.m.small,
                if delta > 0 {
                    tokens::ACCENT
                } else {
                    tokens::DANGER
                },
            );
            commands.entity(parent).add_child(change);
        }
    }
}

/// The preview sheet: a house deck's cards, Add and Add and use in the foot.
fn preview_sheet(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let lib = lobby.library();
    let Some(index) = state.decks.preview else {
        return;
    };
    let Some(deck) = lib.house.get(index) else {
        return;
    };
    let mut body = Vec::new();
    if !deck.description.is_empty() {
        body.push(parts::line(
            commands,
            kit,
            &deck.description,
            kit.m.small,
            tokens::MUTED,
        ));
    }
    match &lib.preview {
        Some((id, snapshot)) if *id == deck.id => {
            for (label, rows) in [
                (Phrase::LibraryCommanders, &snapshot.commanders),
                (Phrase::LibraryMain, &snapshot.cards),
                (Phrase::LibrarySide, &snapshot.sideboard),
            ] {
                if rows.is_empty() {
                    continue;
                }
                body.push(parts::caption(commands, kit, label.text(lang)));
                body.push(parts::line(
                    commands,
                    kit,
                    &rows.join("\n"),
                    kit.m.small,
                    tokens::INK,
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
    let add_use = controls::button(
        commands,
        kit,
        Phrase::ShellAddAndUse.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::AddAndUse(index)),
    );
    let add = controls::button(
        commands,
        kit,
        Phrase::HouseAdd.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        Press::Decks(DecksPress::Add(index)),
    );
    let surface = surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Medium,
        &deck.name,
        &body,
        &[close, add_use, add],
    );
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
            ..
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
            DecksPress::OpenSearch => {
                state.decks.searching = true;
                state.lobby.focus_on(Field::DeckSearch);
            }
            DecksPress::Use(index) => state.lobby.select_deck(index),
            DecksPress::Edit(index) => {
                let request = state.lobby.flush_delete();
                dispatch(state, mailbox, request);
                state.commander_pick = None;
                state.pane = Pane::Deck;
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
                None => {}
            },
            DecksPress::NewDeck => {
                let request = state.lobby.flush_delete();
                dispatch(state, mailbox, request);
                state.commander_pick = None;
                state.pane = Pane::Deck;
                let request = state.lobby.build_deck();
                dispatch(state, mailbox, request);
            }
            DecksPress::Import => {
                let request = state.lobby.flush_delete();
                dispatch(state, mailbox, request);
                open_import(state, scrolled, mailbox, None);
            }
            DecksPress::Preview(index) => {
                state.decks.preview = Some(index);
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
                state.decks.format = if state.decks.format == Some(at) {
                    None
                } else {
                    Some(at)
                };
            }
            DecksPress::CloseSheet => {
                state.decks.preview = None;
                if matches!(state.lobby.library().page, Some(Page::History(_))) {
                    state.lobby.close_library();
                }
            }
            DecksPress::Version(version) => {
                if let Some(Page::History(id)) = state.lobby.library().page.clone() {
                    let request = state.lobby.preview_version(&id, version);
                    dispatch(state, mailbox, request);
                }
            }
            DecksPress::ShowAll => state.decks.show_all = !state.decks.show_all,
            DecksPress::Restore => {
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
    state.pane = Pane::Deck;
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
    let Some(keys) = keys else {
        return;
    };
    let command = if crate::shellkit::keys::mac() {
        keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)
    } else {
        keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
    };
    if bare
        && command
        && keys.just_pressed(KeyCode::KeyV)
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
