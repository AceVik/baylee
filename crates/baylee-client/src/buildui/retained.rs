//! Keep the unaffected half of the editor alive across input and server replies.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::{
    deckbuilder::transfer::Transfer,
    deckbuilder::{Entry, Sort},
    filterdialog::FilterPanel,
    textbuf::TextBuffer,
};

/// What the import or export dialog draws from: the dialog itself, and the
/// two things its report reads from the builder live — the cards the pool
/// lacks and whether the pool has arrived.
#[derive(PartialEq, Eq)]
struct TransferKey {
    transfer: Transfer,
    missing: Vec<String>,
    loaded: bool,
    busy: bool,
}

fn transfer_key(state: &LobbyState) -> Option<TransferKey> {
    let deck = state.lobby.builder();
    deck.transfer().map(|transfer| TransferKey {
        transfer: transfer.clone(),
        missing: deck.missing().to_vec(),
        loaded: deck.loaded(),
        busy: state.lobby.busy(),
    })
}

#[derive(PartialEq, Eq)]
struct DeckKey {
    statistics: bool,
    actions: bool,
    main: Vec<Entry>,
    side: Vec<Entry>,
    name: TextBuffer,
    zone: Zone,
    commanders: Vec<usize>,
    missing: Vec<String>,
    focused: bool,
    pool: u64,
    cmc: Option<u32>,
}
#[derive(PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Independent retained editor view flags.
struct PoolKey {
    completion: Option<usize>,
    completion_hidden: bool,
    commander_pick: Option<bool>,
    commanders: Vec<usize>,
    revision: u64,
    text: TextBuffer,
    results: Vec<usize>,
    colors: Vec<char>,
    kind: Option<String>,
    cmc: Option<u32>,
    playable: bool,
    sort: Sort,
    zone: Zone,
    focused: bool,
    filters: bool,
    panel: Option<FilterPanel>,
    inspecting: Option<usize>,
    inspected_counts: Option<(u16, u16)>,
}
#[derive(PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent editor states used as a render key
struct BarKey {
    editing: Option<String>,
    dirty: bool,
    saveable: bool,
    busy: bool,
    status: String,
    leaving: bool,
}

pub(crate) struct Retained {
    root: Entity,
    body: Entity,
    bar: Entity,
    deck: Option<Entity>,
    pool: Option<Entity>,
    picker: Option<Entity>,
    transfer: Option<Entity>,
    deck_key: DeckKey,
    pool_key: PoolKey,
    bar_key: BarKey,
    picker_key: Option<Picker>,
    transfer_key: Option<TransferKey>,
}
impl Retained {
    /// `drawn` is the deck list, the pool, the printing picker and the
    /// import or export dialog, each as far as it was drawn.
    pub(super) fn new(
        state: &LobbyState,
        root: Entity,
        body: Entity,
        bar: Entity,
        drawn: [Option<Entity>; 4],
    ) -> Self {
        let [deck, pool, picker, transfer] = drawn;
        let (deck_key, pool_key, bar_key) = keys(state);
        Self {
            root,
            body,
            bar,
            deck,
            pool,
            picker,
            transfer,
            deck_key,
            pool_key,
            bar_key,
            picker_key: state.lobby.builder().picker().cloned(),
            transfer_key: transfer_key(state),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn patch(
        &mut self,
        commands: &mut Commands,
        state: &LobbyState,
        fonts: &UiFonts,
        metrics: Metrics,
        scrolled: &Scrolled,
        assets: Option<&AssetServer>,
        cards: Option<&mut UiCards<'_>>,
    ) {
        let (deck_key, pool_key, bar_key) = keys(state);
        if self.deck_key != deck_key {
            if let Some(old) = self.deck.take() {
                commands.entity(old).despawn();
            }
            let deck = deck_panel(commands, state, fonts, metrics, scrolled);
            commands.entity(self.body).insert_children(0, &[deck]);
            self.deck = Some(deck);
            self.deck_key = deck_key;
        }
        if self.pool_key != pool_key {
            if let Some(old) = self.pool.take() {
                commands.entity(old).despawn();
            }
            let pool = pool_panel(commands, state, fonts, metrics, scrolled);
            commands.entity(self.body).add_child(pool);
            self.pool = Some(pool);
            self.pool_key = pool_key;
        }
        if self.bar_key != bar_key {
            commands.entity(self.bar).despawn();
            self.bar = build_bar(commands, state, fonts, metrics);
            commands.entity(self.root).insert_children(0, &[self.bar]);
            self.bar_key = bar_key;
        }
        let picker = state.lobby.builder().picker();
        if self.picker_key.as_ref() != picker {
            if let Some(old) = self.picker.take() {
                commands.entity(old).despawn();
            }
            if let Some(picker) = picker {
                let dialog = printing_picker(
                    commands,
                    fonts,
                    metrics,
                    state.lobby.lang(),
                    state.lobby.builder(),
                    picker,
                    assets,
                    cards,
                    scrolled,
                );
                commands.entity(self.root).add_child(dialog);
                self.picker = Some(dialog);
            }
            self.picker_key = picker.cloned();
        }
        let transfer = transfer_key(state);
        if self.transfer_key != transfer {
            if let Some(old) = self.transfer.take() {
                commands.entity(old).despawn();
            }
            if let Some(open) = state.lobby.builder().transfer() {
                let dialog = super::transfer::transfer_dialog(
                    commands,
                    fonts,
                    metrics,
                    state.lobby.lang(),
                    state.lobby.builder(),
                    open,
                    scrolled,
                );
                commands.entity(self.root).add_child(dialog);
                self.transfer = Some(dialog);
            }
            self.transfer_key = transfer;
        }
    }
}

fn keys(state: &LobbyState) -> (DeckKey, PoolKey, BarKey) {
    let deck = state.lobby.builder();
    (
        DeckKey {
            statistics: state.stats_open,
            actions: state.deck_actions_open,
            main: deck.entries(Zone::Main).to_vec(),
            side: deck.entries(Zone::Side).to_vec(),
            name: deck.buffer(BuildField::Name).clone(),
            zone: deck.zone(),
            commanders: deck.commanders().to_vec(),
            missing: deck.missing().to_vec(),
            focused: deck.focus() == BuildField::Name,
            pool: deck.pool_revision(),
            cmc: deck.cmc(),
        },
        PoolKey {
            completion: state.completion,
            completion_hidden: state.completion_hidden,
            commander_pick: state.commander_pick,
            commanders: deck.commanders().to_vec(),
            revision: deck.pool_revision(),
            text: deck.buffer(BuildField::Search).clone(),
            results: deck.results().to_vec(),
            colors: deck.colors().to_vec(),
            kind: deck.kind().map(str::to_owned),
            cmc: deck.cmc(),
            playable: deck.playable_only(),
            sort: deck.sort(),
            zone: deck.zone(),
            focused: deck.focus() == BuildField::Search && deck.picker().is_none(),
            filters: state.filters_open,
            panel: deck.panel().cloned(),
            inspecting: deck.inspecting(),
            inspected_counts: deck.inspecting().map(|slot| {
                (
                    deck.count_of(slot, Zone::Main),
                    deck.count_of(slot, Zone::Side),
                )
            }),
        },
        BarKey {
            editing: deck.editing().map(str::to_owned),
            dirty: deck.dirty(),
            saveable: deck.saveable(),
            busy: state.lobby.busy(),
            status: state.lobby.status().into(),
            leaving: state.confirm_leave,
        },
    )
}
