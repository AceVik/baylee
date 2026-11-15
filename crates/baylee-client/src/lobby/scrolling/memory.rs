use bevy::prelude::Resource;

/// The lists that remember where they were left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum List {
    /// The searchable card pool.
    Pool,
    /// The deck check's sheet.
    Check,
    /// The deck being built.
    Deck,
    /// The tables and decks on the lobby screen.
    Table,
    /// Tower shop details, independent of the stock/inventory surface.
    EchoesInspector,
    /// The saved gateways on the front door.
    Gateways,
    /// Gateway information and discovery actions.
    Discovery,
    Games,
    RoomCards,
    Library,
    PickerSets,
    PickerPanel,
    Settings,
    /// The import or export dialog.
    Transfer,
    /// The terms sheet's text (WP1).
    Terms,
    /// The About sheet's text (WP1).
    About,
    /// Baylee's story, independent of its photograph selection.
    Memorial,
    /// A language-model profile's sheet (WP5).
    ProfileSheet,
    /// The settings screen's section list (WP5).
    SettingsNav,
    /// The Replays tab's list.
    Replays,
    /// Play's deck picker (Change deck).
    DeckPicker,
    /// The Create table / Edit rules sheet, independent of the deck check.
    TableRules,
}

/// Where each list was left, across rebuilds of the node tree.
///
/// Deliberately not part of [`LobbyState`]: the tree is rebuilt whenever that
/// changes, so keeping the offsets there would rebuild sixty rows on every
/// notch of the wheel. Kept apart, adding a card rebuilds the list *and*
/// leaves it where the player was reading — which is the only reason they
/// scrolled there.
#[derive(Resource, Default)]
pub(crate) struct Scrolled {
    pool: f32,
    check: f32,
    deck: f32,
    table: f32,
    echoes_inspector: f32,
    gateways: f32,
    discovery: f32,
    games: f32,
    room_cards: f32,
    library: f32,
    picker_panel: f32,
    settings: f32,
    transfer: f32,
    terms: f32,
    about: f32,
    memorial: f32,
    profile_sheet: f32,
    settings_nav: f32,
    replays: f32,
    deck_picker: f32,
    table_rules: f32,
}

impl Scrolled {
    pub(crate) fn get(&self, list: List) -> f32 {
        match list {
            List::Pool => self.pool,
            List::Check => self.check,
            List::Deck => self.deck,
            List::Table => self.table,
            List::EchoesInspector => self.echoes_inspector,
            List::Gateways => self.gateways,
            List::Discovery => self.discovery,
            List::Games => self.games,
            List::RoomCards => self.room_cards,
            List::PickerSets => 0.0,
            List::Library => self.library,
            List::PickerPanel => self.picker_panel,
            List::Settings => self.settings,
            List::Transfer => self.transfer,
            List::Terms => self.terms,
            List::About => self.about,
            List::Memorial => self.memorial,
            List::ProfileSheet => self.profile_sheet,
            List::SettingsNav => self.settings_nav,
            List::Replays => self.replays,
            List::DeckPicker => self.deck_picker,
            List::TableRules => self.table_rules,
        }
    }

    pub(crate) fn set(&mut self, list: List, at: f32) {
        match list {
            List::Pool => self.pool = at,
            List::Check => self.check = at,
            List::Deck => self.deck = at,
            List::Table => self.table = at,
            List::EchoesInspector => self.echoes_inspector = at,
            List::Gateways => self.gateways = at,
            List::Discovery => self.discovery = at,
            List::Games => self.games = at,
            List::RoomCards => self.room_cards = at,
            List::PickerSets => {}
            List::Library => self.library = at,
            List::PickerPanel => self.picker_panel = at,
            List::Settings => self.settings = at,
            List::Transfer => self.transfer = at,
            List::Terms => self.terms = at,
            List::About => self.about = at,
            List::Memorial => self.memorial = at,
            List::ProfileSheet => self.profile_sheet = at,
            List::SettingsNav => self.settings_nav = at,
            List::Replays => self.replays = at,
            List::DeckPicker => self.deck_picker = at,
            List::TableRules => self.table_rules = at,
        }
    }
}
