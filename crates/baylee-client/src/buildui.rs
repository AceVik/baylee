//! The deck builder screen (the shell design, `.claude/ux-b6/DESIGN-v5.md`
//! §7): the pool on the left, the deck on the right, one save state in the
//! header.
//!
//! Split out of `lobby.rs` for the same reason as `settingsui.rs` — it is a
//! screen, not a lobby. Everything that *decides* is in
//! [`baylee_client_core::deckbuilder`]; what is here is the node tree, drawn
//! with the shell's kit (`crate::shellkit`), and the little view state that
//! is the screen's own ([`BuildUi`]: which tab, which pane, which menu).
//!
//! The shape follows the size class (§2.7):
//!
//! - **Wide / Vast**: two mirrored columns, the pool 3/5, the deck 2/5.
//! - **Narrow / Compact** (960 × 700): one column and a bottom tab bar,
//!   Pool · Deck n · Stats; Import, Export and History move into the
//!   header's `⋯`.
//! - **Phone**: the pool and a 260-px deck rail; at 640 one pane, with a
//!   Pool / Deck switch in the header.
//!
//! Every list row has one line per cell, cut rather than wrapped, at the
//! pitch the kit gives a row (`shell.row`) and clipped (§10 #6): a fixed
//! pitch and free content can no longer disagree, which is what made two-line
//! names overlap at 960 × 700.
//!
//! The tree is retained (§10 #2): each section — the header, the pool's
//! toolbar, the pool list, the deck's head, its body, its foot, each overlay
//! — is redrawn alone when its own key changes, so a key typed into the search
//! redraws the toolbar and the list and nothing else.

use crate::cardmat::UiCards;
use crate::hud::{UiFonts, icon_tf, tf, tf_bold};
use crate::lobby::{BuildPress, LobbyState, Metrics, Press, Scrolled};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Stop, TabOrder};
use crate::shellkit::{Frame, Role, px_fixed, tokens};
use baylee_client_core::deckbuilder::{
    CURVE_BUCKETS, Coverage, DeckBuilder, Group, Grouping, SectionKey, Zone,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::images::FinishTreatment;
use baylee_core::preset::Finish;
use bevy::prelude::*;

pub(crate) mod print_picker;
pub(crate) mod transfer;
pub(crate) mod virtual_rows;

mod deck;
pub(crate) mod draft;
pub(crate) mod focus;
pub(crate) mod header;
pub(crate) mod pool;
mod retained;
mod rows;
mod sheets;
mod stats;

pub(crate) use deck::order as deck_order;
pub(crate) use retained::Retained;

// ------------------------------------------------------------------ state

/// The focus table every builder control names (`KEYBOARD.md` §1.3, the
/// builder's row): the header, the pool's toolbar and list, the deck side.
pub(crate) const BUILDER: &str = "builder";

/// The builder's Tab order (`KEYBOARD.md` §1.3).
pub(crate) const BUILDER_ORDER: TabOrder = TabOrder {
    name: BUILDER,
    stops: &[
        "back",
        "title",
        "import",
        "export",
        "history",
        "save",
        "retry",
        "menu",
        "panes",
        "search",
        "syntax",
        "gear",
        "colours",
        "chips",
        "filters",
        "playable",
        "sort",
        "clear",
        "pool",
        "commander",
        "tabs",
        "group",
        "collapse",
        "deck",
        "lit",
        "draw",
        "rail",
    ],
    modal: false,
};

/// The table of the builder's sheets and menus: modal, so Tab cycles inside
/// one while it is up.
pub(crate) const BUILDER_SHEET: &str = "builder-sheet";

/// The order inside a builder sheet or menu.
pub(crate) const BUILDER_SHEET_ORDER: TabOrder = TabOrder {
    name: BUILDER_SHEET,
    stops: &[
        "keep",
        "discard",
        "add",
        "other",
        "printing",
        "commander",
        "item",
        "close",
    ],
    modal: true,
};

/// A builder control's focus stop.
pub(crate) const fn stop(id: &'static str) -> Stop {
    Stop::new(BUILDER, id)
}

/// A builder sheet's or menu's focus stop.
pub(crate) const fn sheet_stop(id: &'static str) -> Stop {
    Stop::new(BUILDER_SHEET, id)
}

/// The deck side's three tabs. Main and Sideboard are also where `+` adds
/// (`DeckBuilder::zone`); Stats leaves that where it was.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum DeckTab {
    /// The main deck.
    #[default]
    Main,
    /// The sideboard.
    Side,
    /// The deck's numbers.
    Stats,
}

