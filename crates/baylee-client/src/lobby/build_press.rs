//! The deck builder's presses: what each control `crate::buildui` draws
//! does when it is clicked.

use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::buildui::{BuildMenu, DeckTab, Pane};

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
    /// Show the pool, the deck or the numbers, on a frame with room for one.
    SetPane(Pane),
    /// Show the main deck, the sideboard or the numbers on the deck side.
    SetTab(DeckTab),
    /// Section the deck list by type, mana value or colour.
    SetGrouping(baylee_client_core::deckbuilder::Grouping),
    /// Fold one section of the deck list, or open it again.
    ToggleSection(baylee_client_core::deckbuilder::SectionKey),
    /// Fold every section, or open them all.
    CollapseAll,
    /// A pool row's `+`: one copy to the list the deck side shows, or with
    /// Shift (`true`) to the other one.
    AddFromPool(usize, bool),
    /// Take a whole row out of the shown list.
    RemoveAll(usize),
    /// Open the header's menu, or shut it.
    ToggleHeaderMenu,
    /// Open a pool row's menu, by its card's slot.
    PoolMenu(usize),
    /// Open a deck row's menu, by its index in the shown list.
    DeckMenu(usize),
    /// Shut the open menu.
    CloseMenu,
    /// Rename the deck: the title becomes its box (F2, the pen).
    Rename,
    /// The settings screen, from the header's menu.
    BuilderSettings,
    /// Open the Filters rail (a sheet on a phone), or shut it.
    ToggleRail,
    /// Open the search syntax popover, or shut it.
    ToggleSyntax,
    /// Insert one of the popover's examples into the search.
    InsertSyntax(usize),
    /// Drop the mana value chip.
    ClearCmc,
    /// Light one type on the Stats curve, or none.
    SetLit(Option<baylee_client_core::deckbuilder::Group>),
    /// Draw a sample hand of seven.
    DrawSeven,
    /// The phone's Stats sheet.
    OpenStatsSheet,
    /// Put the phone's Stats sheet away.
    CloseStatsSheet,
    /// "Discard changes?": stay in the builder.
    KeepEditing,
    /// "Discard changes?": leave without saving, the draft forgotten.
    DiscardAndLeave,
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
    /// Make a pool card the deck's commander.
    SetCommander(usize),
    ChooseCommander(bool),
    CancelCommanderPick,
    AddPartner(usize),
    RemoveCommander(usize),
    /// Take the commander mark off, leaving the card in the deck.
    ClearCommander,
    /// Put it away again.
    CloseCard,
    /// Open the filter-string builder on what the search box holds, or shut
    /// it. The cogwheel inside the box.
    ToggleFilterPanel,
}

/// Leaves the builder for Decks, the question answered.
fn leave_the_builder(state: &mut ResMut<LobbyState>, mailbox: &Mailbox) {
    if state.confirm_leave {
        state.confirm_leave = false;
    }
    state.hub = Hub::Decks;
    let request = state.lobby.close_builder();
    dispatch(state, mailbox, request);
}

