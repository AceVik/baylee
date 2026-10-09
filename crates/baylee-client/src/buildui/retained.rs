//! The builder's retained tree (the shell design, §10 #2): every section is
//! drawn into a holder of its own and redrawn only when its own key changes.
//!
//! A key is what its section draws *from*, compared and never read off a
//! change flag: the toolbar's is the search box and the chips, the pool
//! list's the results, the deck body's the shown list. So a letter typed
//! into the search redraws the toolbar and the list; adding a card redraws
//! the deck side and leaves the pool where it was (its count badges follow
//! the deck by a system of their own, `virtual_rows::in_deck`).
//!
//! What no section can absorb — the language, the shape, the pane a
//! one-pane frame shows, a header popover — makes the whole tree be drawn
//! again ([`Retained::fits`]).

use super::{BuildMenu, BuildUi, DeckTab, Env, Layout, Nav, Pane, SaveState};
use crate::cardmat::UiCards;
use baylee_client_core::deckbuilder::{
    BuildField, Entry, Grouping, SectionKey, Sort, Zone, transfer::Transfer,
};
use baylee_client_core::filterdialog::FilterPanel;
use baylee_client_core::i18n::Lang;
use baylee_client_core::textbuf::TextBuffer;
use bevy::prelude::*;

/// The holders the tree was drawn with; `None` where this shape has no such
/// section.
pub(crate) struct Holders {
    pub(crate) header: Entity,
    pub(crate) toolbar: Option<Entity>,
    pub(crate) pool_list: Option<Entity>,
    pub(crate) pool_over: Option<Entity>,
    pub(crate) deck_head: Option<Entity>,
    pub(crate) deck_body: Option<Entity>,
    pub(crate) deck_foot: Option<Entity>,
    pub(crate) rail: Option<Entity>,
    pub(crate) tabbar: Option<Entity>,
    pub(crate) menu: Option<Entity>,
    pub(crate) sheet: Option<Entity>,
    pub(crate) picker: Option<Entity>,
    pub(crate) transfer: Option<Entity>,
}

impl Default for Holders {
    fn default() -> Self {
        Self {
            header: Entity::PLACEHOLDER,
            toolbar: None,
            pool_list: None,
            pool_over: None,
            deck_head: None,
            deck_body: None,
            deck_foot: None,
            rail: None,
            tabbar: None,
            menu: None,
            sheet: None,
            picker: None,
            transfer: None,
        }
    }
}

/// What makes the whole tree be drawn again.
#[derive(Clone, PartialEq, Debug)]
struct Whole {
    lang: Lang,
    layout: Layout,
    /// The pane, where one pane stands at a time.
    pane: Option<Pane>,
    /// The header popover (drawn by the lobby's header, outside these
    /// sections), as its `Debug` spelling.
    popover: String,
}

impl Whole {
    fn of(env: &Env) -> Self {
        Self {
            lang: env.lang(),
            layout: env.layout,
            pane: env.layout.one_pane().then_some(env.ui().pane),
            popover: format!("{:?}", env.state.header_menu),
        }
    }
}

/// The header's key.
#[derive(Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent facts the bar shows
struct HeaderKey {
    name: TextBuffer,
    renaming: bool,
    editing: Option<String>,
    dirty: bool,
    changes: u32,
    saveable: bool,
    first_problem: Option<String>,
    busy: bool,
    save: SaveState,
    signed_in: bool,
    offline: bool,
    feed_down: bool,
    unread: usize,
    handle: Option<String>,
    menu: bool,
    pane: Pane,
}

/// The pool toolbar's key.
#[derive(Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent facts the toolbar shows
struct ToolbarKey {
    text: TextBuffer,
    typing: bool,
    panel: Option<FilterPanel>,
    colors: Vec<char>,
    kind: Option<String>,
    cmc: Option<u32>,
    playable: bool,
    sort: Sort,
    filtered: bool,
    rail: bool,
    syntax: bool,
    results: usize,
    pool: usize,
    loaded: bool,
    commander_pick: Option<bool>,
}

/// The pool list's key.
#[derive(Clone, PartialEq)]
struct ListKey {
    revision: u64,
    results: Vec<usize>,
    commander_pick: Option<bool>,
    loaded: bool,
    has_text: bool,
}

/// The pool's overlays: the Filters rail and the syntax popover.
#[derive(Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // independent facts the rail and popover show
struct OverKey {
    rail: bool,
    syntax: bool,
    kind: Option<String>,
    cmc: Option<u32>,
    playable: bool,
    colors: Vec<char>,
    filtered: bool,
}