/// The part of the builder a one-pane frame shows (Narrow's bottom tabs,
/// the 640 phone's switch).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Pane {
    /// The searchable pool.
    #[default]
    Pool,
    /// The deck.
    Deck,
    /// The deck's numbers.
    Stats,
}

/// A builder menu that is open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BuildMenu {
    /// The header's `⋯`.
    Header,
    /// A pool row's `⋯`, by pool slot.
    Pool(usize),
    /// A deck row's `⋯`, by its index in the shown list.
    Deck(usize),
}

/// Where the last save stands — the header's one save state (§2.5).
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) enum SaveState {
    /// Nothing asked this visit.
    #[default]
    Idle,
    /// The request is out.
    Saving,
    /// Saved at `at` (seconds of real time), `minutes` ago as last drawn.
    Saved { at: f64, minutes: u32 },
    /// The gateway refused, or did not answer.
    Failed,
}

/// Where the keyboard is in the builder: typing into one of the deck
/// builder's boxes, on a row of a list, or on a control the Tab walk
/// reached (`KEYBOARD.md` §7.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Nav {
    /// Typing into `DeckBuilder::focus()`'s box.
    #[default]
    Field,
    /// On a pool row, by its place in the results.
    Pool(usize),
    /// On a deck row, by its place in the drawn list.
    Deck(usize),
    /// On another control, or nowhere.
    Idle,
}

impl Nav {
    /// Whether it is the same kind of place, cursor aside.
    #[must_use]
    pub(crate) fn same_kind(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// The builder's own view state: what the screen shows, not what the deck
/// is (that is `DeckBuilder`'s). Written only where it changes, since a write
/// to the lobby's state is a redraw.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BuildUi {
    /// The deck side's tab.
    pub(crate) tab: DeckTab,
    /// The pane a one-pane frame shows.
    pub(crate) pane: Pane,
    /// How the deck list is sectioned.
    pub(crate) grouping: Grouping,
    /// The deck list's folded sections.
    pub(crate) collapsed: Vec<SectionKey>,
    /// Whether the Filters rail (a sheet on a phone) is open.
    pub(crate) rail: bool,
    /// Whether the search syntax popover is open.
    pub(crate) syntax: bool,
    /// The open menu.
    pub(crate) menu: Option<BuildMenu>,
    /// The type the Stats curve lights.
    pub(crate) lit: Option<Group>,
    /// The last sample hand, as pool slots.
    pub(crate) hand: Vec<usize>,
    /// The header's save state.
    pub(crate) save: SaveState,
    /// Whether the phone's Stats sheet is up.
    pub(crate) stats_sheet: bool,
    /// Where the keyboard is ([`Nav`]); cursor moves inside a list are
    /// written past change detection, since only the highlight follows them.
    pub(crate) nav: Nav,
    /// Bumped whenever the keyboard model moves [`Self::nav`], so the focus
    /// ring follows a key and a click alike (`focus::follow`).
    pub(crate) nav_epoch: u64,
}

impl BuildUi {
    /// The view of a builder just opened: the pool, the main deck, the caret
    /// in the search (`KEYBOARD.md` W4: "builder with an empty deck, focus in
    /// pool search").
    #[must_use]
    pub(crate) fn opened() -> Self {
        Self::default()
    }

    /// The zone the tab names, if it names one.
    #[must_use]
    pub(crate) fn zone(&self) -> Option<Zone> {
        match self.tab {
            DeckTab::Main => Some(Zone::Main),
            DeckTab::Side => Some(Zone::Side),
            DeckTab::Stats => None,
        }
    }

