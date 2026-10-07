//! What a click on a lobby control means: [`Press`], and the way from a
//! clicked entity to the control it belongs to.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// A component whose click means something.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Press {
    Hub(Hub),
    /// A control of the language-model seat's panel on the settings screen.
    Seat(baylee_client_core::llmseat::panel::Act),
    /// A control of a profile's key box on that panel.
    SeatKey(crate::seatpanel::KeyPress),
    /// A language-model control of a chair: the room's index, the chair.
    RoomLlm(usize, u32, crate::tableseats::LlmPress),
    AddGateway,
    SelectGateway(usize),
    /// Asks, in the confirm dialog, whether a saved gateway leaves the list.
    ForgetGateway(usize),
    /// Back from the account form to the gateway form.
    LeaveGateway,
    /// Opens or closes the front door's gear menu.
    FrontMenu,
    BrowseHouse,
    BrowseHistory,
    DeckHistory(usize),
    CloseLibrary,
    RetryLibrary,
    PreviewHouse(usize),
    PreviewVersion(i32),
    CopyHouse(usize),
    RestoreVersion,
    /// Put the caret in this field.
    Focus(Field),
    /// Show a masked field in the clear, or cover it again.
    Reveal(Field),
    /// Swap the form between log-in and sign-up.
    ToggleRegistering,
    /// Send the sign-in form.
    Submit,
    /// Play the house AI in this process, no account needed.
    PlayOffline,
    /// Forget the account.
    SignOut,
    /// Play as a guest (#269): the one kept here, or a new one.
    PlayAsGuest,
    /// Re-read decks and tables.
    Refresh,
    /// Read the table list again for whatever the search box says.
    Search,
    /// Clear a table search and return to its first page.
    ClearSearch,
    /// Step one page through the table list. `true` is forwards.
    Page(bool),
    /// Pick a deck by its index in the list.
    SelectDeck(usize),
    /// Open a new table.
    Host(GameMode),
    /// Open a table with a chosen number of chairs.
    OpenRoom(usize),
    /// Edit the local room draft.
    RoomAdjust(baylee_client_core::lobby::room::Adjustment),
    /// Apply the host draft; true explicitly removes the password.
    SaveRoom(bool),
    /// Apply a chosen deck directly to this seat.
    RoomDeck(usize, u32, usize),
    /// Toggle the deck choices for a seat.
    RoomDeckPicker(u32),
    /// Expand or collapse a seat's optional starting-position editor.
    RoomSetup(u8),
    RoomCardAdd(u8, usize, bool),
    RoomCardRemove(u8, usize),
    RoomCardEdit(u8, usize),
    RoomCardPrint(u8, usize),
    RoomCounterAdd(u8, usize),
    RoomCounterStep(u8, usize, usize, i16),
    /// Sit down at a listed table by its index.
    Join(usize),
    /// Give up a chair. The room outlives it.
    LeaveTable(usize),
    /// Say whether this player is ready at a listed table.
    Ready(usize, bool),
    /// Start a room this account hosts.
    StartRoom(usize),
    /// Hand the room to the player in a chair.
    HandOver(usize, u32),
    /// Make a chair a person's or the AI's.
    SeatKind(usize, u32, SeatKind),
    /// Set an AI chair's difficulty.
    SeatAi(usize, u32, &'static str),
    /// Move a chair onto a side. `0` puts it back on its own.
    SeatTeam(usize, u32, u8),
    /// Leave a finished game.
    Leave,
    /// Play that game again. Beside [`Press::Leave`], because those are the
    /// only two things left to do with a table that is over.
    PlayAgain,
    /// Take the chair kept for this player at a listed rematch room.
    Rematch(usize),
    /// Open the settings screen.
    OpenSettings,
    /// Leave it.
    CloseSettings,
    /// Wait for a key and bind it to this action.
    Rebind(baylee_client_core::prefs::Action),
    /// Put one action back to its default key.
    ResetBinding(baylee_client_core::prefs::Action),
    /// Put every key back.
    ResetAllBindings,
    /// Flip one automation switch.
    ToggleAuto(baylee_client_core::prefs::AutoRule),
    /// Return every card ability to manual responses.
    ResetAbilityOrders,
    /// Forget one ability's policy.
    ForgetAbility(baylee_core::ids::AbilityRef),
    /// Stop the table moving, or let it move again.
    ToggleMotion,
    /// Speak this language from now on.
    PickLang(Lang),
    /// Put a sky behind the table, or let the clock choose one.
    PickSky(baylee_client_core::sky::SkyMode),
    /// Turn the table up, down, or off.
    PickSound(baylee_client_core::cue::Loudness),
    /// Put weather in the air over the table, or take it away.
    PickAtmosphere(baylee_client_core::atmosphere::Atmosphere),
    /// Turn one step of the phase rail red or green.
    ToggleRail(
        baylee_client_core::automation::RailSide,
        baylee_client_core::automation::RailRow,
    ),
    /// Put the whole rail to a preset.
    SetRail(baylee_client_core::automation::RailPreset),
    /// Open the builder on a new deck.
    NewDeck,
    /// Open the builder on a new deck with the import dialog over it.
    ImportDeck,
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
    /// Open the builder on a saved deck, by its index in the list.
    EditDeck(usize),
    /// Throw a saved deck away, by its index in the list.
    DeleteDeck(usize),
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
    ConfirmDestructive,
    CancelDestructive,
    /// Open the confirmation that deletes the account (#292).
    AskToDeleteAccount,
    /// Send the deletion.
    ConfirmAccountDeletion,
    /// Close the confirmation, deleting nothing.
    CancelAccountDeletion,
    /// Open the source address in the browser (#299).
    OpenSource,
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
    /// Nothing. Carried by the picker's own panel so a tap inside it is
    /// not also a tap on the shade behind it, which would close it.
    PickerNothing,
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

/// The nearest [`Press`] at or above an entity, so a click on a button's
/// label counts as a click on the button.
pub(super) fn in_lineage<'a>(
    entity: Entity,
    presses: &'a Query<&Press>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Press> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok(found) = presses.get(e) {
            return Some(found);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}