/// The deck head's key: the commander slot, the tabs, the grouping.
#[derive(Clone, PartialEq)]
struct HeadKey {
    commanders: Vec<usize>,
    commander_rows: Vec<Option<Entry>>,
    counts: (u32, u32, u32, u32, u32),
    tab: DeckTab,
    grouping: Grouping,
    all_folded: bool,
    revision: u64,
    pick: Option<bool>,
}

/// The deck body's key: the shown list, or the numbers.
#[derive(Clone, PartialEq)]
struct BodyKey {
    tab: DeckTab,
    entries: Vec<Entry>,
    main: Vec<Entry>,
    grouping: Grouping,
    collapsed: Vec<SectionKey>,
    lit: Option<baylee_client_core::deckbuilder::Group>,
    hand: Vec<usize>,
    revision: u64,
    commanders: Vec<usize>,
}

/// The deck foot's key.
#[derive(Clone, PartialEq)]
struct FootKey {
    problems: Vec<(bool, String)>,
    shaky: u32,
    missing: Vec<String>,
}

/// The phone rail's key.
#[derive(Clone, PartialEq)]
struct RailKey {
    commanders: Vec<usize>,
    counts: (u32, u32),
    tab: DeckTab,
    last: Vec<usize>,
    revision: u64,
}

/// The tab bar's key (Narrow).
#[derive(Clone, PartialEq)]
struct TabbarKey {
    pane: Pane,
    main: u32,
}

/// The open menu's key, with what its items read.
#[derive(Clone, PartialEq)]
struct MenuKey {
    menu: Option<BuildMenu>,
    zone: Zone,
    entries: Vec<Entry>,
    commanders: Vec<usize>,
    busy: bool,
    history: bool,
}

/// The open sheet's key.
#[derive(Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // one sheet at a time, each its own fact
struct SheetKey {
    leaving: bool,
    inspecting: Option<usize>,
    held: Option<(u16, u16)>,
    stats: bool,
    filters: bool,
    tab: DeckTab,
    main: Vec<Entry>,
    lit: Option<baylee_client_core::deckbuilder::Group>,
    hand: Vec<usize>,
    kind: Option<String>,
    cmc: Option<u32>,
    playable: bool,
    colors: Vec<char>,
    commanders: Vec<usize>,
}

/// The import or export dialog's key.
#[derive(PartialEq, Eq)]
struct TransferKey {
    transfer: Transfer,
    missing: Vec<String>,
    loaded: bool,
    busy: bool,
}

#[allow(clippy::struct_field_names)] // one key per section, named for it
#[derive(Default)]
#[allow(clippy::option_option)] // not drawn yet, or drawn from no dialog
struct Keys {
    header: Option<HeaderKey>,
    toolbar: Option<ToolbarKey>,
    list: Option<ListKey>,
    over: Option<OverKey>,
    head: Option<HeadKey>,
    body: Option<BodyKey>,
    foot: Option<FootKey>,
    rail: Option<RailKey>,
    tabbar: Option<TabbarKey>,
    menu: Option<MenuKey>,
    sheet: Option<SheetKey>,
    picker: Option<Option<super::cardwindow::WindowKey>>,
    transfer: Option<Option<TransferKey>>,
}

/// The builder as drawn: its holders, and what each section was drawn from.
pub(crate) struct Retained {
    holders: Holders,
    whole: Whole,
    keys: Keys,
}

impl Retained {
    /// Draws every section into its holder.
    pub(crate) fn new(
        commands: &mut Commands,
        env: &Env,
        holders: Holders,
        assets: Option<&AssetServer>,
        cards: Option<&mut UiCards<'_>>,
    ) -> Self {
        let mut retained = Self {
            holders,
            whole: Whole::of(env),
            keys: Keys::default(),
        };
        retained.patch(commands, env, assets, cards);
        retained
    }

    /// Whether the tree can be patched in place for `env`, rather than
    /// drawn again whole.
    pub(crate) fn fits(&self, env: &Env) -> bool {
        self.whole == Whole::of(env)
    }