    /// Whether a sheet or menu of the builder's stands over it.
    #[must_use]
    pub(crate) fn covered(&self) -> bool {
        self.menu.is_some() || self.stats_sheet
    }
}

/// Moves the keyboard to `nav`, and the focus ring with it
/// (`focus::follow`, by the epoch). A move inside one list is written past
/// change detection: only the row's highlight follows it, never a redraw.
pub(crate) fn move_nav(state: &mut ResMut<LobbyState>, nav: Nav) {
    if state.build.nav.same_kind(nav) {
        let quiet = state.bypass_change_detection();
        quiet.build.nav = nav;
        quiet.build.nav_epoch = quiet.build.nav_epoch.wrapping_add(1);
    } else {
        state.build.nav = nav;
        state.build.nav_epoch = state.build.nav_epoch.wrapping_add(1);
    }
}

/// Selects a box's whole text (a rename starts with the old name selected).
pub(crate) fn select_all(buffer: &mut baylee_client_core::textbuf::TextBuffer) {
    buffer.select_all();
}

/// The builder's shape for the window it is drawn in (§2.7).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Layout {
    /// Pool and deck side by side (Wide, Vast).
    Columns,
    /// One pane and a bottom tab bar (Narrow, Compact).
    Single,
    /// The pool and a 260-px deck rail (Phone).
    Rail,
    /// One pane and a switch in the header (a Phone under 720 wide).
    PhoneSingle,
}

impl Layout {
    /// The shape for a size class at a window width.
    #[must_use]
    pub(crate) fn of(frame: Frame, width: f32) -> Self {
        match frame {
            Frame::Wide | Frame::Vast => Self::Columns,
            Frame::Narrow | Frame::Compact => Self::Single,
            Frame::Phone if width < PHONE_TWO_PANES => Self::PhoneSingle,
            Frame::Phone => Self::Rail,
        }
    }

    /// Whether one pane stands at a time.
    #[must_use]
    pub(crate) fn one_pane(self) -> bool {
        matches!(self, Self::Single | Self::PhoneSingle)
    }
}

/// The narrowest phone window that still takes the pool and the deck rail
/// side by side (§2.7: 844 does, 640 does not).
pub(crate) const PHONE_TWO_PANES: f32 = 720.0;

/// What every drawing function in the builder reads.
#[derive(Clone, Copy)]
pub(crate) struct Env<'a> {
    /// The kit: fonts, sizes, language.
    pub(crate) kit: Kit<'a>,
    /// The lobby, the deck builder inside it, and [`BuildUi`].
    pub(crate) state: &'a LobbyState,
    /// Where each list was left.
    pub(crate) scrolled: &'a Scrolled,
    /// The shape.
    pub(crate) layout: Layout,
    /// The Save key's cap (`Cmd+S` / `Ctrl+S`), from the account's keymap.
    pub(crate) save_keys: Option<&'a str>,
}

impl Env<'_> {
    /// The deck builder.
    pub(crate) fn deck(&self) -> &DeckBuilder {
        self.state.lobby.builder()
    }

    /// The interface's language.
    pub(crate) fn lang(&self) -> Lang {
        self.state.lobby.lang()
    }

    /// The builder's view state.
    pub(crate) fn ui(&self) -> &BuildUi {
        &self.state.build
    }

    /// The lobby's older sizes, for the makers the builder still borrows
    /// from it (the text field, the scrollbar), at the kit's values.
    pub(crate) fn lobby_metrics(&self) -> Metrics {
        let m = self.kit.m;
        Metrics {
            frame: match m.frame {
                Frame::Phone | Frame::Compact => Frame::Compact,
                Frame::Narrow => Frame::Narrow,
                Frame::Wide | Frame::Vast => Frame::Wide,
            },
            text: m.text,
            head: m.head,
            small: m.small,
            tap: m.control,
            pad: m.pad,
            gap: m.gap,
        }
    }

    /// A pool row's pitch: the kit's row, never under a finger's target.
    pub(crate) fn pool_pitch(&self) -> f32 {
        self.kit.m.row.max(self.kit.m.hit + 6.0)
    }

    /// A deck row's pitch: three quarters of a pool row, never under a
    /// finger's target.
    pub(crate) fn deck_pitch(&self) -> f32 {
        (self.kit.m.row * 0.75).max(self.kit.m.hit + 6.0)
    }

    /// A deck section heading's height.
    pub(crate) fn head_pitch(&self) -> f32 {
        (self.kit.m.small * 2.6).max(if self.kit.m.touch() {
            self.kit.m.hit
        } else {
            0.0
        })
    }
}

// ------------------------------------------------------------- the screen

