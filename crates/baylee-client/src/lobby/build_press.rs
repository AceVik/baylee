//! The deck builder's presses: what each control `crate::buildui` draws
//! does when it is clicked.

use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// A control of the deck builder, its printing picker, and its import and
/// export dialogs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BuildPress {
    /// Open the import dialog over the builder.
    OpenImport,
    /// Open the export dialog over the builder.
    OpenExport,
    /// Put the import or export dialog away.
    TransferClose,
    /// Nothing: carried by the dialog's own panel, so a tap inside it is not
    /// also a tap on the shade behind it.
    TransferNothing,
    /// Read the clipboard into the import box.
    ImportPaste,
    /// Empty the import box.
    ImportClear,
    /// Take the read deck into the builder.
    ImportTake,
    /// Show the export in this format.
    ExportFormat(baylee_client_core::deckbuilder::transfer::FormatId),
    /// Put the export on the clipboard.
    ExportCopy,
    /// Save the export as a file.
    ExportSave,
    /// Leave the builder for the tables.
    CloseBuilder,
    /// Save whatever the builder holds.
    SaveDeck,
    /// Put the caret in one of the builder's boxes.
    FocusBuild(BuildField),
    /// Build into the main deck or the sideboard.
    SetZone(Zone),
    /// Turn one colour of the identity filter on or off.
    ToggleColor(char),
    /// Show only one card type, or all of them again.
    SetKind(Option<&'static str>),
    /// Show only one mana value, or all of them again. Doubles as the click
    /// target on a curve bar.
    SetCmc(u32),
    /// Hide the cards the engine does not play properly, or stop hiding them.
    TogglePlayable,
    /// Change what the results are sorted by.
    CycleSort,
    /// Drop every filter at once.
    ClearFilters,
    /// Empty both zones.
    ClearDeck,
    /// Show the pool or the deck, on a screen with room for one.
    ShowPane(Pane),
    /// Read a card in full, by its slot in the pool.
    Inspect(usize),
    /// Open the printing picker on a pool card, by its slot.
    PickPrint(usize),
    PickRowPrint(usize),
    /// Open the printing picker on a commander's own deck row, by the
    /// commander's slot: its picture in the commander box does what a deck
    /// row's picture does, with either list open (#255).
    PickCommanderPrint(usize),
    /// Move the picker's carousel.
    PickerStep(i32),
    /// Jump the carousel to one printing, by its place in the visible list.
    PickerGo(usize),
    /// Limit the carousel to one language, by its place in the picker's list,
    /// or `None` for all of them. An index rather than the code itself
    /// because a `Press` is `Copy` and a language code is a `String`.
    PickerLang(Option<usize>),
    /// Choose a finish for the printing the carousel is on.
    PickerFinish(Finish),
    PickerRefresh,
    PickerForceFinish,
    PickerSet(Option<usize>),
    /// Add the picked printing to the deck.
    PickerConfirm,
    /// Put the picker away, adding nothing.
    PickerClose,
    /// Take one copy out of a named row of the deck list.
    RemoveRow(usize),
    AddRow(usize),
    /// Move one copy of a named row to the other list — deck to sideboard,
    /// or back. The row keeps the printing it was chosen with.
    MoveRow(usize),
    /// Add one copy of a pool card to a named list, whichever one is open.
    AddCardTo(usize, Zone),
    RemoveCardFrom(usize, Zone),
    /// Make a pool card the deck's commander.
    SetCommander(usize),
    ChooseCommander(bool),
    CancelCommanderPick,
    AddPartner(usize),
    RemoveCommander(usize),
    ToggleStatistics,
    ToggleDeckActions,
    CompleteSearch(usize),
    /// Take the commander mark off, leaving the card in the deck.
    ClearCommander,
    /// Put it away again.
    CloseCard,
    /// Show or hide the filter chips on a narrow screen.
    ToggleFilters,
    /// Open the filter-string builder on what the search box holds, or shut
    /// it. The cogwheel inside the box.
    ToggleFilterPanel,
}