    /// Redraws the sections whose keys changed; returns how many did.
    #[allow(clippy::too_many_lines)] // one arm per section, in drawing order
    pub(crate) fn patch(
        &mut self,
        commands: &mut Commands,
        env: &Env,
        assets: Option<&AssetServer>,
        cards: Option<&mut UiCards<'_>>,
    ) -> u32 {
        let mut drawn = 0;
        let h = &self.holders;
        let k = &mut self.keys;
        drawn += redraw(
            commands,
            Some(h.header),
            &mut k.header,
            header_key(env),
            |c, at| {
                super::header::draw(c, at, env);
            },
        );
        drawn += redraw(
            commands,
            h.toolbar,
            &mut k.toolbar,
            toolbar_key(env),
            |c, at| {
                super::pool::toolbar(c, at, env);
            },
        );
        drawn += redraw(
            commands,
            h.pool_list,
            &mut k.list,
            list_key(env),
            |c, at| {
                super::pool::list(c, at, env);
            },
        );
        drawn += redraw(
            commands,
            h.pool_over,
            &mut k.over,
            over_key(env),
            |c, at| {
                super::pool::over(c, at, env);
            },
        );
        drawn += redraw(
            commands,
            h.deck_head,
            &mut k.head,
            head_key(env),
            |c, at| {
                super::deck::head(c, at, env);
            },
        );
        drawn += redraw(
            commands,
            h.deck_body,
            &mut k.body,
            body_key(env),
            |c, at| {
                super::deck::body(c, at, env);
            },
        );
        drawn += redraw(
            commands,
            h.deck_foot,
            &mut k.foot,
            foot_key(env),
            |c, at| {
                super::deck::foot(c, at, env);
            },
        );
        drawn += redraw(commands, h.rail, &mut k.rail, rail_key(env), |c, at| {
            super::deck::rail(c, at, env);
        });
        drawn += redraw(
            commands,
            h.tabbar,
            &mut k.tabbar,
            tabbar_key(env),
            |c, at| {
                super::deck::tabbar(c, at, env);
            },
        );
        drawn += redraw(commands, h.menu, &mut k.menu, menu_key(env), |c, at| {
            super::sheets::menu(c, at, env);
        });
        drawn += redraw(commands, h.sheet, &mut k.sheet, sheet_key(env), |c, at| {
            super::sheets::sheet(c, at, env);
        });
        let deck = env.deck();
        drawn += redraw(
            commands,
            h.picker,
            &mut k.picker,
            super::cardwindow::key(env),
            |c, at| {
                if let Some(window) = super::cardwindow::window(c, env, assets, cards) {
                    c.entity(at).add_child(window);
                }
            },
        );
        drawn += redraw(
            commands,
            h.transfer,
            &mut k.transfer,
            transfer_key(env),
            |c, at| {
                if let Some(open) = deck.transfer() {
                    let dialog = super::transfer::transfer_dialog(
                        c,
                        env.kit.fonts,
                        env.lobby_metrics(),
                        env.lang(),
                        deck,
                        open,
                        env.scrolled,
                    );
                    c.entity(at).add_child(dialog);
                }
            },
        );
        drawn
    }
}

/// Draws `draw` into `holder` when `now` differs from what it was drawn
/// from; 1 when it did.
fn redraw<K: PartialEq>(
    commands: &mut Commands,
    holder: Option<Entity>,
    drawn: &mut Option<K>,
    now: K,
    draw: impl FnOnce(&mut Commands, Entity),
) -> u32 {
    let Some(holder) = holder else {
        return 0;
    };
    if drawn.as_ref() == Some(&now) {
        return 0;
    }
    commands.entity(holder).despawn_children();
    draw(commands, holder);
    *drawn = Some(now);
    1
}

fn header_key(env: &Env) -> HeaderKey {
    let deck = env.deck();
    let state = env.state;
    HeaderKey {
        name: deck.buffer(BuildField::Name).clone(),
        renaming: env.ui().nav == Nav::Field && deck.focus() == BuildField::Name,
        editing: deck.editing().map(str::to_owned),
        dirty: deck.dirty(),
        changes: deck.changes(),
        saveable: deck.saveable(),
        first_problem: deck
            .problems(env.lang())
            .into_iter()
            .find(|p| p.blocking)
            .map(|p| p.message),
        busy: state.lobby.busy(),
        save: env.ui().save,
        signed_in: state.lobby.token().is_some(),
        offline: state.lobby.offline(),
        feed_down: state.feed_down,
        unread: state.bell.unread(),
        handle: super::header::handle(state),
        menu: env.ui().menu == Some(BuildMenu::Header),
        pane: env.ui().pane,
    }
}

fn toolbar_key(env: &Env) -> ToolbarKey {
    let deck = env.deck();
    let state = env.state;
    ToolbarKey {
        text: deck.buffer(BuildField::Search).clone(),
        typing: super::pool::search_has_caret(env),
        panel: deck.panel().cloned(),
        colors: deck.colors().to_vec(),
        kind: deck.kind().map(str::to_owned),
        cmc: deck.cmc(),
        playable: deck.playable_only(),
        sort: deck.sort(),
        filtered: deck.filtered(),
        rail: env.ui().rail,
        syntax: env.ui().syntax,
        results: deck.results().len(),
        pool: deck.pool().len(),
        loaded: deck.loaded(),
        commander_pick: state.commander_pick,
    }
}