/// The deck builder, under the shell's strips at the top of `root`.
pub(crate) fn builder(
    commands: &mut Commands,
    root: Entity,
    env: &Env,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) -> Retained {
    let m = env.kit.m;
    let header = holder(
        commands,
        root,
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
    );
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Row,
                column_gap: px_fixed(m.body),
                padding: UiRect::all(px_fixed(m.body)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);

    let ui = env.ui();
    let (show_pool, show_deck, show_rail) = match env.layout {
        Layout::Columns => (true, true, false),
        Layout::Rail => (true, false, true),
        Layout::Single | Layout::PhoneSingle => {
            (ui.pane == Pane::Pool, ui.pane != Pane::Pool, false)
        }
    };
    let mut holders = retained::Holders {
        header,
        ..retained::Holders::default()
    };
    if show_pool {
        let panel = pool::panel(commands, env, &mut holders);
        commands.entity(body).add_child(panel);
    }
    if show_deck {
        let panel = deck::panel(commands, env, &mut holders);
        commands.entity(body).add_child(panel);
    }
    if show_rail {
        let rail = deck::rail_panel(commands, env, &mut holders);
        commands.entity(body).add_child(rail);
    }
    if env.layout == Layout::Single {
        holders.tabbar = Some(holder(
            commands,
            root,
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
        ));
    }
    // Last, so every overlay stands over the whole builder.
    holders.menu = Some(overlay_holder(commands, root));
    holders.sheet = Some(overlay_holder(commands, root));
    holders.picker = Some(overlay_holder(commands, root));
    holders.transfer = Some(overlay_holder(commands, root));
    Retained::new(commands, env, holders, assets, cards)
}

/// An empty node a section is drawn into, so the section can be drawn again
/// without the rest of the tree.
pub(crate) fn holder(commands: &mut Commands, parent: Entity, node: Node) -> Entity {
    let id = commands.spawn((node, Pickable::IGNORE)).id();
    commands.entity(parent).add_child(id);
    id
}