impl BuildPress {
    /// What a click on this control does.
    #[allow(clippy::too_many_lines)] // one flat match, read top to bottom
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs: cx_prefs,
            scrolled,
            mailbox,
            settings: cx_settings,
        } = cx;
        // A press on anything but a menu's own opener puts the open menu
        // away; the item pressed then does what it says.
        if state.build.menu.is_some()
            && !matches!(
                self,
                BuildPress::ToggleHeaderMenu | BuildPress::PoolMenu(_) | BuildPress::DeckMenu(_)
            )
        {
            state.build.menu = None;
        }
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
            // Back asks first when there is something unsaved: "Discard
            // changes?", the builder's one confirm (KEYBOARD §2.5).
            BuildPress::CloseBuilder => {
                if state.lobby.builder().dirty() {
                    if !state.confirm_leave {
                        state.confirm_leave = true;
                    }
                } else {
                    leave_the_builder(state, mailbox);
                }
            }
            BuildPress::KeepEditing => {
                if state.confirm_leave {
                    state.confirm_leave = false;
                }
            }
            BuildPress::DiscardAndLeave => {
                let editing = state.lobby.builder().editing().map(str::to_owned);
                crate::buildui::draft::forget(&[editing.as_deref()]);
                leave_the_builder(state, mailbox);
            }
            BuildPress::SaveDeck => {
                let request = state.lobby.save_deck();
                if request.is_some() {
                    state.build.save = crate::buildui::SaveState::Saving;
                }
                dispatch(state, mailbox, request);
            }
            BuildPress::FocusBuild(field) => {
                let deck = state.lobby.builder_mut();
                // A tap in the search box shuts the builder and takes the
                // caret, which is the way back out of it — the same rule the
                // zone browser's box follows.
                if field == BuildField::Search {
                    deck.close_panel();
                }
                deck.focus_on(field);
                crate::buildui::move_nav(state, crate::buildui::Nav::Field);
            }
            BuildPress::Rename => {
                // The name selected whole, as an OS rename has it: typing
                // replaces it, an arrow keeps it.
                state.lobby.builder_mut().focus_on(BuildField::Name);
                state
                    .lobby
                    .builder_mut()
                    .edit_buffer(BuildField::Name, crate::buildui::select_all);
                crate::buildui::move_nav(state, crate::buildui::Nav::Field);
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
            BuildPress::AddCardTo(slot, zone) => {
                state.lobby.builder_mut().add(slot, zone);
            }
            BuildPress::ChooseCommander(partner) => {
                state.commander_pick = Some(partner);
                if state.build.pane != Pane::Pool {
                    state.build.pane = Pane::Pool;
                }
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
                    if state.build.pane == Pane::Pool {
                        state.build.pane = Pane::Deck;
                    }
                    state.lobby.builder_mut().set_text("");
                }
            }
            BuildPress::RemoveCommander(slot) => state.lobby.builder_mut().remove_commander(slot),
            BuildPress::ClearCommander => state.lobby.builder_mut().clear_commander(),
            BuildPress::ToggleColor(color) => state.lobby.builder_mut().toggle_color(color),
            BuildPress::SetKind(kind) => {
                let builder = state.lobby.builder_mut();
                // A second tap on the open chip is how it is closed again;
                // without it a filter can only be dropped from "Clear".
                let same = builder.kind() == kind;
                builder.set_kind(if same { None } else { kind });
            }
            // A second press on the chosen value drops it, as a type chip's does.
            BuildPress::SetCmc(cmc) => {
                let builder = state.lobby.builder_mut();
                let same = builder.cmc() == Some(cmc);
                builder.set_cmc(if same { None } else { Some(cmc) });
            }
            BuildPress::ClearCmc => state.lobby.builder_mut().set_cmc(None),
            BuildPress::TogglePlayable => state.lobby.builder_mut().toggle_playable_only(),
            BuildPress::CycleSort => state.lobby.builder_mut().cycle_sort(),
            BuildPress::ClearFilters => state.lobby.builder_mut().clear_filters(),
            BuildPress::ClearDeck => {
                state.build.menu = None;
                state.confirmation = Some(confirm::Destructive::Clear(
                    state.lobby.builder().editing().map(str::to_owned),
                ));
            }
            BuildPress::SetPane(pane) => {
                if state.build.pane != pane {
                    state.build.pane = pane;
                }
            }
            BuildPress::SetTab(tab) => {
                if state.build.tab != tab {
                    state.build.tab = tab;
                }
                if let Some(zone) = state.build.zone()
                    && state.lobby.builder().zone() != zone
                {
                    state.lobby.builder_mut().set_zone(zone);
                    scrolled.set(List::Deck, 0.0);
                }
            }
            BuildPress::SetGrouping(grouping) => {
                if state.build.grouping != grouping {
                    state.build.grouping = grouping;
                    state.build.collapsed.clear();
                }
            }
            BuildPress::ToggleSection(key) => {
                let folded = &mut state.build.collapsed;
                if let Some(at) = folded.iter().position(|k| *k == key) {
                    folded.remove(at);
                } else {
                    folded.push(key);
                }
            }
            BuildPress::CollapseAll => {
                let deck = state.lobby.builder();
                let zone = state.build.zone().unwrap_or_else(|| deck.zone());
                let keys: Vec<_> = deck
                    .sections(zone, state.build.grouping)
                    .into_iter()
                    .map(|s| s.key)
                    .collect();
                let all =
                    !keys.is_empty() && keys.iter().all(|k| state.build.collapsed.contains(k));
                state.build.collapsed = if all { Vec::new() } else { keys };
            }
            BuildPress::AddFromPool(slot, other) => {
                let shown = state
                    .build
                    .zone()
                    .unwrap_or_else(|| state.lobby.builder().zone());
                let zone = match (shown, other) {
                    (zone, false) => zone,
                    (Zone::Main, true) => Zone::Side,
                    (Zone::Side, true) => Zone::Main,
                };
                if !state.lobby.builder_mut().add(slot, zone) {
                    state.lobby.tell_refusal(Phrase::NoRoomForCopy, &[]);
                }
            }
            BuildPress::RemoveAll(at) => {
                let zone = state.lobby.builder().zone();
                while state
                    .lobby
                    .builder()
                    .entries(zone)
                    .get(at)
                    .is_some_and(|e| e.count > 1)
                {
                    state.lobby.builder_mut().remove_at(at, zone);
                }
                state.lobby.builder_mut().remove_at(at, zone);
            }
            BuildPress::ToggleHeaderMenu => {
                state.build.menu = if state.build.menu == Some(BuildMenu::Header) {
                    None
                } else {
                    Some(BuildMenu::Header)
                };
            }
            BuildPress::PoolMenu(slot) => {
                state.build.menu = if state.build.menu == Some(BuildMenu::Pool(slot)) {
                    None
                } else {
                    Some(BuildMenu::Pool(slot))
                };
            }
            BuildPress::DeckMenu(at) => {
                state.build.menu = if state.build.menu == Some(BuildMenu::Deck(at)) {
                    None
                } else {
                    Some(BuildMenu::Deck(at))
                };
            }
            BuildPress::CloseMenu => {
                if state.build.menu.is_some() {
                    state.build.menu = None;
                }
            }
            BuildPress::BuilderSettings => {
                state.build.menu = None;
                SettingsPress::OpenSettings.handle(Cx {
                    state,
                    prefs: cx_prefs,
                    scrolled,
                    mailbox,
                    settings: cx_settings,
                });
            }
            BuildPress::ToggleRail => state.build.rail = !state.build.rail,
            BuildPress::ToggleSyntax => state.build.syntax = !state.build.syntax,
            BuildPress::InsertSyntax(at) => {
                if let Some((example, _)) = crate::buildui::pool::SYNTAX.get(at) {
                    let deck = state.lobby.builder_mut();
                    deck.close_panel();
                    deck.focus_on(BuildField::Search);
                    deck.edit_buffer(BuildField::Search, |buffer| {
                        let text = buffer.text();
                        let room = !text.is_empty() && !text.ends_with(' ');
                        buffer.insert(&if room {
                            format!(" {example}")
                        } else {
                            (*example).to_string()
                        });
                    });
                    scrolled.set(List::Pool, 0.0);
                }
                state.build.syntax = false;
                crate::buildui::move_nav(state, crate::buildui::Nav::Field);
            }
            BuildPress::SetLit(lit) => {
                if state.build.lit != lit {
                    state.build.lit = lit;
                }
            }
            BuildPress::DrawSeven => {
                state.build.hand = state.lobby.builder().sample_hand(crate::host::fresh_seed());
            }
            BuildPress::OpenStatsSheet => state.build.stats_sheet = true,
            BuildPress::CloseStatsSheet => {
                if state.build.stats_sheet {
                    state.build.stats_sheet = false;
                }
            }
            BuildPress::Inspect(slot) => state.lobby.builder_mut().inspect(slot),
            BuildPress::CloseCard => state.lobby.builder_mut().stop_inspecting(),
            BuildPress::ToggleFilterPanel => state.lobby.builder_mut().toggle_panel(),
        }
    }
}