impl BuildPress {
    /// What a click on this control does.
    #[allow(clippy::too_many_lines)] // one flat match, read top to bottom
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            scrolled,
            mailbox,
            ..
        } = cx;
        match self {
            // A press on the transfer dialog's panel only keeps it from
            // reaching the shade behind.
            BuildPress::TransferNothing => {}
            BuildPress::OpenImport => {
                scrolled.set(List::Transfer, 0.0);
                state.lobby.builder_mut().open_import();
            }
            BuildPress::OpenExport => {
                scrolled.set(List::Transfer, 0.0);
                state.lobby.builder_mut().open_export();
            }
            BuildPress::TransferClose => state.lobby.builder_mut().close_transfer(),
            BuildPress::ImportPaste => state
                .transfer_asks
                .push(crate::buildui::transfer::Ask::Paste),
            BuildPress::ImportClear => state.lobby.builder_mut().import_clear(),
            BuildPress::ImportTake => {
                let lang = state.lobby.lang();
                state.lobby.builder_mut().import_confirm(lang);
                scrolled.set(List::Deck, 0.0);
            }
            BuildPress::ExportFormat(format) => state.lobby.builder_mut().export_choose(format),
            BuildPress::ExportCopy => state
                .transfer_asks
                .push(crate::buildui::transfer::Ask::Copy),
            BuildPress::ExportSave => state
                .transfer_asks
                .push(crate::buildui::transfer::Ask::Save),
            BuildPress::CloseBuilder => {
                if state.lobby.builder().dirty() && !state.confirm_leave {
                    state.confirm_leave = true;
                    state.lobby.tell_refusal(Phrase::UnsavedChanges, &[]);
                } else {
                    state.confirm_leave = false;
                    state.hub = Hub::Decks;
                    let request = state.lobby.close_builder();
                    dispatch(state, mailbox, request);
                }
            }
            BuildPress::SaveDeck => {
                let request = state.lobby.save_deck();
                dispatch(state, mailbox, request);
            }
            BuildPress::FocusBuild(field) => {
                state.completion_hidden = false;
                state.completion = None;
                let deck = state.lobby.builder_mut();
                // A tap in the search box shuts the builder and takes the
                // caret, which is the way back out of it — the same rule the
                // zone browser's box follows.
                if field == BuildField::Search {
                    deck.close_panel();
                }
                deck.focus_on(field);
            }
            BuildPress::PickRowPrint(at) => {
                scrolled.set(List::PickerPanel, 0.0);
                let zone = state.lobby.builder().zone();
                let request = state.lobby.builder_mut().open_row_picker(at, zone);
                dispatch(state, mailbox, request);
            }
            BuildPress::PickCommanderPrint(slot) => {
                let Some(at) = state.lobby.builder().commander_row(slot) else {
                    return;
                };
                scrolled.set(List::PickerPanel, 0.0);
                let request = state.lobby.builder_mut().open_row_picker(at, Zone::Main);
                dispatch(state, mailbox, request);
            }
            BuildPress::PickPrint(slot) => {
                scrolled.set(List::PickerPanel, 0.0);
                let zone = state.lobby.builder().zone();
                let request = state.lobby.builder_mut().open_picker(slot, zone);
                dispatch(state, mailbox, request);
            }
            BuildPress::PickerStep(by) => {
                state.lobby.builder_mut().picker_close_sets();
                state.lobby.builder_mut().picker_step(by);
            }
            BuildPress::PickerGo(at) => state.lobby.builder_mut().picker_go(at),
            BuildPress::PickerLang(which) => {
                // The list the index came from is the one being read here, so
                // a stale index simply selects nothing rather than panicking.
                let lang = which.and_then(|i| {
                    state
                        .lobby
                        .builder()
                        .picker()
                        .and_then(|p| p.langs().get(i).cloned())
                });
                state.lobby.builder_mut().picker_set_lang(lang.as_deref());
            }
            BuildPress::PickerRefresh => {
                let request = state.lobby.builder_mut().refresh_printings();
                let card = state
                    .lobby
                    .builder()
                    .picker()
                    .and_then(|p| state.lobby.builder().card(p.slot()))
                    .cloned();
                if request.is_some()
                    && !cfg!(test)
                    && let Some(card) = card.filter(|c| uuid::Uuid::parse_str(&c.oracle_id).is_ok())
                {
                    let fallback = state
                        .lobby
                        .builder()
                        .picker()
                        .map(|p| p.all_printings().to_vec())
                        .unwrap_or_default();
                    super::print_catalog::fetch(
                        card.index,
                        &card.oracle_id,
                        fallback,
                        state.gateway_epoch,
                        mailbox,
                    );
                } else {
                    dispatch(state, mailbox, request);
                }
            }
            BuildPress::PickerForceFinish => state.lobby.builder_mut().picker_force_finish(),
            BuildPress::PickerSet(at) => state.lobby.builder_mut().picker_set_set(at),
            BuildPress::PickerFinish(finish) => state.lobby.builder_mut().picker_set_finish(finish),
            BuildPress::PickerConfirm => {
                if !state.lobby.room_confirm_print() && !state.lobby.builder_mut().picker_confirm()
                {
                    state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
                }
            }
            BuildPress::PickerClose => state.lobby.room_close_print(),
            BuildPress::AddRow(at) => {
                let zone = state.lobby.builder().zone();
                if let Some(entry) = state.lobby.builder().entries(zone).get(at).cloned()
                    && !state
                        .lobby
                        .builder_mut()
                        .add_print(entry.slot, zone, entry.print)
                {
                    state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
                }
            }
            BuildPress::RemoveRow(at) => {
                let zone = state.lobby.builder().zone();
                state.lobby.builder_mut().remove_at(at, zone);
            }
            BuildPress::MoveRow(at) => {
                let from = state.lobby.builder().zone();
                let to = match from {
                    Zone::Main => Zone::Side,
                    Zone::Side => Zone::Main,
                };
                state.lobby.builder_mut().move_entry(at, from, to);
            }
            BuildPress::RemoveCardFrom(slot, zone) => {
                state.lobby.builder_mut().remove(slot, zone);
            }
            BuildPress::AddCardTo(slot, zone) => {
                state.lobby.builder_mut().add(slot, zone);
            }
            BuildPress::ChooseCommander(partner) => {
                state.commander_pick = Some(partner);
                state.pane = Pane::Cards;
                state.lobby.builder_mut().clear_filters();
                state.lobby.builder_mut().set_text("is:commander");
                state.lobby.builder_mut().focus_on(BuildField::Search);
                scrolled.set(List::Pool, 0.0);
            }
            BuildPress::CancelCommanderPick => {
                state.commander_pick = None;
                state.lobby.builder_mut().set_text("");
            }
            BuildPress::SetCommander(slot) | BuildPress::AddPartner(slot) => {
                let accepted = if matches!(self, BuildPress::AddPartner(_)) {
                    state.lobby.builder_mut().add_partner(slot)
                } else {
                    state.lobby.builder_mut().set_commander(slot)
                };
                if accepted {
                    state.commander_pick = None;
                    state.pane = Pane::Deck;
                    state.lobby.builder_mut().set_text("");
                }
            }
            BuildPress::RemoveCommander(slot) => state.lobby.builder_mut().remove_commander(slot),
            BuildPress::CompleteSearch(slot) => {
                crate::buildui::autocomplete::choose(state, slot);
                scrolled.set(List::Pool, 0.0);
            }
            BuildPress::ToggleDeckActions => state.deck_actions_open = !state.deck_actions_open,
            BuildPress::ToggleStatistics => state.stats_open = !state.stats_open,
            BuildPress::ClearCommander => state.lobby.builder_mut().clear_commander(),
            BuildPress::SetZone(zone) => state.lobby.builder_mut().set_zone(zone),
            BuildPress::ToggleColor(color) => state.lobby.builder_mut().toggle_color(color),
            BuildPress::SetKind(kind) => {
                let builder = state.lobby.builder_mut();
                // A second tap on the open chip is how it is closed again;
                // without it a filter can only be dropped from "Clear".
                let same = builder.kind() == kind;
                builder.set_kind(if same { None } else { kind });
            }
            BuildPress::SetCmc(cmc) => state.lobby.builder_mut().set_cmc(Some(cmc)),
            BuildPress::TogglePlayable => state.lobby.builder_mut().toggle_playable_only(),
            BuildPress::CycleSort => state.lobby.builder_mut().cycle_sort(),
            BuildPress::ClearFilters => state.lobby.builder_mut().clear_filters(),
            BuildPress::ClearDeck => {
                state.confirmation = Some(confirm::Destructive::Clear(
                    state.lobby.builder().editing().map(str::to_owned),
                ));
            }
            BuildPress::ShowPane(pane) => state.pane = pane,
            BuildPress::Inspect(slot) => state.lobby.builder_mut().inspect(slot),
            BuildPress::CloseCard => state.lobby.builder_mut().stop_inspecting(),
            BuildPress::ToggleFilters => state.filters_open = !state.filters_open,
            BuildPress::ToggleFilterPanel => state.lobby.builder_mut().toggle_panel(),
        }
    }
}