/// A holder for an overlay: over the whole window, taking no room in the
/// column and catching nothing itself; what is drawn into it positions
/// itself (a sheet's scrim fills it, a menu stands where it is placed).
fn overlay_holder(commands: &mut Commands, parent: Entity) -> Entity {
    holder(
        commands,
        parent,
        Node {
            position_type: PositionType::Absolute,
            left: px_fixed(0.0),
            top: px_fixed(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    )
}

// ------------------------------------------------------------ the makers

/// One line of text that never wraps: the words in a box that may shrink
/// and clips them (`Overflow` clips children, not a node's own glyphs), so a
/// long name is cut at the box's edge and the row's height never changes.
pub(crate) fn cell(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    size: f32,
    ink: Color,
    bold: bool,
) -> Entity {
    let words = commands
        .spawn((
            Text::new(text),
            if bold {
                tf_bold(kit.fonts, size)
            } else {
                tf(kit.fonts, size)
            },
            TextColor(ink),
            TextLayout::no_wrap(),
            Node {
                min_width: px_fixed(0.0),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let clip = commands
        .spawn((
            Node {
                min_width: px_fixed(0.0),
                flex_shrink: 1.0,
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(clip).add_child(words);
    clip
}

/// A glyph from the icon face.
pub(crate) fn glyph(
    commands: &mut Commands,
    kit: Kit,
    mark: char,
    size: f32,
    ink: Color,
) -> Entity {
    commands
        .spawn((
            Text::new(mark.to_string()),
            icon_tf(kit.fonts, size),
            TextColor(ink),
            // A mark is never squeezed: the words beside it give way.
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A plain label, muted or not.
pub(crate) fn words(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    size: f32,
    ink: Color,
) -> Entity {
    controls::label(commands, kit, text, size, ink)
}

/// A square button that carries a glyph: the row's `⋯`, the gear.
pub(crate) fn icon_button(
    commands: &mut Commands,
    kit: Kit,
    mark: char,
    on: bool,
    action: impl Bundle,
) -> Entity {
    let side = kit
        .m
        .control
        .min(kit.m.scaled(36.0))
        .max(kit.m.scaled(28.0));
    let face = commands
        .spawn((
            Role::Button,
            Node {
                width: px_fixed(side),
                height: px_fixed(side),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(if on {
                tokens::SELECTED
            } else {
                tokens::CONTROL
            }),
            BorderColor::all(if on { tokens::ACCENT } else { tokens::BORDER }),
            crate::ambience::Feel::new(if on {
                tokens::SELECTED
            } else {
                tokens::CONTROL
            }),
        ))
        .id();
    let mark = glyph(commands, kit, mark, kit.m.small, tokens::INK);
    commands.entity(face).add_child(mark);
    controls::hit(commands, kit, face, action)
}

/// The least of buttons: words only, at the caption's size in the accent,
/// in a hit area a finger can take ("— none · choose", "Collapse all",
/// "Clear", "Retry").
pub(crate) fn link(commands: &mut Commands, kit: Kit, text: &str, action: impl Bundle) -> Entity {
    let face = commands
        .spawn((
            Node {
                min_height: px_fixed(kit.m.scaled(28.0)),
                padding: UiRect::axes(kit.m.px(4.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::ACCENT.with_alpha(0.5)),
            BackgroundColor(Color::NONE),
        ))
        .id();
    let words = controls::label(commands, kit, text, kit.m.small, tokens::ACCENT);
    commands.entity(face).add_child(words);
    controls::hit(commands, kit, face, action)
}

/// A flexible gap that pushes what follows to the far end.
pub(crate) fn spring(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A row of children, centred, with the kit's gap.
pub(crate) fn line(commands: &mut Commands, kit: Kit, children: &[Entity]) -> Entity {
    let id = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(id).add_children(children);
    id
}

/// A small-capitals heading inside a panel: muted, letter-spaced by its case.
pub(crate) fn caption(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    let upper = text.to_uppercase();
    commands
        .spawn((
            Text::new(upper),
            tf_bold(kit.fonts, kit.m.small * 0.92),
            TextColor(tokens::MUTED),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id()
}

/// The builder's panel: translucent over the painting, its contents a column
/// that may not grow past the window.
pub(crate) fn panel(commands: &mut Commands, kit: Kit, grow: f32, width: Option<f32>) -> Entity {
    commands
        .spawn((
            Role::Panel,
            Node {
                flex_grow: grow,
                flex_shrink: if width.is_some() { 0.0 } else { 1.0 },
                flex_basis: width.map_or(px_fixed(0.0), px_fixed),
                width: width.map_or(Val::Auto, px_fixed),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id()
}

/// The colours the identity filter offers, and the pips it counts.
pub(crate) const COLORS: [(char, Phrase); 6] = [
    ('W', Phrase::ColorWhite),
    ('U', Phrase::ColorBlue),
    ('B', Phrase::ColorBlack),
    ('R', Phrase::ColorRed),
    ('G', Phrase::ColorGreen),
    ('C', Phrase::ColorColourless),
];

/// The card types worth a chip of their own, each with the word it is drawn
/// as. The key stays English: it is matched against a printed type line and
/// is what `BuildPress::SetKind` carries, so translating it would filter for a
/// word no card is printed with.
pub(crate) const KINDS: [(&str, Phrase); 7] = [
    ("Creature", Phrase::KindCreature),
    ("Instant", Phrase::KindInstant),
    ("Sorcery", Phrase::KindSorcery),
    ("Artifact", Phrase::KindArtifact),
    ("Enchantment", Phrase::KindEnchantment),
    ("Planeswalker", Phrase::KindPlaneswalker),
    ("Land", Phrase::KindLand),
];

/// The chips narrowing the pool, in the player's own words, or `None`.
///
/// The renderer's half of [`DeckBuilder::chips_in_force`]: the model says
/// *what* filters and this says what it is called, because both words live in
/// `COLORS` and `KINDS` above — and `KINDS`' key is English on purpose, so it
/// is the only one of the four that must be translated rather than printed.
///
/// The curve bucket is two sentences and not one, because its last bucket is
/// "that or more"; `CURVE_BUCKETS` is where that is decided and this reads it
/// rather than repeating the number.
pub(crate) fn chips_in_words(deck: &DeckBuilder, lang: Lang) -> Option<String> {
    use baylee_client_core::deckbuilder::Chip;

    let parts: Vec<String> = deck
        .chips_in_force()
        .into_iter()
        .map(|chip| match chip {
            Chip::Colors(letters) => letters
                .iter()
                .map(|letter| {
                    COLORS
                        .iter()
                        .find(|(c, _)| c == letter)
                        .map_or_else(|| letter.to_string(), |(_, p)| p.text(lang).to_string())
                })
                .collect::<Vec<_>>()
                .join("/"),
            Chip::Kind(kind) => KINDS
                .iter()
                .find(|(k, _)| *k == kind)
                .map_or_else(|| kind.to_string(), |(_, p)| p.text(lang).to_string()),
            Chip::Cmc(cmc) => {
                let last = cmc as usize == CURVE_BUCKETS - 1;
                let phrase = if last {
                    Phrase::FilterChipCmcUp
                } else {
                    Phrase::FilterChipCmc
                };
                phrase.fill(lang, &[&cmc.to_string()])
            }
            Chip::PlayableOnly => Phrase::FilterChipPlayable.text(lang).to_string(),
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The picker's chosen finish as an image treatment.
pub(crate) fn treatment(finish: Finish) -> FinishTreatment {
    match finish {
        Finish::Normal => FinishTreatment::Plain,
        Finish::Foil => FinishTreatment::Foil,
        Finish::Etched => FinishTreatment::Etched,
        Finish::Holographic => FinishTreatment::Holographic,
        Finish::Glitter => FinishTreatment::Glitter,
        Finish::Galaxy => FinishTreatment::Galaxy,
    }
}

/// What a list says about a card the engine does not play as printed.
pub(crate) fn coverage_mark(coverage: Coverage) -> Option<(Phrase, Color)> {
    match coverage {
        Coverage::Implemented => None,
        Coverage::Partial => Some((Phrase::CoveragePartial, tokens::GOLD)),
        Coverage::Unimplemented => Some((Phrase::CoverageStub, tokens::DANGER)),
    }
}

/// The colour a mana symbol's disc is drawn in. Muted rather than saturated:
/// these sit next to body text, and a full-strength red would shout over it.
pub(crate) fn mana_tone(letter: char) -> Color {
    match letter {
        'W' => Color::srgb(0.93, 0.90, 0.78),
        'U' => Color::srgb(0.42, 0.65, 0.88),
        'B' => Color::srgb(0.62, 0.56, 0.68),
        'R' => Color::srgb(0.88, 0.48, 0.42),
        'G' => Color::srgb(0.46, 0.74, 0.52),
        _ => Color::srgb(0.72, 0.72, 0.70),
    }
}

/// Icon-face code points the builder draws (`fa-solid-900`, read out of its
/// cmap): the interface faces have no `✎ ⋯ ✓ ● ▾ ▸ ⚠`.
pub(crate) mod mark {
    /// `pen`: rename.
    pub(crate) const PEN: char = '\u{f304}';
    /// `ellipsis`: a row's or the header's menu.
    pub(crate) const MORE: char = '\u{f141}';
    /// `check`: saved.
    pub(crate) const CHECK: char = '\u{f00c}';
    /// `circle`: unsaved.
    pub(crate) const DOT: char = '\u{f111}';
    /// `caret-down`: an open section, a disclosure.
    pub(crate) const OPEN: char = '\u{f0d7}';
    /// `caret-right`: a folded section.
    pub(crate) const FOLDED: char = '\u{f0da}';
    /// `triangle-exclamation`: a warning.
    pub(crate) const WARN: char = '\u{f071}';
    /// `circle-question`: the search syntax.
    pub(crate) const HELP: char = '\u{f059}';
    /// `sort`: the pool's order.
    pub(crate) const SORT: char = '\u{f0dc}';
    /// `angle-left`: back.
    pub(crate) const BACK: char = '\u{f104}';
    /// `xmark`: remove, close.
    pub(crate) const CLOSE: char = '\u{f00d}';
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_follows_the_size_class_and_a_narrow_phone_takes_one_pane() {
        assert_eq!(Layout::of(Frame::Wide, 1920.0), Layout::Columns);
        assert_eq!(Layout::of(Frame::Vast, 2560.0), Layout::Columns);
        assert_eq!(Layout::of(Frame::Narrow, 960.0), Layout::Single);
        assert_eq!(Layout::of(Frame::Compact, 700.0), Layout::Single);
        assert_eq!(Layout::of(Frame::Phone, 844.0), Layout::Rail);
        assert_eq!(Layout::of(Frame::Phone, 920.0), Layout::Rail);
        assert_eq!(Layout::of(Frame::Phone, 640.0), Layout::PhoneSingle);
    }

    #[test]
    fn every_builder_stop_and_sheet_stop_is_named_once() {
        for order in [&BUILDER_ORDER, &BUILDER_SHEET_ORDER] {
            let mut seen = std::collections::BTreeSet::new();
            for id in order.stops {
                assert!(seen.insert(*id), "{id} twice in {}", order.name);
            }
        }
    }
}