fn list_key(env: &Env) -> ListKey {
    let deck = env.deck();
    ListKey {
        revision: deck.pool_revision(),
        results: deck.results().to_vec(),
        commander_pick: env.state.commander_pick,
        loaded: deck.loaded(),
        has_text: deck.has_text(),
    }
}

fn over_key(env: &Env) -> OverKey {
    let deck = env.deck();
    OverKey {
        rail: env.ui().rail && env.layout != Layout::Rail && env.layout != Layout::PhoneSingle,
        syntax: env.ui().syntax,
        kind: deck.kind().map(str::to_owned),
        cmc: deck.cmc(),
        playable: deck.playable_only(),
        colors: deck.colors().to_vec(),
        filtered: deck.filtered(),
    }
}

fn counts(env: &Env) -> (u32, u32, u32, u32, u32) {
    let c = env.deck().counts();
    (c.main, c.side, c.lands, c.creatures, c.spells)
}

fn head_key(env: &Env) -> HeadKey {
    let deck = env.deck();
    let ui = env.ui();
    HeadKey {
        commanders: deck.commanders().to_vec(),
        commander_rows: deck
            .commanders()
            .iter()
            .map(|slot| {
                deck.commander_row(*slot)
                    .and_then(|at| deck.entries(Zone::Main).get(at).cloned())
            })
            .collect(),
        counts: counts(env),
        tab: ui.tab,
        grouping: ui.grouping,
        all_folded: super::deck::all_folded(env),
        revision: deck.pool_revision(),
        pick: env.state.commander_pick,
    }
}

fn body_key(env: &Env) -> BodyKey {
    let deck = env.deck();
    let ui: &BuildUi = env.ui();
    let shown = super::deck::shown_zone(env);
    BodyKey {
        tab: if env.layout.one_pane() && ui.pane == Pane::Stats {
            DeckTab::Stats
        } else {
            ui.tab
        },
        entries: deck.entries(shown).to_vec(),
        main: deck.entries(Zone::Main).to_vec(),
        grouping: ui.grouping,
        collapsed: ui.collapsed.clone(),
        lit: ui.lit,
        hand: ui.hand.clone(),
        revision: deck.pool_revision(),
        commanders: deck.commanders().to_vec(),
    }
}

fn foot_key(env: &Env) -> FootKey {
    let deck = env.deck();
    FootKey {
        problems: deck
            .problems(env.lang())
            .into_iter()
            .map(|p| (p.blocking, p.message))
            .collect(),
        shaky: deck.counts().shaky,
        missing: deck.missing().to_vec(),
    }
}

fn rail_key(env: &Env) -> RailKey {
    let deck = env.deck();
    let c = deck.counts();
    RailKey {
        commanders: deck.commanders().to_vec(),
        counts: (c.main, c.side),
        tab: env.ui().tab,
        last: deck.last_added().to_vec(),
        revision: deck.pool_revision(),
    }
}

fn tabbar_key(env: &Env) -> TabbarKey {
    TabbarKey {
        pane: env.ui().pane,
        main: env.deck().counts().main,
    }
}

fn menu_key(env: &Env) -> MenuKey {
    let deck = env.deck();
    let zone = super::deck::shown_zone(env);
    MenuKey {
        menu: env.ui().menu,
        zone,
        entries: deck.entries(zone).to_vec(),
        commanders: deck.commanders().to_vec(),
        busy: env.state.lobby.busy(),
        history: env.state.lobby.token().is_some() && deck.editing().is_some(),
    }
}

fn sheet_key(env: &Env) -> SheetKey {
    let deck = env.deck();
    let ui = env.ui();
    let phone_filters = ui.rail && matches!(env.layout, Layout::Rail | Layout::PhoneSingle);
    SheetKey {
        leaving: env.state.confirm_leave,
        inspecting: deck.inspecting(),
        held: deck.inspecting().map(|slot| {
            (
                deck.count_of(slot, Zone::Main),
                deck.count_of(slot, Zone::Side),
            )
        }),
        stats: ui.stats_sheet,
        filters: phone_filters,
        tab: ui.tab,
        main: if ui.stats_sheet {
            deck.entries(Zone::Main).to_vec()
        } else {
            Vec::new()
        },
        lit: ui.lit,
        hand: ui.hand.clone(),
        kind: deck.kind().map(str::to_owned),
        cmc: deck.cmc(),
        playable: deck.playable_only(),
        colors: deck.colors().to_vec(),
        commanders: deck.commanders().to_vec(),
    }
}

fn transfer_key(env: &Env) -> Option<TransferKey> {
    let deck = env.deck();
    deck.transfer().map(|transfer| TransferKey {
        transfer: transfer.clone(),
        missing: deck.missing().to_vec(),
        loaded: deck.loaded(),
        busy: env.state.lobby.busy(),
    })
}