/// Enter or Space on a focused builder control does what a press on it
/// does (`KEYBOARD.md` §1.2): the Tab walk reaches every control, and this
/// is how the keyboard then uses one. Not on the search, the name, or a
/// list (their keys are `editing::builder_keys`'), not on the card sheet
/// (whose Enter is Add), never on a disabled control.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn activate_focused(
    mut keys: MessageReader<KeyboardInput>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    stops: Query<(
        &crate::shellkit::focus::Stop,
        &Press,
        Has<crate::shellkit::controls::Disabled>,
    )>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    holds: Option<Res<crate::shellkit::KitHolds>>,
) {
    let pressed = keys.read().any(|k| {
        k.state.is_pressed() && !k.repeat && matches!(k.logical_key, Key::Enter | Key::Space)
    });
    if !pressed
        || !matches!(state.lobby.screen(), Screen::Build)
        || holds.is_some_and(|h| h.0)
        || state.lobby.builder().inspecting().is_some()
    {
        return;
    }
    let Some(entity) = focus
        .as_deref()
        .and_then(bevy::input_focus::InputFocus::get)
    else {
        return;
    };
    let Ok((stop, &press, disabled)) = stops.get(entity) else {
        return;
    };
    let ours = stop.table == crate::buildui::BUILDER || stop.table == crate::buildui::BUILDER_SHEET;
    let typed = matches!(stop.id, "search" | "pool" | "deck")
        || (stop.id == "title" && state.build.nav == crate::buildui::Nav::Field);
    if !ours || typed || disabled {
        return;
    }
    let cx = Cx {
        state: &mut state,
        prefs: &mut prefs,
        scrolled: &mut scrolled,
        mailbox: &mailbox,
        settings: &mut settings,
    };
    match press {
        Press::Build(press) => press.handle(cx),
        Press::Library(press) => press.handle(cx),
        Press::Settings(press) => press.handle(cx),
        Press::Header(press) => press.handle(cx),
        Press::Shared(press) => press.handle(cx),
        _ => {}
    }
}
