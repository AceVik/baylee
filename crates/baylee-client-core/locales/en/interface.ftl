### One thing the interface says.

# Army selected without targeting for amass.
NounArmyToAmass = Army to amass

# Plural label for the amass choice.
NounArmiesToAmass = Armies to amass

# Optional replacement for entering, before any enters-the-battlefield triggers.
# Arguments: arg0, arg1
EnterTappedForCounters = Enter tapped and get { $arg0 } { $arg1 }?

# Order for interacting simultaneous entry replacements.
ChooseEntryOrder = Which permanent’s entry replacements will you apply first?

# Choice among applicable replacements of the same battlefield arrival.
ChooseEntryReplacement = Which entry replacement will you apply next?

# Dredge (CR 702.52a), asked at each card draw it could replace: the cards in the graveyard are the menu, naming none draws. The German is the keyword as the German printings name it ("Ausgraben").
ChooseDredge = Dredge a card instead of drawing? Choose one from your graveyard, or confirm without a selection to draw.

# Choice among the replacements that could modify one card draw (CR 616.1): dredge cards, Chains of Mephistopheles, Aladdin's Lamp, Ring of Ma'rûf.
ChooseDrawReplacement = Which replacement will you apply to this draw next?

# Full sacrifice needed before the announced card can enter the battlefield.
# Arguments: arg0
SacrificeToEnter = Sacrifice { $arg0 } eligible permanents so this card can enter?

# Number of permanents to sacrifice as an object enters.
# Arguments: arg0, arg1
ChooseSacrificeCount = How many permanents will you sacrifice? ({ $arg0 }–{ $arg1 })

# Number of energy counters to pay; choosing zero declines.
# Arguments: arg0, arg1, arg2
ChooseEnergyPayment = Pay energy ({ $arg0 }–{ $arg1 }); up to { $arg2 } tapped permanents can receive counters

# One of the permanents actually tapped by this instruction.
NounTappedForCounter = tapped permanent to receive a counter

# Multiple non-targeted counter recipients.
NounTappedForCounters = tapped permanents to receive a counter

# A resolving effect asks which kind of counter to move.
ChooseCounterKind = Choose a counter kind

# One stun counter.
LogCounterStun = stun counter

# Several stun counters.
LogCountersStun = stun counters

# One finality counter.
LogCounterFinality = finality counter

# Multiple finality counters.
LogCountersFinality = finality counters

# Hold Shift to turn the preview.
PreviewTurn = Turn over

# Hold Alt/Option for the alternative preview.
PreviewAlternate = Alternate view

# Compact preview footer: hold Shift.
PreviewTurnCompact = Turn

# Hold Shift to rotate a flip or an Aftermath card to its other half.
PreviewRotate = Other half

# Compact preview footer: hold Shift to rotate to the other half.
PreviewRotateCompact = Half

# Compact preview footer: hold Alt/Option.
PreviewAlternateCompact = View

# Deck-construction choice for this physical copy.
SpiresCircles = Circle two colors for this copy

# A missing required deck choice.
SpiresNeedsCircles = Cryptic Spires needs two circled colors. Choose them in the printing picker.

# A circled pair outside the commanders' identity.
SpiresOutsideIdentity = Cryptic Spires’ circled colors exceed the commander’s color identity.

# Restyle an existing row, preserving quantity.
ApplyPrinting = Apply to this row

# Explicit catalog insertion destinations.
AddMainShort = + Main

# Explicit catalog insertion destinations.
AddSideShort = + Side

# Account-only save history.
HistoryAccountHint = History is available for account decks after signing in.

# New deck without a persisted identity.
HistorySaveHint = Save this deck to start its history.

# The builder's and the deck list's button that opens the import dialog.
ImportDeck = Import

# The builder's button that opens the export dialog.
ExportDeck = Export

# Heading of the import dialog.
ImportTitle = Import a deck

# Heading of the export dialog.
ExportTitle = Export this deck

# Puts the clipboard's text into the import box.
PasteFromClipboard = Paste

# Empties the import box.
ImportClear = Clear

# Takes the read deck into the builder.
ImportTake = Import deck

# Closes the import or export dialog.
TransferClose = Close

# Copies the export to the clipboard.
CopyToClipboard = Copy

# Saves the export as a file.
SaveToFile = Save file

# What the import box shows while empty.
ImportBoxEmpty = Nothing pasted yet

# Baylee's own text format, as the format chooser names it.
FormatBaylee = Baylee text

# The name an imported deck gets when its file names none.
ImportedDeckName = Imported deck

# The import dialog's instructions before anything is pasted.
ImportHowTo = Copy a deck list, a Baylee, JSON or YAML export, or a Moxfield export (More → Export → Copy for Moxfield), then paste it here with Paste or Ctrl/Cmd+V. The format is recognised by itself.

# A paste read as a deck. `{0}` the format, `{1}` rows, `{2}` cards.
# Arguments: arg0, arg1, arg2
ImportReadAs = Read as { $arg0 }. Rows: { $arg1 } · Cards: { $arg2 }

# Warns that importing replaces what is in the builder.
ImportReplaces = Importing replaces the deck in the builder with a new, unsaved deck. The saved deck stays as it is.

# A pasted Moxfield deck link. `{0}` is the site's name.
# Arguments: arg0
ImportMoxfieldLink = That is a link to a deck on { $arg0 }, which Baylee cannot read directly. Open the deck on { $arg0 } → More → Export → Copy for Moxfield, then paste it here.

# A link to a site with an API this build does not fetch yet. `{0}` the site.
# Arguments: arg0
ImportNotFetched = Decks linked from { $arg0 } cannot be fetched by this version yet. Export the deck there as text and paste it here.

# A link to a site no source knows. `{0}` the host.
# Arguments: arg0
ImportUnknownLink = Baylee cannot read decks from { $arg0 }. Export the deck there as a text list and paste the list here.

# A deck taken. `{0}` rows, `{1}` cards, `{2}` the format.
# Arguments: arg0, arg1, arg2
ImportTook = Imported from { $arg2 }. Rows: { $arg0 } · Cards: { $arg1 }

# The pool has not arrived, so names are not resolved yet.
ImportWaitingForPool = Waiting for the card pool to check the names…

# Cards the pool does not have. `{0}` how many.
# Arguments: arg0
ImportUnknownCards = Not in this gateway's card pool: { $arg0 }. Remove them before saving:

# Lines that were not rows. `{0}` how many.
# Arguments: arg0
ImportSkipped = Lines that are not deck rows: { $arg0 }

# One such line. `{0}` its number, `{1}` its text.
# Arguments: arg0, arg1
ImportSkippedLine = {"  "}line { $arg0 }: { $arg1 }

# Maybeboard rows, not kept. `{0}` how many.
# Arguments: arg0
ImportMaybeNotKept = Maybeboard rows not kept, as Baylee stores no maybeboard: { $arg0 }

# A commander the rules will not seat. `{0}` its name.
# Arguments: arg0
ImportNotALeader = { $arg0 } cannot lead a deck, so it is not marked as commander.

# The file named no deck. `{0}` the name given.
# Arguments: arg0
ImportNamedForYou = The file names no deck, so it is called “{ $arg0 }”.

# Rows with no printing. `{0}` how many.
# Arguments: arg0
ImportDefaultPrinting = Rows without a printing play the default printing: { $arg0 }

# Rows with no language. `{0}` how many.
# Arguments: arg0
ImportDefaultLanguage = Rows without a language are English: { $arg0 }

# A hundred-card main deck without a commander.
ImportNoCommander = The file names no commander. For a Commander deck, set one in the deck list's card menu.

# More entries than the list shows. `{0}` how many more.
# Arguments: arg0
AndMore = {"  "}and { $arg0 } more

# A paste over the document limit.
ImportTooLarge = That text is too large for a deck: at most 256 KiB.

# A document with too many rows. `{0}` how many.
# Arguments: arg0
ImportTooManyRows = A deck file holds at most 1000 rows; this one holds { $arg0 }.

# Nothing but blank lines and comments.
ImportNothing = There is no deck in that text.

# Not one line was a row. `{0}` the first line's number, `{1}` its text.
# Arguments: arg0, arg1
ImportUnreadable = No deck rows found. Line { $arg0 } reads: { $arg1 }

# JSON or YAML that did not parse. `{0}` the parser's words.
# Arguments: arg0
ImportSyntax = The file could not be read: { $arg0 }

# A document version this build does not read. `{0}` the version.
# Arguments: arg0
ImportVersion = This is a deck document of version { $arg0 }; this build reads version 1. Update Baylee to import it.

# A deck name that is too long or holds a line break.
ImportDeckName = The deck's name is too long or holds a line break.

# One card refused. `{0}` its position, `{1}` why.
# Arguments: arg0, arg1
ImportCardRefused = Card { $arg0 } cannot be imported: { $arg1 }

# Why: no name.
ImportCardName = it has no name.

# Why: a bad count.
ImportCardCount = its count must be from 1 to 1000000.

# Why: a long note.
ImportCardNote = its note is longer than 500 characters.

# Why: a field a stored row could not say back.
ImportCardUnstorable = its name or printing cannot be stored as written.

# The export says everything.
ExportComplete = This format keeps everything about the deck.

# The export leaves things out. `{0}` the format.
# Arguments: arg0
ExportLeavesOut = { $arg0 } cannot say everything; left out:

# Loss: the deck's name.
LossName = the deck's name

# Loss: languages. `{0}` rows.
# Arguments: arg0
LossLang = languages, rows: { $arg0 }

# Loss: printing ids. `{0}` rows.
# Arguments: arg0
LossScryfallId = exact printing ids, rows: { $arg0 }

# Loss: notes. `{0}` rows.
# Arguments: arg0
LossNote = notes, rows: { $arg0 }

# Loss: a finish written as non-foil. `{0}` rows.
# Arguments: arg0
LossFinish = holographic, glitter and galaxy finishes, written as non-foil, rows: { $arg0 }

# Loss: collector numbers without a set. `{0}` rows.
# Arguments: arg0
LossCollectorNumber = collector numbers without a set, rows: { $arg0 }

# Loss: the maybeboard. `{0}` rows.
# Arguments: arg0
LossMaybeboard = the maybeboard, rows: { $arg0 }

# The export is on the clipboard.
ExportCopied = Copied to the clipboard.

# The clipboard refused.
ExportCopyFailed = The clipboard could not be written. Save it as a file instead.

# Saved. `{0}` the path.
# Arguments: arg0
ExportSavedTo = Saved as { $arg0 }

# Handed to the browser. `{0}` the file name.
# Arguments: arg0
ExportDownloaded = Downloading { $arg0 }

# Not saved. `{0}` the system's words.
# Arguments: arg0
ExportSaveFailed = Could not save the file: { $arg0 }

# The import dialog's keys.
ImportKeys = Ctrl/Cmd+V pastes · Enter imports · Esc closes

# The export dialog's keys.
ExportKeys = ← → format · Ctrl/Cmd+C copies · Ctrl/Cmd+S saves · Esc closes

# Arguments: arg0
DeleteDeckQuestion = Delete “{ $arg0 }”?

# Destructive deck confirmation.
# Arguments: arg0
ClearDeckQuestion = Empty “{ $arg0 }”?

# Destructive deck confirmation.
DestructiveHint = Please confirm. Clearing removes the main deck and sideboard; deleting removes the saved deck.

# Commander controls in the deck overview.
CommanderSection = Commanders

# Commander selection.
ChooseCommander = Choose / replace

# Partner selection.
ChoosePartner = + Partner

# Return from selecting a leader to normal card browsing.
DoneChoosing = Back to all cards

# Roles and deck cards are distinct.
CommanderHint = Choose a leader here. Removing the role keeps the card in your deck.

# A partner must match the selected commander's rules.
PartnerHint = Only compatible partners are shown. Some commanders cannot have a partner.

# Show or hide detailed deck statistics.
DeckStatistics = Statistics

# Hub navigation.
HubPlay = Play

# Hub navigation.
HubDecks = My collection

# Hub navigation.
# The deck selected for the next game
SelectedDeck = Your next game

# Choose a deck
ChooseDeck = Choose a deck

# Hub navigation.
CreateTable = Create table

# Hub navigation.
# Arguments: arg0
PlayerCount = { $arg0 } players

# Hub navigation.
NoOwnDecks = Start with a house deck or build your own.

# Empty collection heading.
FirstDeck = Your first deck

# Empty table listing heading.
EmptyTablesTitle = A place for your next game

# A new player needs a deck before hosting a game.
ChooseDeckToBegin = Open a table and choose your deck in the room. Friends can join through the table search.

# A table search returned no rows.
NoMatches = No matching tables

# Clear a table search to see the unfiltered listing.
ClearTableSearch = Show all tables

# Hub navigation.
CollectionHint = Your decks, your ideas. Every saved change stays in your history.

# Gateway selection.
ChooseGateway = Choose your gateway

# Gateway selection.
GatewayHint = Where your account, decks and tables live.

# Gateway selection.
ChooseGatewayFirst = Select a gateway to sign in or register.

# Gateway selection.
GatewayAddress = GATEWAY ADDRESS

# Gateway selection.
SaveGateway = Save gateway

# Gateway selection.
GatewayUrlInvalid = Enter an http:// or https:// address without credentials, query parameters or a fragment.

# The front door's gear, pointed at: what is behind it.
LanguageAndSettings = Language and settings

# The front door's gear menu: the way to the whole settings screen.
AllSettings = All settings

# A saved gateway's bin, pointed at.
ForgetGateway = Remove from this list

# Asked before a saved gateway leaves the list.
# Arguments: arg0
ForgetGatewayQuestion = Remove “{ $arg0 }” from this list?

# Under that question: what removing it does, and what it does not.
ForgetGatewayHint = Only this device forgets it. Your account and decks stay on the gateway.

# A saved gateway's dot, pointed at, when it answered.
GatewayAnswering = answering

# Under the Fan Content notice: where the source is, the AGPL's §13
# offer (#270). `{0}` is the address, drawn as it came.
# Arguments: arg0
SourceCode = Source code (AGPL-3.0): { $arg0 }

# Compact settings action; its destination host is shown alongside.
ImpressumAction = Legal notice

# The gateway's legal notice (`/info` `impressum`) as a link, on the
# front door's About and the terms sheet. `{0}` is
# the host it opens, so the player sees where it goes.
# Arguments: arg0
ImpressumLink = Legal notice (Impressum) · { $arg0 }

# The front door's source tile (Wide and Vast): its label, over the
# address it opens.
TileSource = Source code (AGPL-3.0)

# The front door's Discord tile (Wide and Vast): its label, over the
# invitation it opens.
TileDiscord = Community on Discord

# Scryfall's attribution, which its terms ask of every client that
# shows its data or images (`docs/legal.md` §3, #325). Under the Fan
# Content notice, and under the version in the game menu, which a
# client seated straight into a game shows instead of the front door.
ScryfallCredit = Card data and images provided by Scryfall.

# Gateway selection: an address is asked about itself before it is saved.
# Arguments: arg0
GatewayChecking = Checking { $arg0 }…

# Gateway selection: the address answered and is saved.
# Arguments: arg0
GatewaySaved = Saved { $arg0 }.

# Gateway selection: nothing at the address answered as a gateway.
# Arguments: arg0
GatewayNotFound = No Baylee gateway answered at { $arg0 }. Check the address, or try again once it is up.

# Gateway row, beside its address, while it is being asked.
GatewayCheckingShort = checking…

# Gateway row: a gateway from before version checks.
GatewayVersionUnknown = version unknown

# Gateway row: a saved gateway that is down.
GatewayNotAnswering = not answering

# Gateway warning. `{0}` is the gateway's protocol version, `{1}` this client's.
# Arguments: arg0, arg1
GatewayProtocolMismatch = This gateway speaks protocol { $arg0 } and this client speaks { $arg1 }, so its games will not open here.

# Gateway warning. `{0}` is the gateway's view version, `{1}` this client's.
# Arguments: arg0, arg1
GatewayViewMismatch = This gateway's games send view version { $arg0 } and this client reads { $arg1 }, so they will not open here.

# Gateway warning: a gateway from before version checks.
GatewayOlder = This gateway is older than version checks. Whether its games open here shows only when one starts.

# Gateway warning: a saved gateway that is down.
GatewaySilent = This gateway is not answering right now.

# Server display-name rules, displayed before a registration is
# submitted, and what the display name is for: the username is private.
AccountNameHint = Others see it as Name#tag. 3–16 characters: A–Z, 0–9, _ or -, a letter or number at each end.

# Password registration guidance.
AccountPasswordHint = At least 8 characters, not a common password, and neither of your names.

# A rejected password during registration.
AccountPasswordInvalid = Choose a password with 8–256 characters, different from your username and display name, and not a common password.

# A library reply was not a valid response.
LibraryReadFailed = The library response could not be read. Please try again.

# Library and front-door interface.
WelcomeTitle = Your next game starts here.

# Library and front-door interface.
WelcomeNote = Build a deck. Find your table. Make it yours.

# Library and front-door interface.
OfflineBenefit = Try a game without an account. Offline decks stay on this device.

# Library and front-door interface.
LobbyGuide = Choose your deck, then join or open a table.

# Library and front-door interface.
DeckHistory = History

# Library and front-door interface.
HistoryHint = Every save is kept. Restoring creates a new version; later saves remain available.

# Library and front-door interface.
WorkingDiff = Changes from the latest saved version

# Special actions currently granted to this player.
GrantedActions = Granted actions

# Explicit life payment for one temporary action.
# Arguments: arg0
GrantedLifeCost = Pay { $arg0 } life

# Bound recipient of an already granted prevention action.
# Arguments: arg0
GrantedRecipient = Recipient: { $arg0 }

# One specific delayed life-loss obligation and its payment deadline.
# Arguments: arg0
GrantedCancelLifeLoss = Cancel this pending { $arg0 } life loss before your next draw step

# Confirms one explicitly selected temporary action.
GrantedConfirm = Perform action

# Library and front-door interface.
RestoreVersion = Restore this version

# Library and front-door interface.
ConfirmRestore = Confirm restore

# Library and front-door interface.
RestoreWarning = This replaces your working deck, including unsaved edits. Saved versions remain available.

# Library and front-door interface.
CurrentVersion = Current

# Compact printing-picker control.
ChoosePrintShort = Art

# Timestamp of the current save.
VersionSavedAt = Saved

# Historical timestamps describe when the next save replaced a version.
VersionSupersededAt = Replaced by next save

# Library and front-door interface.
# Arguments: arg0
VersionLabel = Version { $arg0 }

# Library and front-door interface.
LibraryEmpty = No decks published yet. You can still build your own.

# Library and front-door interface.
NoPastVersions = Your next save will appear here.

# Library and front-door interface.
LibraryLoading = Loading your library…

# Library and front-door interface.
LibraryRetry = Try again

# Library and front-door interface.
LibraryBack = Back

# Library and front-door interface.
LibraryMain = Main deck

# Library and front-door interface.
LibrarySide = Sideboard

# Library and front-door interface.
LibraryCommanders = Commanders

# Library and front-door interface.
NoChanges = No changes in this section.

# Library and front-door interface.
# Arguments: arg0, arg1
LibraryCounts = { $arg0 } cards · { $arg1 } sideboard

# Library and front-door interface.
# Arguments: arg0, arg1
LobbyCounts = { $arg0 } decks · { $arg1 } matching tables

# Library and front-door interface.
Composition = Deck overview

# Library and front-door interface.
UniqueCards = Unique cards

# Library and front-door interface.
AverageMana = Avg. mana · nonlands

# Library and front-door interface.
LandShare = Land share

# Library and front-door interface.
OpeningLand = Land in opening seven

# Library and front-door interface.
ConfirmDeleteDeck = Delete permanently?

# The product's name. Not translated, and here so that the one place it
# is written stays one place.
AppName = Baylee

# Caption over the username field.
Username = USERNAME

# Caption over the name field, when registering.
DisplayName = DISPLAY NAME

# Caption over the password field.
Password = PASSWORD

# Caption of the second password box, when an account is created.
PasswordAgain = REPEAT PASSWORD

# The account form's submit, in both of its modes: the tab above it
# already says which.
Continue = Continue

# The button that signs in.
SignIn = Sign in

# The button that registers.
CreateAccount = Create account

# Swaps the form to registering.
WantAnAccount = Create an account

# Swaps the form back to signing in.
HaveAnAccount = I already have an account

# Opens the lobby with no account and no gateway.
PlayOffline = Play offline

# The status line while offline play is on.
PlayingOffline = Offline: your decks and a table of house AI

# Opens the settings screen.
Settings = Settings

# The music's switch while the music plays (#296); pressed, it stops.
MusicPlaying = Music

# The music's switch while the music is silent; pressed, it plays.
MusicSilent = Music off

# The button that changes the interface language.
Language = Language

# The heading over the graphics knobs.
Graphics = Graphics

# The preset row: one name for every knob below it.
GraphicsPreset = Quality

# A preset, and the lowest ambient-effects level.
QualityLow = Low

# A preset, and the middle ambient-effects level.
QualityMedium = Medium

# A preset, and the highest ambient-effects level.
QualityHigh = High

# The highest preset.
QualityUltra = Ultra

# The preset once a knob was moved by hand.
QualityCustom = Custom

# Edge smoothing on the table.
AntiAliasing = Edge smoothing

# Whether frames wait for the display.
VSync = VSync

# Vertical sync that lets a late frame through.
VSyncAdaptive = adaptive

# The most frames per second with the window in front.
FrameLimit = Frame limit

# No frame limit.
Unlimited = unlimited

# The most frames per second behind other windows and on an idle
# front door.
BackgroundFrames = In the background

# Graphics: how many frames a table at rest draws.
RowRestLimit = Frame rate at rest

# How much the ambient surfaces (the front door's world, the cloth,
# the sky) move and how finely.
AmbientEffects = Ambient effects

# The heading over the volume knobs.
Audio = Volume

# Every sound and the music together.
MasterVolume = Overall

# The game's sounds, not the music.
EffectsVolume = Game sounds

# Silence while another window has the focus.
MuteInBackground = Silent in the background

# A newer release is downloaded, verified and waiting. `{0}` its version.
# Arguments: arg0
UpdateReady = Update { $arg0 } ready – installs when you quit

# A newer release exists and will not install itself. `{0}` its version.
# Arguments: arg0
UpdateAvailable = Update { $arg0 } available – download it here

# Shown once, at the first start after an update. `{0}` the version.
# Arguments: arg0
UpdatedTo = Updated to { $arg0 }

# Why an update is only a link: the player switched installing off.
UpdateWhyOff = Automatic updates are off in the settings.

# Why: a development build.
UpdateWhyDev = This is a development build: it checks, and never installs by itself.

# Why: the folder it runs from cannot be written by this user.
UpdateWhyFolder = Baylee cannot write to its own folder, so it cannot replace itself.

# Why: macOS runs the app from a read-only copy (App Translocation)
# and would not say where the original is.
UpdateWhyMoveApp = macOS runs Baylee from a read-only copy. Move it to Applications and it can update itself.

# Why, exactly: the folder of the installation cannot be written by
# this user. `{0}` the folder, `{1}` what the system answered.
# Arguments: arg0, arg1
UpdateWhyReadOnly = Baylee installs no updates here: it may not write to { $arg0 } ({ $arg1 }). Move it to your Applications folder and it can.

# Settings: macOS runs the app from a read-only copy, and updates
# install anyway.
UpdateTranslocated = macOS runs Baylee from a read-only copy, because it still lies where it was unpacked. Updates install anyway; moved to Applications, it starts without the detour.

# The button: copy the app into `~/Applications` and start it there.
UpdateMoveHome = Move to my Applications folder

# The button: copy the app into `/Applications`, for every user.
UpdateMoveSystem = Move to Applications for all users

# Under the buttons: what moving does.
UpdateMoveWhat = Copies Baylee there, starts the copy and closes this one. The old copy is deleted only if you say so.

# Moving failed. `{0}` why.
# Arguments: arg0
UpdateMoveFailed = Baylee could not be moved: { $arg0 }

# The first start after a move. `{0}` where it runs now.
# Arguments: arg0
UpdateMoved = Baylee now runs from { $arg0 }.

# Under it. `{0}` the old copy.
# Arguments: arg0
UpdateOldCopy = The old copy is still at { $arg0 }.

# Moves the old copy to the Trash.
UpdateTrashOld = Move old copy to Trash

# Keeps the old copy, and stops asking.
UpdateKeepOld = Keep it

# The Trash refused. `{0}` why.
# Arguments: arg0
UpdateTrashFailed = It could not be moved to the Trash ({ $arg0 }); drag it there yourself if you like.

# Why: the download's signature or checksum did not verify.
UpdateWhyNotVerified = The download could not be verified as ours, so it was not installed.

# Why: anything else (no archive for this system, no signature, a
# failed download or install).
UpdateWhyOther = This update cannot be installed automatically.

# Opens the release page.
ReleaseNotes = Release notes

# Puts the notice away for this session.
UpdateHide = Hide

# The ready update's offer, under its headline (beta.6): a restart
# installs it and comes back to where the player is.
UpdateRestartOffer = Update ready – restart now and continue where you are.

# The ready update's button: restart into it now.
UpdateRestartNow = Restart now

# The ready update's other button: not now, for this session.
UpdateLater = Later

# The restart could not be started. `{0}` what the system said.
# Arguments: arg0
UpdateRestartFailed = Could not restart: { $arg0 }

# After a restart, the place could not be taken up again.
ResumeFailed = Could not continue where you left off – started fresh.

# After a restart, the house game rebuilt from its record is not the
# game that was left (its hash differs), so it is not played on.
ResumeGameDiffers = The game could not be rebuilt exactly as it was, so it was not resumed.

# After a restart, back in a hosted game whose chair the house played
# meanwhile (the reconnect window had run out).
ResumeHousePlayed = Back at your table – the house played for you while you restarted.

# After a restart, back in a hosted game whose chair the house held.
ResumeSeatHeld = Back at your table – your seat was held while you restarted.

# Settings: the per-device switch for asking GitHub.
UpdateAutoCheck = Check for updates automatically

# Under it: what that costs in privacy (`docs/privacy.md`).
UpdateAutoCheckWhy = Asks GitHub at start and every six hours. GitHub sees your IP address and this version. Off: no request at all.

# Settings: the per-device switch for installing.
UpdateAutoInstall = Update automatically

# Under it.
UpdateAutoInstallWhy = Downloads a signed update and installs it when you quit.

# Settings: asks now.
UpdateCheckNow = Check for updates

# While a check runs.
UpdateChecking = Checking…

# A check found nothing newer.
UpdateUpToDate = Baylee is up to date.

# A check that could not ask (offline, rate limited).
UpdateCheckFailed = Could not check for updates.

# Leaves the account.
SignOut = Sign out

# Opens the confirmation that deletes the account (#292), on the
# settings screen.
DeleteAccount = Delete account

# Asked before an account is deleted. `{0}` is its name.
# Arguments: arg0
DeleteAccountQuestion = Delete the account { $arg0 }?

# Under that question, for an account with a password.
DeleteAccountHint = The account, its decks and its settings leave this gateway for good, and every device signed in to it is signed out. Type your password to confirm.

# The same, for a guest, which has no password to type.
DeleteGuestHint = The guest, its decks and its settings leave this gateway for good.

# The confirmation's button that deletes.
DeleteAccountConfirm = Delete for good

# The confirmation was sent with no password typed.
DeleteAccountNeedsPassword = your password, please

# The account is gone.
AccountDeleted = account deleted

# Heading over the account's decks.
YourDecks = Your decks

# Opens the builder on a new deck.
NewDeck = New deck

# Saves the acceptance file's starter deck.
AddStarterDeck = Add the starter deck

# Shown in place of an empty deck list.
NoDecksYet = no decks yet — add the starter deck

# Opens a saved deck in the builder.
Edit = Edit

# Throws a saved deck away.
Delete = Delete

# Heading over the tables.
Tables = Tables

# Caption over the table search box.
Search = SEARCH

# Inside the lobby's table search box while it is empty.
SearchTables = Table or host…

# Inside the deck builder's search box while it is empty.
#
# Three words and not a syntax lesson: the box takes the whole query
# language, and the gear beside it is where that is taught. What a
# placeholder can honestly say is where a bare word looks — which here
# is one field wider than the zone browser's, because a pool row has
# the card's rules text and a projected object does not.
SearchCards = Name, type, text…

# Runs the search.
DoSearch = Search

# Re-reads decks and tables.
Refresh = Refresh

# The one-tap game against the house AI.
PlayTheHouse = Play the house

# Localized name for an unnamed house-controlled chair. Named AI seats
# retain their roster name so the HUD, prompts and stack agree.
SeatHouse = House AI

# Caption over the room password box.
RoomPassword = ROOM PASSWORD

# Before the row of table sizes.
OpenATableFor = Open a table for

# Shown in place of an empty table list.
NoTablesOpen = The tables are quiet for now. Open yours and invite someone to play.

# Empty offline play area: only the local house AI is available.
OfflineReadyToPlay = Your deck is ready. Play the house AI now, or create a table to choose its seats and teams.

# The gateway stopped answering: the feed closed or a request found
# nobody there. Stays until it answers again.
GatewayUnreachable = The gateway is not answering. Trying again…

# The gateway answered again after [`Phrase::GatewayUnreachable`].
GatewayBack = The gateway is back.

# The listing said no agent is there: no room can open or start.
NoNewGames = No new games can start right now — the server may be updating. Games already running go on.

# Shown when a search matched nothing. `{0}` is what was searched for.
# Arguments: arg0
NoTableMatches = no table matches “{ $arg0 }”

# Sits down at a table.
Join = Join

# Says this player is ready.
Ready = Ready

# Takes that back.
NotReady = Not ready

# The host's go.
Start = Start

# Gives up a chair.
Leave = Leave

# Puts the selected deck in a chair.
UseMyDeck = use my deck

# Hands the room to the player in a chair.
MakeHost = make host

# Takes a named chair.
SitHere = sit here

# Turns a chair over to the AI.
SeatToAi = → AI

# Turns it back into a chair for a person.
SeatToOpen = → open

# Seats a language model in an open chair (`docs/llm-seat.md`).
SeatToLlm = → language model

# What a language-model chair plays. `{0}` the model's label and id, `{1}` the effort.
# Arguments: arg0, arg1
RoomLlmPlays = Language model: { $arg0 }, effort { $arg1 }

# The effort a model picks itself.
RoomLlmEffortOwn = the model's own

# Over the profile chips of a language-model chair.
RoomLlmProfile = Profile (settings file)

# Under a model that takes no named effort.
RoomLlmNoEfforts = This model takes no effort this build can name.

# Over the deck chips of a language-model chair.
RoomLlmDeck = Deck it brings

# Keeps a chair's model and effort as its profile's.
RoomLlmSave = Keep as the profile's

# Takes the language model out of the chair.
RoomLlmRemove = Remove the model

# Before the chair's bridge has said anything.
RoomLlmStarting = Its bridge takes the chair once the room lists it open.

# An open chair's entry for a model the gateway runs.
HostedEntry = Hosted model

# The hosted-model sheet's title.
# Arguments: arg0
HostedTitle = Hosted model for chair { $arg0 }

# Over the hosted-model list.
HostedIntro = A model the gateway runs plays this chair. Pick one:

# While the list is asked for.
HostedAsking = Asking the gateway…

# No profile at all.
HostedNone = This gateway offers no hosted model now.

# Under an unavailable profile with a known return.
# Arguments: arg0
HostedBackAt = back from { $arg0 }

# Under an unavailable profile with no known return.
HostedNotNow = not available now

# Under the chair while its bridge starts.
HostedStarting = Starting…

# Under the chair once its bridge said ready.
HostedReady = Ready

# Under a chair whose order failed.
# Arguments: arg0
HostedFailed = The hosted model failed: { $arg0 }

# A failure without a reason.
HostedNoReason = no reason given

# Where the game's data goes.
# Arguments: arg0
HostedDataGoesTo = Game data goes to { $arg0 }

# Takes a hosted chair back.
HostedTakeBack = Take back

# The in-game panel's title (a debug build only).
GameLlmTitle = Language-model chairs (debug: from the next decision)

# Hands a language-model chair to the house during the game.
GameLlmHouse = House AI

# The gentlest house AI.
AiNovice = novice

# The relaxed house AI.
AiCasual = casual

# The middle one.
AiSteady = steady

# The one that plays to win.
AiSharp = sharp

# The deepest house AI combat search.
AiExpert = expert

# One page back through the table list.
PageBack = ‹ Back

# One page on.
PageMore = More ›

# Which rows are shown. `{0}`–`{1}` of `{2}`.
# Arguments: arg0, arg1, arg2
PageOf = { $arg0 }–{ $arg1 } of { $arg2 }

# A chair nobody is in. `{0}` is the seat number.
# Arguments: arg0
SeatOpen = seat { $arg0 } · open

# A chair the AI plays. `{0}` is the seat, `{1}` the difficulty.
# Arguments: arg0, arg1
SeatAi = seat { $arg0 } · AI ({ $arg1 })

# Somebody else's chair. `{0}` is the seat, `{1}` their name.
# Arguments: arg0, arg1
SeatTaken = seat { $arg0 } · { $arg1 }

# This player's own chair. `{0}` is the seat, `{1}` their name.
# Arguments: arg0, arg1
SeatYours = seat { $arg0 } · { $arg1 } (you)

# How full a table is. `{0}` seated of `{1}` chairs.
# Arguments: arg0, arg1
Seated = { $arg0 }/{ $arg1 } seated

# How big a table that is already playing is. `{0}` chairs.
# Arguments: arg0
SeatCount = { $arg0 } seats

# What a waiting room is waiting for. `{0}` is how many are not ready.
# Arguments: arg0
WaitingFor = waiting for { $arg0 }

# The same, when the seat being waited for is the one reading it.
#
# A phrase of its own rather than [`Phrase::WaitingFor`] filled with a
# pronoun, because a name and a pronoun do not decline alike: German
# wants the accusative after "auf", so this slot would have to be handed
# "dich" while every other line that names the local seat wants "Du". A
# slot that needs a different word in different sentences is a slot that
# gets filled wrongly, and the sentence is what to split.
WaitingForYou = waiting for you

# A room whose chairs are all ready.
AllReady = all ready

# A room that is locked.
Locked = locked

# A table's state, as the listing gives it.
StateWaiting = waiting

# The same, for one being played.
StatePlaying = playing

# The same, for one that is over.
StateOver = over

# The banner over a table of ours nobody has joined yet. `{0}` is its id.
# Arguments: arg0
TableOpenWaiting = your table { $arg0 } is open — waiting for an opponent

# The same banner offline, where the table carries no id worth reading
# and nobody is on their way to it.
TableOpenHouseWaiting = your table is open — arrange the chairs and start

# The veil over the moment between a granted seat and the duel.
TakingYourSeat = taking your seat…

# Back to the lobby from a finished game.
BackToLobby = Back to the lobby

# The other button over a finished game, and the one on a rematch room
# in the list — the same press either way, from the two screens the two
# players are looking at.
PlayAgain = Play again

# Signed in, nothing happening.
SignedIn = signed in

# Signed out.
SignedOut = signed out

# The form was submitted with something missing.
NeedUsernameAndPassword = a username and a password, please

# Registering, and the username has an invisible or direction-turning
# character in it (`names::UsernameFault::Invisible`).
UsernameInvisible = that username holds a character that cannot be seen

# Registering, and the username has a character the rule does not allow.
UsernameCharacters = a username is letters A–Z, digits and _ - . only

# Registering, and the username is too short or too long.
UsernameLength = a username has 3 to 24 characters

# Registering, and the username begins or ends with a separator.
UsernameEdge = a username starts and ends with a letter or a digit

# Registering, and the username has two separators in a row.
UsernameDoubled = a username has no two of _ - . in a row

# Signed in with an address: the name to sign in with from now on.
# `{0}` is the username.
# Arguments: arg0
YourUsernameIs = signed in — your username is { $arg0 }; the address works until the end of 2026

# The button that plays as a new guest (#269).
PlayAsGuest = Play as guest

# The same button, for the guest this device already holds here. `{0}`
# is its handle.
# Arguments: arg0
ContinueAsGuest = Continue as { $arg0 }

# Caption of the box for a guest's name.
GuestName = GUEST NAME

# What that box says while empty: the name the gateway gives a guest
# that chose none. The gateway's word, in every language, because it is
# the name the other players will read.
GuestDefaultName = Guest

# While the gateway makes the guest.
JoiningAsGuest = joining as a guest…

# Just in, as a guest.
PlayingAsGuest = playing as a guest

# On the tables screen for the whole of a guest's visit: what a guest is.
GuestNotice = You are playing as a guest. This account and its decks are deleted about 30 days after your last visit.

# The guest this device held has ended on the gateway.
GuestEnded = that guest has ended — play as a new one, or sign in

# The gateway takes no guests (its `/auth/config` said so).
NoGuests = this gateway takes no guests

# Caption of the box for a closed-beta key (#317).
InviteKey = CLOSED BETA KEY

# What that box says while empty: the shape of a key.
InviteKeyShape = BAYLEE-XXXX-XXXX-XXXX-XXXX

# Under the box: what the key is for, and that pasting it is fine.
InviteKeyHint = This gateway is a closed beta: a new account or a new guest needs a key. Paste it as you got it.

# A new account or guest was asked for with the box empty.
NeedInviteKey = your closed beta key, please

# The gateway refused the key, for whichever reason: mistyped, used,
# expired or revoked. It does not say which, and neither does this.
InviteKeyInvalid = this closed beta key is not valid — check it, or ask for a new one

# The gateway wanted a key and none reached it.
InviteKeyNeeded = this gateway is a closed beta: a new account or guest needs a closed beta key

# Asked before a guest signs out. `{0}` is its handle.
# Arguments: arg0
GuestSignOutQuestion = Sign out { $arg0 }?

# Under that question: what signing out does to a guest.
GuestSignOutHint = A guest cannot sign in again. Signing out deletes this guest and its decks now.

# The same, registering.
NeedDisplayName = a display name, please

# Registering, and the password was typed differently the second time.
PasswordsDiffer = the two passwords differ

# A sign-in refused for a ban with an end (beta.8): `{0}` the end,
# `{1}` the admin's reason.
# Arguments: arg0, arg1
BannedUntil = This account is banned until { $arg0 }. Reason: { $arg1 }

# A sign-in refused for a ban for good: `{0}` the admin's reason.
# Arguments: arg0
BannedForGood = This account is banned. Reason: { $arg0 }

# The face that asks for a new password after an admin reset it.
ChooseNewPassword = Choose a new password

# What that face says, under its title.
PasswordWasReset = An admin reset your password. Choose a new one to sign in.

# The new password's caption, in capitals as every caption is.
NewPassword = NEW PASSWORD

# That face's button.
SetNewPassword = Set password and sign in

# The new password is the one the admin gave.
NewPasswordIsTheOld = choose a password other than the one you were given

# Sending the new password.
SettingPassword = setting your password…

# This gateway does not take sign-ups.
NoSignUps = this gateway is not taking new accounts

# Registering.
CreatingAccount = creating the account…

# Signing in.
SigningIn = signing in…

# Registered, and signing in with the same credentials.
AccountCreated = account created — signing in…

# Fetching the card pool.
LoadingPool = loading the card pool…

# Saving a deck.
SavingDeck = saving the deck…

# Saved.
DeckSaved = deck saved

# Opening a deck in the builder.
OpeningDeck = opening the deck…

# Deleting a deck.
DeletingDeck = deleting the deck…

# Deleted.
DeckDeleted = deck deleted

# Opening a table.
OpeningTable = opening a table…

# Sitting down at one.
SittingDown = sitting down…

# Asking for a chair at the next table.
PlayingAgain = playing again…

# Saying ready.
SayingReady = ready…

# Taking it back.
SayingNotReady = not ready…

# The host's start.
Starting = starting…

# Handing the room on.
HandingOver = handing the room over…

# Arranging a chair.
ArrangingTable = arranging the table…

# Standing up.
LeavingTable = leaving the table…

# A deck is needed first.
PickADeckFirst = pick a deck first

# Our own table is open and waiting.
TableOpen = table open — waiting for an opponent

# The same offline, where the rest of the chairs are the house.
TableOpenHouse = table open — arrange the chairs and start

# Somebody joined it.
OpponentSatDown = an opponent sat down

# A chair at somebody else's room, which does not start on sitting down.
#
# A room takes two statements by two people — this seat's own `ready`
# and the host's `start` — so the seat is held and the game is not
# running yet. Said in the second person because the next move is the
# player's.
YouAreSeated = you are seated — say you are ready, then the host starts

# The seat was granted and the duel is opening.
TakingTheSeat = taking the seat…

# The game we were in ended and the lobby is back.
GameEnded = the game ended

# A body arrived that made no sense. `{0}` names what was being read.
# Arguments: arg0
Unreadable = could not read { $arg0 } the gateway sent

# What `{0}` is, in that message: the table list.
TheGameList = the game list

# …the deck list.
TheDeckList = the deck list

# …the sign-in.
TheSignIn = the sign-in

# …a seat.
TheSeat = the seat

# …the card pool.
ThePool = the card pool

# …a card's printings.
ThePrintings = the printings

# …one deck.
TheDeck = the deck

# Settings
SettingsTitle = Settings

# Back
Back = Back

# Saved to your account — these travel with you to any table.
SettingsOnAccount = Saved to your account — these travel with you to any table.

# Saved on this computer. Sign in and they follow your account.
SettingsOnDevice = Saved on this computer. Sign in and they follow your account.

# Keys
Keys = Keys

# Automation
Automation = Automation

# Where to stop
WhereToStop = Where to stop

# press a key…
PressAKey = press a key…

# unbound
Unbound = unbound

# Reset all
ResetAll = Reset all

# reset
Reset = reset

# on
SwitchOn = on

# off
SwitchOff = off

# A red step is one the client passes for you.
RailExplain = A red step is one the client passes for you. Nothing is red until you make it red.

# The button that writes a preset into the rail.
UsePreset = use

# Stop everywhere
RailPresetEveryStep = Stop everywhere

# Ask me in every step of both turns.
RailPresetEveryStepDetail = Ask me in every step of both turns.

# Skip the quiet steps
RailPresetQuietSteps = Skip the quiet steps

# Not untap, upkeep, draw, damage or cleanup — nothing is decided there.
RailPresetQuietStepsDetail = Not untap, upkeep, draw, damage or cleanup — nothing is decided there.

# Competitive stops
RailPresetCompetitive = Competitive stops

# Both your main phases, both combat declarations, and their end step.
RailPresetCompetitiveDetail = Both your main phases, both combat declarations, and their end step.

# The half of the rail that is your own turns.
YourTurns = Your turns

# The other half.
TheirTurns = Opponents' turns

# Hold the table still
HoldTheTableStill = Hold the table still

# Cards and the camera go straight there instead of moving.
HoldTheTableStillWhy = Cards and the camera go straight there instead of moving.

# Heading over the day/night picker.
Sky = Sky

# What the sky is for, in one line.
SkyWhy = Weather behind the table. It changes nothing in the game.

# Follow the player's own clock.
SkyAuto = Auto

# Always a clouded blue sky.
SkyDay = Day

# Always stars and a crescent moon.
SkyNight = Night

# Heading over the weather picker.
Atmosphere = Atmosphere

# What the weather is for, in one line.
AtmosphereWhy = Aurora and drifting stars in the night sky.

# No weather at all.
AtmosphereOff = Off

# Half the budget: visible when looked for, invisible when played under.
AtmosphereSoft = Soft

# The whole budget.
AtmosphereFull = Full

# Heading over the loudness picker.
Sound = Sound

# What the table says out loud, in one line.
SoundWhy = Short tones for life, your turn, and the end of a game.

# Everything, at the levels the sink was balanced at.
SoundFull = On

# The same balance, half the amplitude.
SoundHalf = Quieter

# Nothing at all.
SoundOff = Off

# Do the obvious thing
ActPrimary = Do the obvious thing

# Explicit shortcut for the current repeated targeting series.
ActConfirmTargetBatch = Confirm targets for this series

# Ordinary confirmation or priority pass.
ActConfirm = Confirm / pass priority

# Cancel
ActCancel = Cancel

# Play or choose the card
ActActivateCard = Play or choose the card

# Choose every card in the group
ActActivateGroup = Choose every card in the group

# Cursor left
ActCursorLeft = Cursor left

# Cursor right
ActCursorRight = Cursor right

# Cursor up
ActCursorUp = Cursor up

# Cursor down
ActCursorDown = Cursor down

# Aim at the next defender
ActCombatFocusNext = Aim at the next defender

# Aim at the previous defender
ActCombatFocusPrev = Aim at the previous defender

# Declare nothing
ActCombatNone = Declare nothing

# Skip to the next phase
ActNextPhase = Skip to the next phase

# Skip to the next turn
ActNextTurn = Skip to the next turn

# Hide the board overlay
# Read card text instead of art
ActToggleTextView = Read card text instead of art

# Open the zone browser. The clause is the settings row's help: a tap on
# a pile whose top card is lit arms that card's cast rather than opening
# the pile (#242), and this key is the way in that is left.
ActToggleBrowser = Open the zone browser, also when a graveyard's top card is lit

# Open the game log (#262).
ActToggleLog = Open the game log

# Let the stack resolve
ActHoldForStack = Let the stack resolve

# Nothing more this turn
ActHoldForTurn = Nothing more this turn

# Keep this hand
ActMulliganKeep = Keep this hand

# Mulligan
ActMulliganTake = Mulligan

# Yes
ActAnswerYes = Yes

# Finite batch of already stacked identical may decisions.
# Arguments: arg0
AnswerYesBatch = Yes to all { $arg0 } identical abilities

# Public continuing effect on a player's hand limit.
NoMaxHandSize = No maximum hand size

# Finish only the current creature's remaining damage division.
AutoCombatDamage = Assign the rest automatically

# No
ActAnswerNo = No

# Number up
ActNumberUp = Number up

# Number down
ActNumberDown = Number down

# Scroll a stack entry's long text back a page
ActTextPageUp = Scroll a stack entry's text up

# Scroll it on a page
ActTextPageDown = Scroll a stack entry's text down

# Rail selection up
ActRailUp = Rail selection up

# Rail selection down
ActRailDown = Rail selection down

# The table-angle row (DESIGN-v7 D20)
RowTableLean = Table angle

# What the table angle does
HelpTableLean = How steeply the camera looks at a table of three seats or more. Steep draws your own board about a third larger and the far seats a little smaller; flat draws every seat's board the same size.

# The steep lean
LeanSteep = Steep

# The gentle lean
LeanGentle = Flat

# The visit-camera row (DESIGN-v7 D21)
RowVisitCamera = Looking at a seat

# What the visit camera does
HelpVisitCamera = Where the camera stands when you look at another seat (its button, F). Automatic: behind a teammate, as if sitting beside them, and across from an opponent, whose board you then read as in a duel. Or always behind it with your own lane or the dial in view, behind it with the dial always in view, or across from it.

# The automatic visit: behind a teammate, across from an opponent
VisitAuto = Automatic

# Behind, with the rule
VisitBehind = Behind

# Behind, the dial always
VisitBehindDial = Behind, dial

# Across
VisitAcross = Across

# The game menu's priority-sound switch, on
PriorityCueOn = Priority sound: on

# …and off
PriorityCueOff = Priority sound: off

# The priority-sound row (DESIGN-v7 D22)
RowPriorityCue = Priority sound

# What the priority sound does
HelpPriorityCue = A soft strike when the table starts waiting for you — only on your own device, never for the others. It has its own switch; the music's switch never silences it.

# Visit the next seat (TABLE-KEYBOARD §9)
ActFocusNextSeat = Visit the next seat

# Visit the previous seat
ActFocusPrevSeat = Visit the previous seat

# Back to my own seat
ActFocusHome = Back to my seat

# Open the arrangement menu (DESIGN-v8 §2.3)
ActArrangementMenu = Choose the arrangement

# Cycle to the next offered arrangement
ActNextArrangement = Next arrangement

# Open or shut the hand's drawer on a phone (DESIGN-v8 WA11)
ActHandDrawer = Open or shut the hand (phone)

# The creature-type chooser's quick list: the types the deck plays most.
TypesInDeck = Most in your deck

# The creature-type chooser's full list.
TypesAll = All types

# A cast made before its mana, asking its own question first: the
# spell, the question, and that the payment comes after (CR 601.2g).
# Arguments: arg0, arg1
CastFirstQuestion = Cast { $arg0 }: { $arg1 } — paid after

# The answer that takes back a spell being cast (CR 732), in its
# questions and its payment window.
CancelCast = Cancel cast

# Fold the question's sheet to its pill, or open it again.
ActFoldDecision = Fold or open the question's sheet

# The ring (DESIGN-v8 arrangement 1)
ArrRing = Ring

# What the ring does
ArrRingBlurb = Every seat on the ring; the camera visits.

# The upright ring (arrangement 2)
ArrUprightRing = Upright ring

# Compact arrangement segment; the letter is added by the control.
ArrUprightRingShort = Upright ring

# What it does
ArrUprightRingBlurb = The ring with every board upright to you; a visit zooms in.

# The turntable (arrangement 3)
ArrTurntable = Turntable

# What it does
ArrTurntableBlurb = You and one opponent as in a duel, the others small at the sides; choosing a seat turns the table.

# The arc rail (arrangement 4)
ArrArcRail = Arc rail

# What it does
ArrArcRailBlurb = The opponents on an arc above you; the camera slides along the rail.

# The pods (arrangement 5)
ArrPods = Pods

# What they do
ArrPodsBlurb = Every opponent upright in a grid, seen from above; a visit zooms in.

# The spotlight (arrangement 6)
ArrSpotlight = Spotlight

# What it does
ArrSpotlightBlurb = A duel against one seat, the others as chips in the strip; turn on “Table follows the turn” to follow play.

# The turntable with rows (arrangement 7)
ArrTurntableRows = Turntable+

# What it does, and what it is here
ArrTurntableRowsBlurb = Turntable up to four seats, Spotlight from five or on a phone.

# What the turntable with rows resolved to at this table
# Arguments: arg0
ArrResolvedTo = Here: { $arg0 }

# The focus ring (arrangement 8)
ArrFocusRing = Focus ring

# What it does
ArrFocusRingBlurb = A duel in the middle, the other seats as peeks at the edges.

# A greyed row: a duel has one arrangement
ArrNotInADuel = A duel has only one arrangement

# A greyed row: not built yet
# Arguments: arg0
ArrComing = Coming ({ $arg0 })

# A greyed row: not at this many seats on this window
ArrNotHereSeats = Not at this many seats on a window this size

# A greyed row: not on a window this small
ArrNotHereWindow = Not on a window this small

# A greyed row: not on a phone
ArrNotOnAPhone = Not on a phone

# The tag on a row whose measurement is outstanding
ArrExperimental = experimental

# The menu's last row: keep this choice for the seat count
# Arguments: arg0
ArrRememberForSeats = Remember for { $arg0 } seats

# The placeholder note under the menu
ArrLettersNote = The letters stand in for pictograms still to be drawn.

# The game menu's row
# Arguments: arg0
ArrGameMenuRow = Arrangement › { $arg0 }

# The arrangement settings row
RowArrangement = Table arrangement

# What it does
HelpArrangement = How the seats of a table of three or more are placed. P chooses one at the table, Shift+P the next; this is the default.

# The follow switch's row
RowFollowTurn = Table follows the turn

# What it does
HelpFollowTurn = As another player's turn begins the table shows that player's seat — a camera arrangement visits it, the others bring it across — and on your turn it comes back to yours. It waits while you are answering a question; a seat you look at by hand stays until the next turn begins.

# The per-seat-count row
RowArrangementBySeats = Per seat count

# What it does
HelpArrangementBySeats = An arrangement remembered for one seat count wins over the default at a table of that size.

# A seat count's label in the per-count rows
# Arguments: arg0
ArrSeats = { $arg0 } seats

# The per-count choice that takes the default
ArrAsDefault = Default

# The `?` overlay's line for the two keys
ArrKeysHint = P chooses the arrangement · Shift+P the next

# Answering
GroupAnswering = Answering

# Moving around
GroupMovingAround = Moving around

# Combat
GroupCombat = Combat

# Phases
GroupPhases = Phases

# Questions
GroupQuestions = Questions

# Display
GroupDisplay = Display

# The chat's keys in Settings → Keys
GroupChat = Chat

# Open the chat with a command
ActChatCommand = Chat: type a command (/)

# Answer the last whisper
ActChatReply = Chat: answer the last whisper

# Open the chat's box
ActChatOpen = Chat: open the box

# Open the friends list
ActFriends = Friends list

# Pass when there is nothing to do
AutoPassLabel = Pass when there is nothing to do

# No land, no spell, no ability, nothing to suspend: pass without asking.
AutoPassDetail = No land, no spell, no ability, nothing to suspend: pass without asking.

# Pass through opponents' turns
AutoSkipTurnsLabel = Pass through opponents' turns

# Priority only. It never declines a block for you.
AutoSkipTurnsDetail = Priority only. It never declines a block for you.

# Skip an empty attack step
AutoSkipAttacksLabel = Skip an empty attack step

# Only when nothing you control can attack.
AutoSkipAttacksDetail = Only when nothing you control can attack.

# Skip an empty block step
AutoSkipBlocksLabel = Skip an empty block step

# Only when nothing you control can block.
AutoSkipBlocksDetail = Only when nothing you control can block.

# Untap
RailUntap = Untap

# Upkeep
RailUpkeep = Upkeep

# Draw
RailDraw = Draw

# Main 1
RailMain1 = Main 1

# Begin Combat
RailCombatBegin = Begin Combat

# Attackers
RailAttackers = Attackers

# Blockers
RailBlockers = Blockers

# Damage
RailDamage = Damage

# End of Combat
RailCombatEnd = End of Combat

# Main 2
RailMain2 = Main 2

# End Step
RailEndStep = End Step

# Cleanup
RailCleanup = Cleanup

# Resolution choice: `MutateOnTop`.
MutateOnTop = Put on top

# Resolution choice: `MutateUnderneath`.
MutateUnderneath = Put underneath

# Resolution choice: `ChooseMutatePosition`.
ChooseMutatePosition = Choose how to merge

# Resolution choice: `ChooseDungeon`.
ChooseDungeon = Choose a dungeon

# Resolution choice: `ChooseDungeonRoom`.
ChooseDungeonRoom = Choose the next room

# White
ColorWhite = White

# Blue
ColorBlue = Blue

# Black
ColorBlack = Black

# Red
ColorRed = Red

# Green
ColorGreen = Green

# Colourless
ColorColourless = Colourless

# Creature
KindCreature = Creature

# Instant
KindInstant = Instant

# Sorcery
KindSorcery = Sorcery

# Artifact
KindArtifact = Artifact

# Enchantment
KindEnchantment = Enchantment

# Planeswalker
KindPlaneswalker = Planeswalker

# Battle
KindBattle = Battle

# Land
KindLand = Land

# Other
KindOther = Other

# Creatures
GroupCreatures = Creatures

# Planeswalkers
GroupPlaneswalkers = Planeswalkers

# Instants
GroupInstants = Instants

# Sorceries
GroupSorceries = Sorceries

# Artifacts
GroupArtifacts = Artifacts

# Enchantments
GroupEnchantments = Enchantments

# Battles
GroupBattles = Battles

# Lands
GroupLands = Lands

# Other
GroupOther = Other

# A–Z
SortName = A–Z

# Cost
SortCost = Cost

# Type
SortType = Type

# Sort: {0}
# Arguments: arg0
SortBy = Sort: { $arg0 }

# Cards ({0})
# Arguments: arg0
PaneCards = Cards ({ $arg0 })

# Unapplied host edits.
RoomDraft = Unsaved settings · apply your changes before starting.

# Room configuration title.
RoomTitle = Your table

# Public name input.
RoomName = Room name

# Shared configuration heading.
RoomRules = Rules & starting position

# Host authority explanation.
RoomHostHelp = You arrange the table. Players choose their own decks and confirm they are ready.

# Offline room guidance: the local player controls every seat.
RoomOfflineHelp = Choose decks and AI strength. Adjust the starting rules if you like, then start the game.

# Visitor authority explanation.
RoomGuestHelp = The host arranges the table. Choose your deck below, then mark yourself ready.

# Seats heading.
RoomPlayers = Players & seats

# Global starting life.
RoomLife = Starting life

# Free mulligan count.
RoomMulligans = Free mulligans

# Permanent input label.
RoomBoard = Find starting permanents

# Expandable starting-position editor; {0} counts configured permanents.
# Arguments: arg0
RoomStartingCards = Starting cards · { $arg0 }

# Draft submit.
RoomApply = Apply settings

# Optional password help.
RoomLockHelp = Leave blank to keep the current password. Changes take effect with Apply settings.

# Remove lock.
RoomUnlock = Remove password

# Preset picker heading.
RoomTemplate = Starting template

# Starting-position card editor.
RoomCardAppearance = Edition & finish

# Starting-position card editor.
RoomCardCounters = Starting counters

# Starting-position card editor.
RoomCounterKind = Counter type

# Starting-position card editor.
RoomCounterHint = +1/+1, -1/-1, loyalty, lore, time, charge, poison, energy, rad, lifelink, level, flying, shadow, indestructible, hone; any +X/+Y or -X/-Y pair; custom:ID

# Starting-position card editor.
RoomAddCounter = Add counter

# Starting-position card editor.
RoomRemoveCard = Remove card

# Starting-position card editor.
RoomNoCardMatches = No matching permanent.

# Accelerated original house-rule scenario.
RoomFast = Five-land start

# Planechase honesty.
RoomPlanechase = Planechase · unavailable: planar cards and planar die are not implemented yet.

# Host handover guarantee.
RoomSuccession = If the host leaves, the longest-standing player takes over.

# Deck selector heading.
RoomPickDeck = Choose a deck

# Shared default for per-seat life.
RoomDefault = Use default

# Empty deck notice in room.
RoomNoDeck = No deck selected

# Deck ({0} / {1})
# Arguments: arg0, arg1
PaneDeck = Deck ({ $arg0 } / { $arg1 })

# Leave without saving
LeaveWithoutSaving = Leave without saving

# ‹ Decks
BackToDecks = ‹ Decks

# Editing a deck
EditingADeck = Editing a deck

# A new deck
ANewDeck = A new deck

# Save deck
SaveDeck = Save deck

# Saved
DeckIsSaved = Saved

# Hide filters
HideFilters = Hide filters

# Filters
ShowFilters = Filters

# Clear
ClearFilters = Clear

# Playable only
PlayableOnly = Playable only

# {0} of {1} cards{2}
# Arguments: arg0, arg1, arg2
PoolTally = { $arg0 } of { $arg1 } cards{ $arg2 }

# — showing {0}, keep typing to narrow it
# Arguments: arg0
PoolNarrow = {" "}— showing { $arg0 }, keep typing to narrow it

# nothing matches — try fewer filters
NothingMatches = nothing matches — try fewer filters

# no printings
NoPrintings = no printings

# looking for other printings…
LookingForPrintings = looking for other printings…

# {0} of {1}
# Arguments: arg0, arg1
PrintingAt = { $arg0 } of { $arg1 }

# Other printings could not be loaded. Showing the reference printing.
NoCatalogOnlyThis = Other printings could not be loaded. Showing the reference printing.

# The printing the build itself carries, where no other is known.
ReferencePrinting = Reference printing

# All
AllSets = All

# Plain
FinishPlain = Plain

# Foil
FinishFoil = Foil

# Etched
FinishEtched = Etched

# Add
AddPrinting = Add

# {0} in the {1}
# Arguments: arg0, arg1
CountInZone = { $arg0 } in the { $arg1 }

# deck
ZoneDeck = deck

# sideboard
ZoneSideboard = sideboard

# no art for this printing
NoArtForPrinting = no art for this printing

# no rules text — this gateway has no card catalog
NoRulesText = no rules text — this gateway has no card catalog

# {0} — this card will not play as printed
# Arguments: arg0
NotAsPrinted = { $arg0 } — this card will not play as printed

# + deck
AddToDeck = + deck

# + sideboard
AddToSideboard = + sideboard

# → sideboard
MoveToSideboard = → sideboard

# → deck
MoveToDeck = → deck

# remove
RemoveCard = remove

# commander ✓
IsCommander = commander ✓

# set as commander
SetCommander = set as commander

# {0} in the deck
# Arguments: arg0
HeldInDeck = { $arg0 } in the deck

# {0} in the sideboard
# Arguments: arg0
HeldInSideboard = { $arg0 } in the sideboard

# {0} in the deck, {1} in the sideboard
# Arguments: arg0, arg1
HeldInBoth = { $arg0 } in the deck, { $arg1 } in the sideboard

# untested
CoverageGenerated = untested

# partial
CoveragePartial = partial

# stub
CoverageStub = stub

# DECK NAME
DeckNameLabel = DECK NAME

# Main {0}
# Arguments: arg0
TabMain = Main { $arg0 }

# Sideboard {0}
# Arguments: arg0
TabSide = Sideboard { $arg0 }

# {0} lands · {1} creatures · {2} other spells
# Arguments: arg0, arg1, arg2
DeckMakeup = { $arg0 } lands · { $arg1 } creatures · { $arg2 } other spells

# empty — tap a card on the left to add it
DeckEmptyHint = empty — tap a card on the left to add it

# Empty the deck
EmptyTheDeck = Empty the deck

# dropped: {0}
# Arguments: arg0
DroppedCards = dropped: { $arg0 }

# The deck needs a name.
DeckNeedsName = The deck needs a name.

# That name is too long (64 characters at most).
DeckNameTooLong = That name is too long (64 characters at most).

# The deck is empty.
DeckIsEmpty = The deck is empty.

# At most {0} different cards per list.
# Arguments: arg0
TooManyLines = At most { $arg0 } different cards per list.

# At most {0} cards in each list.
# Arguments: arg0
TooManyCards = At most { $arg0 } cards in each list.

# {0} is no longer in the card pool.
# Arguments: arg0
CardGoneFromPool = { $arg0 } is no longer in the card pool.

# {0} can no longer be a commander — this deck will save without one.
# Arguments: arg0
CommanderNoLongerEligible = { $arg0 } can no longer be a commander — this deck will save without one.

# {0} cards — a constructed deck wants at least {1}.
# Arguments: arg0, arg1
DeckTooSmall = { $arg0 } cards — a constructed deck wants at least { $arg1 }.

# A sideboard is usually at most {0} cards.
# Arguments: arg0
SideboardTooBig = A sideboard is usually at most { $arg0 } cards.

# {0} lands in {1} cards is thin for this curve.
# Arguments: arg0, arg1
ThinOnLands = { $arg0 } lands in { $arg1 } cards is thin for this curve.

# {0} card is not fully implemented yet and will not play as printed.
# Arguments: arg0
ShakyCard = { $arg0 } card is not fully implemented yet and will not play as printed.

# {0} cards are not fully implemented yet and will not play as printed.
# Arguments: arg0
ShakyCards = { $arg0 } cards are not fully implemented yet and will not play as printed.

# Caption over the small card beside a copy's preview.
#
# The preview draws what the permanent *is* — a copy is a Llanowar
# Elves, mark and all — and the little card beside it is the cardboard
# actually lying there. One word, because the picture says the rest and
# the caption is no wider than the card it labels.
CardUnderneath = UNDERNEATH

# Caption over the small cards beside a permanent's preview that are
# attached to it (CR 701.3; #305): on the table they lie under it and
# may be folded out of sight, which its attachment mark says.
CardAttached = ATTACHED

# Waiting for {0}
#
# `{0}` is a seat's *name* and falls back to [`Phrase::SeatNumbered`],
# so the old wording ("Waiting for seat 1") is what a table with no
# roster still says — and a table that has one says who.
#
# Not [`Phrase::WaitingFor`], which is the same fact reported *about*
# somebody ("wartet auf …") inside a stack entry or a lobby row. This
# is the prompt bar speaking in its own voice, and German conjugates
# the two differently.
# Arguments: arg0
WaitingForPlayer = Waiting for { $arg0 }

# Waiting for {0} players
#
# The bar's line to a seat that has kept while several others still
# decide their opening hands (#257); one other is named, with
# [`Phrase::WaitingForPlayer`]. `{0}` is always two or more, so there
# is no singular to write (German "Spieler" is both anyway).
# Arguments: arg0
WaitingForPlayers = Waiting for { $arg0 } players

# Waiting for the house — {0} is away
#
# The same fact as [`Phrase::WaitingForPlayer`] about a chair the house
# is holding. Since #81 the client *draws* a held chair — a dashed rim
# on its mat — and had no word for one anywhere, so the one place a
# player asks the question ("why is nobody answering?") was answered
# with a name and nothing else.
#
# The indicative is right here and is not the tense
# [`Phrase::LinkStandIn`] uses, which is the distinction worth keeping:
# `SeatIdentity::away` is `SeatKind::StandIn`, so the roster is telling
# this client that the house **is** answering. The banner guesses about
# its own seat and may not say so; this reports another seat and may.
# Arguments: arg0
WaitingForHeldSeat = Waiting for the house — { $arg0 } is away

# You owe mana. Activate mana abilities to pay, or pass.
#
# The sentence a payment window had none of. A CR 605.3a window is an
# ordinary priority round offering mana abilities and nothing else, so
# without this it reads "Your move" over a board with nothing to play —
# which is the same thing the house agent saw before `PlayerView::owed`
# existed, and it passed and lost its spell.
#
# It says what the window *is* and not what is owed: the amount is drawn
# as pips beside the mana pool, where the mana that answers it is also
# drawn, and a number said twice in two registers is a number two things
# have to keep in step. It also stops short of what declining costs —
# countered, or a tax unpaid — because that is the engine's sentence and
# this client does not know which it is.
PayOrPass = You owe mana. Click lands to stage them or choose "Pay now"; nothing taps until you pay.

# "Pay now" in a payment window: stages the client's plan for what is
# still owed, visibly, and sends nothing — the next press pays.
# Arguments: arg0
PayStage = Pay now (tap { $arg0 })

# Go: sends the staged taps and pays the window in one action.
PayGo = Pay

# Take back every tap staged in a payment window.
UnstagePayment = Untap all

# Pass a payment window without paying it.
DeclinePayment = Don't pay

# Pass a cast's payment window: the cast is taken back (CR 601.2h).
DeclinePaymentCast = Don't pay — the spell goes back

# Pass a pact's payment window: "If you don't, you lose the game."
DeclinePact = Don't pay — you lose the game

# A payment the client could not complete: it leaves the window open
# rather than pass it short, which would refuse the payment (#01).
PaymentShort = Not enough mana to pay. Stage more lands, or choose "Don't pay".

# The AI log's heading line for one entry: whose mind it is.
# Arguments: arg0
AiSaidHead = { $arg0 } thinks

# What the model thought before it answered.
# Arguments: arg0
AiSaidThinking = Reasoning: { $arg0 }

# The answer the model chose.
# Arguments: arg0
AiSaidChose = Chose: { $arg0 }

# What the answer cost.
# Arguments: arg0, arg1
AiSaidCost = { $arg0 } tokens, { $arg1 } s

# Private optional creature cast from a resolving Mask ability.
# Arguments: arg0
MaskChoose = Choose a creature to cast face down (X = { $arg0 }), or decline

# Actual receipt with fixed costs shown separately; no invented partition.
# Arguments: arg0, arg1
MaskReceipt = Actual mana payment: { $arg0 }; fixed activation costs: { $arg1 }

# The controlled player's mandatory playable-card choice.
CommandChoose = Choose the card this player must play if able

# An optional face-down cast's explicit confirmation.
ConfirmFaceDown = Cast face down

# Explicitly decline an optional resolving cast.
DeclineCast = Decline casting

# A text change asks two separate words.
ChooseTextReplacement = Choose the old word and its replacement

# Current explicit text-change draft, without rewriting Oracle.
# Arguments: arg0, arg1
TextReplacementSummary = Word change: { $arg0 } → { $arg1 }

# First vocabulary column.
TextOldWord = Old word

# Second vocabulary column.
TextNewWord = New word

# Mandatory mana activation during a resolving instruction.
ChooseRequiredMana = Choose a land mana ability

# Mandatory instruction progress, separate from the compact headline.
# Arguments: arg0
RequiredManaProgress = Required — { $arg0 } activations completed

# Decision controller differs from the resource owner.
# Arguments: arg0
DecidingFor = You decide for { $arg0 }

# A replacement effect turns the affected face-down creature face up.
DamageTurnFaceUp = Turn face up before damage

# Compact fixed-payment guidance when temporary actions share the answer row.
GrantedPaymentHint = Produce mana or pass.

# Mana may be generated before choosing an optional amount to prevent damage.
# Arguments: arg0
PrepareManaPayment = You may generate mana to prevent up to { $arg0 } damage. Pass to choose the amount.

# A variable payment can exceed the damage it could prevent.
# Arguments: arg0, arg1, arg2
ChooseManaPayment = How much mana will you pay? ({ $arg0 }–{ $arg1 }; prevent up to { $arg2 } damage)

# One non-targeted, exact source selection.
ChooseDamageSource = Choose a damage source

# Movement from the stack in the event log.
LogFromStack = from the stack

# Movement onto the stack in the event log.
LogIntoStack = onto the stack

# The hand zone in a source description.
SourceHand = hand

# The library zone in a source description.
SourceLibrary = library

# Incarnation still present in the game.
SourceCurrent = current

# Remembered incarnation that has left its zone.
SourceHistorical = earlier incarnation

# Secondary exact-incarnation label for otherwise similar rows.
# Arguments: arg0
SourceVersion = incarnation { $arg0 }

# Publicly visible spell/ability reference count.
# Arguments: arg0
SourceReferences = { $arg0 } on the stack

# Generic public label for a face-down source.
SourceFaceDown = Face-down source

# No entitled identity is available for the offered source.
SourceUnknown = Unknown source

# Damage decision: `DamageCombat`.
DamageCombat = combat damage

# Damage decision: `DamageNoncombat`.
DamageNoncombat = noncombat damage

# Damage decision: `DamagePart`.
# Arguments: arg0, arg1, arg2, arg3
DamagePart = { $arg0 } → { $arg1 }: { $arg2 } ({ $arg3 })

# Damage decision: `DamageShield`.
# Arguments: arg0
DamageShield = Prevent the next damage; shield remaining: { $arg0 }

# Damage replacement changes each indicated amount without prevention.
# Arguments: arg0
DamageReplaceAmount = Set each indicated source's damage to each recipient to { $arg0 }; this is not prevention

# Reflection adds damage from an exact source to a named player.
# Arguments: arg0
DamageReflect = This effect's source also deals the indicated damage to { $arg0 }

# A shield counter is spent to prevent this event's damage.
DamageShieldCounter = Remove one shield counter and prevent this event's damage

# Damage decision: `DamageEventShield`.
# Arguments: arg0
DamageEventShield = Prevent this event’s damage; remaining: { $arg0 }; excess expires

# Damage decision: `DamagePreventAll`.
DamagePreventAll = Prevent all indicated damage

# Damage decision: `DamagePreventCombat`.
DamagePreventCombat = Prevent all indicated combat damage

# Damage decision: `DamageProtection`.
DamageProtection = Protection prevents the indicated damage

# Damage decision: `DamageRedirect`.
# Arguments: arg0
DamageRedirect = Redirect the damage to { $arg0 }

# Finite redirection names its recipient and remaining capacity.
# Arguments: arg0, arg1
DamageRedirectNext = Redirect damage to { $arg0 }; remaining: { $arg1 }

# Damage decision: `DamageCounter`.
# Arguments: arg0, arg1
DamageCounter = Remove one { $arg0 } counter and try to prevent 1 damage; counters remaining: { $arg1 }

# Damage decision: `DamageSourceShield`.
# Arguments: arg0, arg1
DamageSourceShield = Use the shield against { $arg0 }; leave { $arg1 } damage unprevented

# Damage decision: `DamageShieldLife`.
# Arguments: arg0
DamageShieldLife = ; { $arg0 } gains life equal to the damage prevented

# Prevention assigned to one damage part.
# Arguments: arg0, arg1, arg2
PreventionShare = { $arg0 } — prevent { $arg1 } of at most { $arg2 }

# Damage assigned to redirection rather than prevention.
# Arguments: arg0, arg1, arg2
RedirectionShare = { $arg0 } — redirect { $arg1 } of at most { $arg2 }

# Damage decision: `DamageRule`.
DamageRule = Rule effect

# Explicitly unpreventable damage in a replacement decision.
DamageUnpreventable = ; damage cannot be prevented

# Counter removal allocation can still consume counters without preventing damage.
# Arguments: arg0, arg1
AllocateDamageCounters = Distribute { $arg0 } counter removals — { $arg1 } remaining. Select a source and enter its share.

# A counter-removal share, distinct from guaranteed prevention.
# Arguments: arg0, arg1, arg2
DamageCounterShare = { $arg0 } — apply { $arg1 } counter removals (maximum { $arg2 })

# Choosing between damage replacement and prevention effects.
ChooseDamageEffect = Choose which effect modifies the damage next

# Exact prevention budget and its still unassigned portion.
# Arguments: arg0, arg1
AllocatePrevention = Distribute { $arg0 } prevention — { $arg1 } remaining. Select a source and enter its share.

# Exact redirection budget and its still unassigned portion.
# Arguments: arg0, arg1
AllocateRedirection = Redirect { $arg0 } damage — { $arg1 } remaining. Select a source and enter its share.

# A payment window with no fixed amount owed.
OptionalPayment = Optional payment

# {0} is the player who must sacrifice the chosen permanents.
# Arguments: arg0, arg1
SacrificeForPlayer = { $arg0 } sacrifices the chosen permanents. { $arg1 }

# Owed
#
# The word the mana pool's row opens its second half with, while a
# CR 605.3a window is open. One word and then the pips, beside the pips
# of what is floating, so "owe {2}{G}" and "have {G}" are one glance in
# one register — which is the argument for putting it here rather than
# in the shelf's middle column beside the sentence.
Owed = Owed

# Beside the owed cost on the payment strip: what is still open once
# the pool and the staged taps are counted.
OwedLeft = left

# Waiting
JustWaiting = Waiting

# Keep this hand? (the next mulligan is free)
MulliganFree = Keep this hand? (the next mulligan is free)

# Keep this hand? ({0} taken)
# Arguments: arg0
MulliganTaken = Keep this hand? ({ $arg0 } taken)

# Put {0} card on the bottom
# Arguments: arg0
PutCardOnBottom = Put { $arg0 } card on the bottom

# Put {0} cards on the bottom
# Arguments: arg0
PutOnBottom = Put { $arg0 } cards on the bottom

# Declare attackers
DeclareAttackers = Declare attackers

# Current attack aim and the number already assigned there.
# Arguments: arg0, arg1
AttackAim = Aim: { $arg0 } · { $arg1 } assigned

# Adds undeclared creatures only; other assignments are preserved.
# Arguments: arg0
AttackRemaining = Attack with all remaining → { $arg0 }

# Clears the draft without submitting it.
AttackWithdrawAll = Withdraw all attackers

# The attack chooser's fold row while its rows show (report 25).
AttackListHide = Hide the list ▾

# The fold row while folded. `{0}` is how many rows it holds.
# Arguments: arg0
AttackListShow = Show the list ({ $arg0 }) ▸

# A single unassigned creature.
# Arguments: arg0, arg1
AttackSend = Attack: { $arg0 } → { $arg1 }

# A single assigned creature, naming its existing defender.
# Arguments: arg0, arg1
AttackWithdraw = Withdraw: { $arg0 } → { $arg1 }

# Instructions for the reversible draft.
AttackDraftHint = Choose a defender, then creatures. Withdraw an attacker here to reassign it. Confirm when ready.

# Declare blockers
DeclareBlockers = Declare blockers

# Instructions over the block list (report 31).
BlockDraftHint = Choose an attacker, then the creatures that block it. Choose a blocker again to take it back. Confirm when ready.

# An attacker in the block list, and how many creatures block it so far.
# Arguments: arg0, arg1
BlockAim = Attacker: { $arg0 } · { $arg1 } blocking

# A creature that may block the attacker in aim.
# Arguments: arg0, arg1
BlockSend = Block: { $arg0 } → { $arg1 }

# A creature already blocking, naming the attacker it blocks.
# Arguments: arg0, arg1
BlockWithdraw = Take back: { $arg0 } → { $arg1 }

# Discard {0} card
# Arguments: arg0
DiscardCard = Discard { $arg0 } card

# Discard {0} cards
# Arguments: arg0
DiscardCards = Discard { $arg0 } cards

# Legend rule: keep one
LegendRule = Legend rule: keep one

# card
NounCard = card

# cards
NounCards = cards

# target
NounTarget = target

# targets
NounTargets = targets

# card from your library
NounCardFromLibrary = card from your library

# cards from your library
NounCardsFromLibrary = cards from your library

# card to put on top of your library
NounCardToTop = card to put on top of your library

# cards to put on top of your library
NounCardsToTop = cards to put on top of your library

# card to put into your hand
NounCardToHand = card to put into your hand

# cards to put into your hand
NounCardsToHand = cards to put into your hand

# card to put on the bottom of your library
NounCardToBottom = card to put on the bottom of your library

# cards to put on the bottom of your library
NounCardsToBottom = cards to put on the bottom of your library

# card you may play this turn
NounCardToPlay = card you may play this turn

# cards you may play this turn
NounCardsToPlay = cards you may play this turn

# card to put onto the battlefield
NounCardToBattlefield = card to put onto the battlefield

# cards to put onto the battlefield
NounCardsToBattlefield = cards to put onto the battlefield

# card to put into its owner's graveyard
NounCardToGraveyard = card to put into its owner's graveyard

# cards to put into their owner's graveyard
NounCardsToGraveyard = cards to put into their owner's graveyard

# A private library inspection is acknowledged without choosing a card.
InspectLibrary = Inspect the library cards, then confirm without selecting any.

# Instructions for the optional reveal-or-choose action.
BeholdInstructions = Choose a matching permanent you control or reveal a matching card from your hand. Confirm without a selection to decline.

# Private hand inspection.
InspectHand = Look at the hand, then confirm when finished

# card for the first pile — the rest are the second
NounCardForFirstPile = card for the first pile (the rest are the second)

# cards for the first pile — the rest are the second
NounCardsForFirstPile = cards for the first pile (the rest are the second)

# card from your graveyard — none searches the library instead
NounCardFromGraveyard = card from your graveyard, or none to search your library

# cards from your graveyard — none searches the library instead
NounCardsFromGraveyard = cards from your graveyard, or none to search your library

# card from outside the game
NounCardOutside = card from outside the game

# cards from outside the game
NounCardsOutside = cards from outside the game

# permanent to sacrifice
NounPermanentToSacrifice = permanent to sacrifice

# permanents to sacrifice
NounPermanentsToSacrifice = permanents to sacrifice

# land from which to remove counters
NounLandToRemoveCounters = land to remove counters from

# lands from which to remove counters
NounLandsToRemoveCounters = lands to remove counters from

# land to keep
NounLandToKeep = land to keep

# lands to keep
NounLandsToKeep = lands to keep

# creature to keep
NounCreatureToKeep = creature to keep

# creatures to keep
NounCreaturesToKeep = creatures to keep

# permanent to keep
NounPermanentToKeep = permanent to keep

# permanents to keep
NounPermanentsToKeep = permanents to keep

# hand card to keep
NounCardToKeep = hand card to keep

# cards to keep
NounCardsToKeep = hand cards to keep

# card to discard
NounCardToDiscard = card to discard

# cards to discard
NounCardsToDiscard = cards to discard

# untapped permanent to tap
NounPermanentToTap = untapped permanent to tap

# untapped permanents to tap
NounPermanentsToTap = untapped permanents to tap

# permanent to return to its owner's hand
NounPermanentToReturn = permanent to return to its owner's hand

# permanents to return to their owners' hands
NounPermanentsToReturn = permanents to return to their owners' hands

# card to exile from your graveyard
NounCardToExile = card to exile from your graveyard

# cards to exile from your graveyard
NounCardsToExile = cards to exile from your graveyard

# permanent to leave tapped
NounPermanentToLeaveTapped = permanent to leave tapped

# permanents to leave tapped
NounPermanentsToLeaveTapped = permanents to leave tapped

# object for {0} to enchant: card or token, not a target
# Arguments: arg0
NounNewHostFor = object for { $arg0 } to enchant

# objects for {0} to enchant
# Arguments: arg0
NounNewHostsFor = objects for { $arg0 } to enchant

# The Aura of a new-host question, where it cannot be named.
NounTheAura = the Aura

# permanent to copy (enter as a copy): a choice, not a target
NounPermanentToCopy = permanent to copy

# permanents to copy
NounPermanentsToCopy = permanents to copy

# permanent to untap
NounPermanentToUntap = permanent to untap

# permanents to untap
NounPermanentsToUntap = permanents to untap

# Optional disclosure of one already selected discard.
NounDiscardToReveal = discard to reveal with Nephalia Academy (others stay hidden)

# Optional disclosure of several already selected discards.
NounDiscardsToReveal = discards to reveal with Nephalia Academy (others stay hidden)

# Card to reveal.
NounCardToReveal = card to reveal

# cards to reveal
NounCardsToReveal = cards to reveal

# attacker to join the band (banding, CR 702.22c)
NounAttackerToBand = attacker to join the band

# attackers to join the band
NounAttackersToBand = attackers to join the band

# creature for a pile (Camouflage, the pile unnamed)
NounCreatureForPile = creature for this pile

# creatures for a pile
NounCreaturesForPile = creatures for this pile

# creature for pile {0} of {1} (Camouflage: piles go to attackers at random)
# Arguments: arg0, arg1
NounCreatureForPileOf = creature for pile { $arg0 } of { $arg1 } (each pile blocks an attacker chosen at random)

# creatures for pile {0} of {1}
# Arguments: arg0, arg1
NounCreaturesForPileOf = creatures for pile { $arg0 } of { $arg1 } (each pile blocks an attacker chosen at random)

# creature for the "left" pile (Raging River; the rest go right)
NounCreatureForLeft = creature for the "left" pile (the rest go right)

# creatures for the "left" pile
NounCreaturesForLeft = creatures for the "left" pile (the rest go right)

# Raging River: a label for an attacker, the pile that may block it
ChooseRiverLabel = Choose "left" (first) or "right": only that pile and fliers may block it

# Raging River: the label for the attacker named {0}
# Arguments: arg0
ChooseRiverLabelFor = { $arg0 }: choose "left" or "right"; only that pile and fliers may block it

# Left: {0} — the "left" pile's row of a Raging River label
# Arguments: arg0
RiverLeftRow = Left: { $arg0 }

# Right: {0} — the "right" pile's row of a Raging River label
# Arguments: arg0
RiverRightRow = Right: { $arg0 }

# attacker to block (False Orders' re-block, its blocker unnamed)
NounAttackerToBlock = attacker to block

# attackers to block
NounAttackersToBlock = attackers to block

# attacker for {0} to block
# Arguments: arg0
NounAttackerForBlocker = attacker for { $arg0 } to block

# attackers for {0} to block
# Arguments: arg0
NounAttackersForBlocker = attackers for { $arg0 } to block

# attacker to band with {0} (banding, CR 702.22c): the leader named
# Arguments: arg0
NounAttackerToBandWith = attacker to band with { $arg0 }

# attackers to band with {0}
# Arguments: arg0
NounAttackersToBandWith = attackers to band with { $arg0 }

# Convoke or waterbend: tap permanents to help pay.
#
# Neutral, because the question does not say which keyword asked it:
# both arrive as `TargetPrompt::Convoke`, and they tap different things
# (CR 702.51a creatures, CR 701.67a artifacts and creatures). The
# permanents it may tap are the lit ones, and naming a type here was
# wrong for one of the two.
TapToHelpPay = Tap permanents to help pay — each pays for one

# Delve: exile cards from your graveyard to help pay
DelveToHelpPay = Exile cards from your graveyard to help pay — each pays for one

# Crew N: tap creatures with total power {0} or more
# Arguments: arg0
CrewWithPower = Crew { $arg0 }: tap any number of your other untapped creatures with total power { $arg0 } or more

# Saddle N: pay by tapping creatures with enough effective power.
# Arguments: arg0
SaddleWithPower = Saddle { $arg0 }: tap any number of your other untapped creatures with total power { $arg0 } or more

# Choose up to {0} {1}
# Arguments: arg0, arg1
ChooseUpTo = Choose up to { $arg0 } { $arg1 }

# A resolving effect changes one target at a time.
ChooseNewTarget = Choose a new target

# Selecting nothing preserves the current target of the spell or copy.
ChooseNewTargetOrKeep = Choose a new target, or keep this target without selecting one

# Identifies the current target before asking whether to change it.
# Arguments: arg0, arg1, arg2, arg3
RetargetContext = Target { $arg0 } of { $arg1 }: { $arg2 }. { $arg3 }

# An exact former target, without exposing its internal version number.
TargetBeforeZoneChange = before it changed zones

# A returned card must be chosen anew to replace its earlier identity.
TargetReturnedHint = To target a returned card, select it again below.

# Identifies the original target without repeating selection instructions.
# Arguments: arg0, arg1, arg2
RetargetOriginal = Previous target { $arg0 } of { $arg1 }: { $arg2 }

# A former object target may no longer be visible in the current view.
PreviousTarget = previous target

# Confirm an optional retarget with no selected replacement.
KeepCurrentTarget = Keep current target

# Confirm a selected replacement target.
ChangeTarget = Change target

# The shelf's question while a copy's targets are asked one by one and
# this one may be kept. `{0}` of `{1}`.
# Arguments: arg0, arg1
RetargetKeepOrChange = Target { $arg0 } of { $arg1 }: keep it, or choose a new one

# The same, for a target that may not be kept (another target of the
# copy already holds it, or the engine asks for a new one).
# Arguments: arg0, arg1
RetargetMustChange = Target { $arg0 } of { $arg1 }: choose a new one

# Keep this target and every later one of the copy that may be kept;
# `{0}` is how many that is at most, this one included.
# Arguments: arg0
KeepRemainingTargets = Keep all remaining ({ $arg0 })

# Optional payment permits only zero or the entire tap group.
# Arguments: arg0
ChooseExactlyOrNoTaps = Choose exactly { $arg0 } permanents to tap, or choose none

# Choose {0} {1}
# Arguments: arg0, arg1
ChooseExactly = Choose { $arg0 } { $arg1 }

# Choose {0}–{1} {2}
# Arguments: arg0, arg1, arg2
ChooseBetween = Choose { $arg0 }–{ $arg1 } { $arg2 }

# Choose a creature type
ChooseCreatureType = Choose a creature type

# Choose a basic land type (Phantasmal Terrain)
ChooseBasicLandType = Choose a basic land type

# Choose a card name (Pithing Needle)
ChooseCardName = Choose a card name

# Choose a colour
ChooseColour = Choose a colour

# One or several colours selected during resolution.
ChooseColours = Choose the colour or colours

# Counter amount chosen while an effect resolves.
# Arguments: arg0, arg1, arg2
ChooseCounters = How many { $arg0 }? ({ $arg1 }–{ $arg2 })

# Counter amount with the recipient named.
# Arguments: arg0, arg1, arg2, arg3
ChooseCountersNamed = How many { $arg0 } on { $arg1 }? ({ $arg2 }–{ $arg3 })

# Choose a number ({0}–{1})
# Arguments: arg0, arg1
ChooseNumberIn = Choose a number ({ $arg0 }–{ $arg1 })

# One target's share of damage divided as the player chooses (Fury).
# Arguments: arg0, arg1, arg2, arg3, arg4
DamageShare = Damage to target { $arg0 } of { $arg1 }, { $arg2 } left to divide ({ $arg3 }–{ $arg4 })

# One creature's share of combat damage this player divides
# (CR 510.1c–d, 702.22j–k), when the creatures cannot be named.
# Arguments: arg0, arg1, arg2, arg3, arg4
CombatDamageShare = Combat damage to creature { $arg0 } of { $arg1 }, { $arg2 } left to divide ({ $arg3 }–{ $arg4 })

# The same share, naming the creature dealing the damage and the one
# the share goes to.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6
CombatDamageShareNamed = Combat damage from { $arg0 } to { $arg1 } ({ $arg2 } of { $arg3 }), { $arg4 } left to divide ({ $arg5 }–{ $arg6 })

# Replicate {0}: pay it how many times? ({1}–{2})
# Arguments: arg0, arg1, arg2
ReplicateHowOften = Replicate { $arg0 }: pay it how many times? ({ $arg1 }–{ $arg2 })

# Choose a player
ChoosePlayer = Choose a player

# Choose how to cast
ChooseHowToCast = Choose how to cast

# Choose a pile to put into your hand (Fact or Fiction)
ChoosePileForHand = Choose a pile for your hand; the other goes to your graveyard

# Pile {0}: {1} — one row of a pile choice
# Arguments: arg0, arg1
PileRow = Pile { $arg0 }: { $arg1 }

# no cards — an empty pile's row
EmptyPile = no cards

# Mode {0}
#
# Only where a mode has no printed sentence: a modal trigger that states
# its choice inside one sentence, or a mode that declines.
# Arguments: arg0
CastModeNumber = Mode { $arg0 }

# Back face
CastBackFace = Back face

# Play as a land
CastLandFace = Play as a land

# Miracle
CastMiracle = Miracle

# Flashback
CastFlashback = Flashback

# Beam me up's casting option.
CastBeamMeUp = Beam me up

# Agadeem's Awakening target constraint.
DifferentManaValues = Choose targets with different mana values

# Dash (CR 702.109a), the keyword as the German printing names it
CastDash = Dash

# Mutate (CR 702.140a), the keyword as the German printing names it
CastMutate = Mutate

# Escape (CR 702.138a), the keyword as the German printing names it
CastEscape = Escape

# Choose the source of a casting permission.
CastPermission = Use this casting permission

# A casting permission without a limited allowance.
CastPermissionUnlimited = Use an unrestricted casting permission

# Click a card in your hand
HintClickHand = Click a card in your hand

# Click what you are choosing
HintClickBoard = Click what you are choosing

# Type to narrow the list
HintTypeToFilter = Type to narrow the list

# A permanent's label: it has lost all its abilities (report 21).
NoAbilities = No abilities

# A leveler's label, short (report 30). `{0}` is its level.
# Arguments: arg0
LevelShort = LV { $arg0 }

# The same label under the pointer, in full.
# Arguments: arg0
LevelLong = Level { $arg0 }

# What an empty search box in the question's sheet says (report 16).
FilterPlaceholder = type to search…

# Put these in order
PutInOrder = Put these in order

# The first card in a graveyard ordering will be on top of the others.
OrderGraveyard = Order graveyard cards: first card on top

# Put these back on top: the first is the new top card
OrderOnTop = Put these back on top: the first is the new top card

# Put these on the bottom: the last is the bottom card
OrderOnBottom = Put these on the bottom: the last is the bottom card

# A scry (CR 701.22a): the looked-at cards go back on top or under the
# library, each pile in an order.
ScryPrompt = Scry: keep cards on top or put them on the bottom

# The shared surveil/explore choice: keep cards on top or put them into
# the graveyard, without labeling either operation as the other mechanic.
SurveilPrompt = Keep cards on top or put them into your graveyard

# Library of Leng: cards an effect makes you discard go into the
# graveyard, or on top of your library in an order.
DiscardToLibraryPrompt = Discard: into your graveyard, or on top of your library instead

# The game is over
TheGameIsOver = The game is over

# Pay {0} life? Otherwise it enters tapped
# Arguments: arg0
PayLifeOrTapped = Pay { $arg0 } life? Otherwise it enters tapped

# One-spell spending permission, restricted to the printed mana cost.
UseManaAnyType = Use North Star to pay this spell’s mana cost with mana of any type? Additional costs still apply normally.

# Pay the additional cost?
PayAdditionalCost = Pay the additional cost?

# Optional life payment for a resolving ability.
# Arguments: arg0
PayLife = Pay { $arg0 } life?

# Pay {{0}}?
# Arguments: arg0
PayTax = Pay {"{"}{ $arg0 }{"}"}?

# Pay a price with colour in it; the cost renders its own braces.
# Arguments: arg0
PayMana = Pay { $arg0 }?

# The player may activate mana abilities before paying a pact.
# Arguments: arg0
PayPact = Pact: pay { $arg0 }. Make mana now? If you decline or finish without enough mana, you lose the game.

# Cast it for its miracle cost?
CastForMiracle = Cast it for its miracle cost?

# Cast it without paying its mana cost? (cascade)
CastWithoutPaying = Cast it without paying its mana cost?

# Cast it now, paying its mana cost? (Conduit of Worlds)
CastPaying = Cast it now, paying its mana cost? You make the mana first.

# {0} offers a draw. Accept?
#
# Named rather than passive, and the name is the whole of the repair:
# at a duel "a draw was offered" is obvious, and at a table of four it
# is a question nobody can answer.
# Arguments: arg0
DrawOfferedBy = { $arg0 } offers a draw. Accept?

# Put your commander into the command zone?
CommanderToCommandZone = Put your commander into the command zone?

# Command zone instead of your hand? (CR 903.9b)
CommanderInsteadOfHand = Your commander would go to your hand. Command zone instead?

# Command zone instead of your library? (CR 903.9b)
CommanderInsteadOfLibrary = Your commander would go into your library. Command zone instead?

# Choose a one-shot regeneration shield instead of the permanent's own regeneration.
UseRegenerationShield = Use up a regeneration shield? No uses the permanent's own regeneration and keeps the shield.

# Choose a shield counter instead of an applicable regeneration replacement.
UseShieldCounter = Use a shield counter instead of regeneration?

# Choose damage clearing instead of another destruction replacement.
UseDamageClearingReplacement = Replace destruction by removing all marked damage? No uses another applicable replacement.

# Order an additive counter replacement; declining applies another replacement first.
# Arguments: arg0
ApplyAdditionalCountersNext = Apply { $arg0 } additional counters of each kind next? No applies another replacement first.

# Use the optional part of this ability? ("you may …")
UseTheOptionalAbility = This ability is optional. Use it?

# Your card goes into your library: the top, or the bottom? (Subtlety)
TopOfLibraryOrBottom = Your card goes into your library. On top? (No puts it on the bottom.)

# One card of a named card type into the hand, or none (Atraxa).
# Arguments: arg0
TakeOneOfType = Put up to one { $arg0 } card into your hand

# Cast the discovered card for free, or take it into the hand?
CastDiscovered = You discovered this card. Cast it without paying its mana cost? (No puts it into your hand.)

# A turn replacement with an explicit turn cost, not a free optional effect.
SkipTurnToUntap = Skip this turn to untap this permanent?

# Yes or no?
YesOrNo = Yes or no?

# Offer a draw
OfferADraw = Offer a draw

# Concede
Concede = Concede

# A lobby row's button: watch a running table (`docs/protocol.md`
# §"Spectators").
Watch = Watch

# The game menu's way out for a spectator.
StopWatching = Stop watching

# How many watch a table. `{0}` the count.
# Arguments: arg0
SpectatorCount = { $arg0 } watching

# A new room's option: whether players without a chair may watch.
AllowSpectators = Allow spectators

# The lobby's line after a spectator leaves a table.
StoppedWatching = You stopped watching

# The table's badge for a spectator: nothing here is theirs to play.
YouAreWatching = You are watching

# Concede, armed and waiting for the second press.
ConcedeConfirm = Concede? Press again

# The game menu's way up from a running table without conceding
# (report 33).
LeaveTable = Leave the table

# The lobby's note after a player left a running table from its menu.
LeftTheTable = You left the table

# Leaving a networked table, armed: the house plays the chair on.
LeaveTableConfirm = Leave? The house plays on for you. Press again

# Leaving a game this client hosts offline, armed: it ends with it.
LeaveTableConfirmOffline = Leave? This game ends. Press again

# The game log panel's title (#262). Not a `Log…` phrase: those are the
# book's own sentences, which `gamelog` writes; these are the panel's.
GameLogTitle = Game log

# The game log panel before its first line has arrived.
GameLogEmpty = Nothing has happened yet

# The game log panel has been scrolled up, and lines arrived under it.
GameLogNewBelow = New lines below

# The same pill on a list scrolled well up with nothing new under it:
# pressing it goes back to the newest line.
GameLogLatest = Back to the latest

# The head of the sheet that holds up cards another seat revealed.
# `{0}` is the seat's name. Present tense, unlike the log's line: the
# sheet stands while the cards are being shown (CR 701.20a).
# Arguments: arg0
RevealedBy = { $arg0 } reveals

# Under that head: how many more reveals wait behind this one.
# Arguments: arg0
RevealedWaiting = { $arg0 } more to come

# Under that head, while nothing waits: how to see a card larger.
RevealedHint = Point at a card to enlarge it

# The reveal sheet's answer while more reveals wait behind it: puts
# this one away and stands the next one up.
RevealNext = Next

# The reveal sheet's answer for the last reveal standing: puts it away.
RevealDone = Done

# The indicator that says this seat is not being asked right now.
HoldingPriority = Not asking you

# The button that cancels a running hold.
HoldRelease = Ask me again

# The armed button for a spell or a land: pressing it sends the card.
ArmedPlay = Play this card

# Explicit confirmation of the ability shown in the retained dialog.
ArmedActivate = Activate ability

# The armed button for a spell whose mana still has to be tapped.
#
# `{0}` is the *price* — the spell's mana cost, tax and all — and it is
# one of the two placeholders here never filled with a string (the other
# is [`Self::ArmedSuspend`], which prices the same way): the row
# splits the phrase at it and draws the cost as mana pips, so a
# translation is free to put the price first (as the German does) and
# still gets the discs in the right place. It used to read "Tap 3",
# which counted the client's own lands instead of naming what the
# spell costs.
# Arguments: arg0
ArmedPayAndCast = Pay { $arg0 } and cast

# Confirm an ability and the automatic taps that pay its mana.
# Arguments: arg0
ArmedPayAndActivate = Pay { $arg0 } and activate

# The armed button for suspending a card whose cost still has to be tapped.
#
# `{0}` is the suspend cost, drawn as pips like [`Self::ArmedPayAndCast`]
# — and it is the *suspend* cost, which is a different number from the
# card's own: Ancestral Vision prints no mana cost and suspends for
# `{U}`.
# Arguments: arg0
ArmedSuspend = Pay { $arg0 } and suspend

# Heading of the public suspend queue.
SuspendedCards = Suspended cards

# Card name, owner and remaining time counters.
# Arguments: arg0, arg1, arg2
SuspendedCard = { $arg0 } · { $arg1 }{"\u000A"}{ $arg2 } time counters · inspect

# Confirm a suspend action with no outstanding mana payment.
ArmedSuspendNow = Suspend this card

# The button that puts an armed deed back, with nothing sent.
ArmedCancel = Not yet

# Aim next
AimNext = Aim next

# Attack
Attack = Attack

# Block
Block = Block

# None
DeclareNone = None

# Keep
KeepHand = Keep

# Mulligan
TakeMulligan = Mulligan

# OK
# Target selection explanation or explicit shortcut.
# Arguments: arg0
TargetingFor = Targets for { $arg0 }

# Target selection explanation or explicit shortcut.
TargetingSecond = Second target clause

# Target selection explanation or explicit shortcut.
# Exact target count, available choices and current selection.
# Arguments: arg0, arg1, arg2, arg3
TargetingExactly = Selected: { $arg3 }/{ $arg0 } • { $arg2 } available targets

# Permitted target range and current selection.
# Arguments: arg0, arg1, arg2, arg3
TargetingChoices = Choose { $arg0 }–{ $arg1 } • { $arg2 } legal choices • { $arg3 } selected

# Target selection explanation or explicit shortcut.
# Arguments: arg0
TargetingBatch = Confirm for all { $arg0 }

# Target selection explanation or explicit shortcut.
# Arguments: arg0
TargetingBatchHint = { $arg0 } identical triggers waiting. Confirm once, or use this selection for all { $arg0 }.

# Target selection explanation or explicit shortcut.
# Arguments: arg0
TargetingSelected = Selected: { $arg0 }

# Clear the target list's controller filter.
TargetingAllSeats = All players

# Current page and total legal choices in the filtered list.
# Arguments: arg0, arg1, arg2
TargetingPage = Choices { $arg0 }–{ $arg1 } of { $arg2 }. You can also select on the table.

# Cast with its optional additional kicker cost paid.
CastKicked = With kicker

# Cast face down using disguise.
# Arguments: arg0, arg1, arg2
CastDisguise = Disguise — 2/2, ward { $arg2 }

# Face-up special action.
TurnFaceUp = Turn face up

# Unlock {0}: a Room's special action, naming the door it opens.
# Arguments: arg0
UnlockDoor = Unlock { $arg0 }

# Cast using prototype characteristics.
CastPrototype = Prototype

# Confirm the current selection.
ConfirmOk = OK

# Pass
PassPriority = Pass

# Skip the rest of the turn
SkipTheTurn = Skip turn

# Hold priority until the stack is empty — the ledge's own button.
#
# [`Phrase::ActHoldForStack`] is the same hold *described* in the
# keyboard menu, where a line has room to say what a key does. This is
# the label on a 28-pixel button, so it says what pressing it does and
# nothing else.
ResolveTheStack = Resolve the stack

# Day
DesignationDay = Day

# Night
DesignationNight = Night

# In order
SortByPlace = In order

# By name
SortByName = By name

# By cost
SortByCost = By cost

# By type
SortByType = By type

# Token
IsToken = Token

# Your move
YourMove = Your move

# You may respond
#
# The same priority window, on somebody else's turn. "Your move" there
# reads as "it is your turn", which it is not.
YouMayRespond = You may respond

# Resolve the stack while retaining manual choices.
StackRun = Resolve stack

# Stack automation control.
StackRunTo = Resolve up to selection

# Stack automation control.
StackStopHint = Stop before the marked ability. Click again to clear.

# Stack automation control.
StackSelectHint = Select an entry to stop before it.

# Stack automation control.
StackAbilityPolicy = For this card ability

# Stack automation control.
StackAlwaysPass = Always pass

# Stack automation control.
StackAsk = Always ask

# Stack automation control.
StackAlwaysYes = Always yes

# Stack automation control.
StackAlwaysNo = Always no

# Stack automation control.
StackResetRules = Reset ability automation

# Stack automation control.
StackNoPolicy = This entry has no reusable card ability.

# Stack automation control.
StackPolicyHint = Targets and payments still require your decision.

# Stack panel title.
# Arguments: arg0, arg1
StackRuleName = { $arg0 } · Ability { $arg1 }

# Holographic card finish.
FinishHolographic = Holographic

# Glitter card finish.
FinishGlitter = Glitter

# Galaxy card finish.
FinishGalaxy = Galaxy

# Stack panel title.
StackTitle = Stack

# Spell
StackSpell = Spell

# Ability
StackAbilityBare = Ability

# Ability · {0}
# Arguments: arg0
StackAbility = Ability · { $arg0 }

# +{0} more
# Arguments: arg0
StackMore = +{ $arg0 } more

# Seat {0}
# Arguments: arg0
SeatNumbered = Seat { $arg0 }

# You ({0})
# Arguments: arg0
YouNamed = You ({ $arg0 })

# You
You = You

# Aimed at {0} ({1} of {2})
# Arguments: arg0, arg1, arg2
AimedAt = Aimed at { $arg0 } ({ $arg1 } of { $arg2 })

# {0} declared
# Arguments: arg0
DeclaredCount = { $arg0 } declared

# {0}: {1} attacking, {2} unblocked
# Arguments: arg0, arg1, arg2
IncomingAt = { $arg0 }: { $arg1 } attacking, { $arg2 } unblocked

# you
IncomingYou = you

# a seat
ASeat = a seat

# a permanent
APermanent = a permanent

# nothing
AimingAtNothing = nothing

# YOU
RailYou = YOU

# OPPONENT
RailOpponent = OPPONENT

# Connection lost — reconnecting…
#
# Said for the first [`crate::reconnect::Retry::PATIENCE`] seconds of a
# drop — or for the whole of it, at a table that hands no chair over and
# at one this client was never told the window of.
# [`crate::reconnect::Window`] is which, and it is the only sentence
# that is true at all three.
#
# The wording is deliberately not "the game is over": the table is still
# there and the seat is resumed from where it left off. What it used to
# give as the reason was that "the engine's decision clock does not run
# for a seat with no socket" — true, and the wrong clock. The
# **reconnect** clock is running the whole time this is on screen, and
# when it expires the house takes the chair. That is why this sentence
# is the short one now and does not stand alone for two minutes.
LinkLost = Connection lost — reconnecting…

# Still reconnecting — the house will answer for your seat until you are back.
#
# The second form, once a drop has outlived
# [`crate::reconnect::Retry::PATIENCE`]. It says the consequence the
# first one leaves out: `Session::stand_in` gives the chair to the
# house after `HouseRules::reconnect_window_secs`, and `hand_back`
# returns it the moment the player is attached again.
#
# **The tense is still the finding, and half its reason has gone.** "The
# house *is* playing your seat" is the sentence this obviously wants and
# is the one thing that cannot be written: a client is disconnected for
# exactly the span it would have to count, so the moment of the handover
# is not observable from here however much it knows. That part has not
# changed. What has is that the window itself **is** known now —
# `GameStatic::reconnect_secs` since `VIEW_VERSION` 26 — so this is no
# longer shown at a table where the handover is not coming at all, and
# no longer shown late at one whose window is shorter than the cap.
# "Will answer" is true whenever it is on screen, which is what
# [`crate::reconnect::Retry::brief`] is for.
#
# The German says it in the present, which is not a drift: German
# present carries the near future, and "wird … antworten" reads as a
# prediction where the English "will" reads as a rule.
#
# Since 10.10.2026 the house sits down only while another player is at
# the table, after the table's wait (`{0}`, m:ss, counted from the
# loss); alone with the house the game pauses instead
# ([`Phrase::LinkPaused`]).
# Arguments: arg0
LinkStandIn = Still reconnecting — the table waits { $arg0 } for you, then the house plays your seat until you are back.

# Still reconnecting, at a table with no other player: nothing moves
# until this seat is back.
LinkPaused = Still reconnecting — the game is paused until you are back.

# The banner over the table while another player's connection is lost:
# `{0}` the player, `{1}` what is left of the table's wait (m:ss),
# ticking once a second.
# Arguments: arg0, arg1
LostWaiting = { $arg0 } lost the connection – waiting { $arg1 }

# The banner while a lost player's game is paused (nobody else at the
# table but this seat's own view, e.g. a teammate's spectator).
# Arguments: arg0
LostPaused = { $arg0 } lost the connection – the game is paused until they are back

# The banner once the wait is out: the house plays the chair until its
# player is back.
# Arguments: arg0
LostHousePlays = The house AI plays for { $arg0 }

# The table cannot be reached. Rejoin from the lobby.
#
# After the retry schedule runs out. "That game no longer exists" reaches
# a client as a refusal string rather than as a state it can match on, so
# this is what a client says when it cannot tell the two apart.
LinkGaveUp = The table cannot be reached. Rejoin from the lobby.

# The table refused this client's protocol, and speaks a newer one
# (#271). No numbers: the log has both, and the player can act only on
# which side is behind.
ClientOutdated = Your client is out of date: update it to join.

# The table refused this client's protocol, and speaks an older one
# (#271).
TableOlder = This table runs an older version than your client.

# A card has no candidate satisfying its required targets (#112).
CardHasNoTarget = Nothing left for this card to target

# Timing prevents playing a card (#112).
CardWrongTime = This card cannot be played in this window

# A "choose one" spell whose printed-price mode has no legal target,
# so only its mode with a price of its own (overload) can be cast, and
# the mana in reach does not pay that price (#00).
OnlyDearerModeLeft = No legal target for its normal mode, so only its costlier mode (such as overload) can be cast, and your mana does not pay for that

# The current resources do not cover the card's costs (#112).
CardCostsUnavailable = The resources to pay for this card are not available

# The engine no longer offers that.
#
# An armed deed re-checked against the current `LegalActions` and gone:
# the question moved on between the tap that armed it and the tap that
# would have sent it. Deliberately not an apology and not a diagnosis —
# the player's next action is to look at what is offered now.
DeedWithdrawn = The engine no longer offers that

# The engine no longer offers that way of casting it.
#
# The same, one level in: the card is still castable and the *mode*
# picked out of `ChooseCastMode` is not, which is a different sentence
# because the card is still worth looking at.
CastModeWithdrawn = The engine no longer offers that way of casting it

# A land the plan counted on can no longer be tapped.
#
# One of six a mana run gives up with. They are six sentences and not
# one, because each names a different thing to do next, which is the
# only justification a refusal has for existing: a land that went away
# means tap something else, a spell that is no longer castable means
# the mana is floating and the turn is not lost.
PlanLandGone = A land the plan counted on can no longer be tapped

# That source cannot make the colour the plan wanted.
PlanColourGone = That source cannot make the colour the plan wanted

# The mana is up but the spell is not castable.
PlanSpellRefused = The mana is up but the spell is not castable

# The mana is up but the card cannot be suspended.
PlanSuspendRefused = The mana is up but the card cannot be suspended

# The run lost the card it was paying for.
PlanCardGone = The run lost the card it was paying for

# The game asked something else.
#
# A mana ability does not use the stack, so priority never leaves the
# seat in the middle of a plan (CR 605.3a); a different question means
# the game moved on without this client and the plan is void.
PlanQuestionChanged = The game asked something else

# Mana pool
ManaPool = Mana pool

# A seat's plate and chip, said in words (tooltip, accessible name).
# Arguments: arg0
PlateLife = { $arg0 } life

# Cards in a seat's hand.
# Arguments: arg0
PlateHand = { $arg0 } in hand

# Cards in a seat's library.
# Arguments: arg0
PlateLibrary = { $arg0 } in library

# Cards in a seat's graveyard.
# Arguments: arg0
PlateGraveyard = { $arg0 } in graveyard

# Cards in a seat's public exile.
# Arguments: arg0
PlateExile = { $arg0 } in exile

# Poison counters on a seat.
# Arguments: arg0
PlatePoison = { $arg0 } poison

# Public radiation counter total.
# Arguments: arg0
PlateRad = { $arg0 } radiation

# Optional energy payment.
# Arguments: arg0
PayEnergy = Pay { $arg0 } energy counters?

# Proliferate adds every counter kind, including harmful ones.
# Arguments: arg0
ProliferatePlayer = Give { $arg0 } one additional counter of each kind already present?

# Singular proliferate selection.
NounProliferatePermanent = permanent to give one more counter of each kind

# Plural proliferate selection.
NounProliferatePermanents = permanents to give one more counter of each kind

# Public speed after starting engines.
# Arguments: arg0
PlateSpeed = Speed { $arg0 }/4

# Four speed, enabling max-speed abilities.
PlateMaxSpeed = Max speed (4/4)

# Public energy counter total.
# Arguments: arg0
PlateEnergy = { $arg0 } energy

# The most combat damage one commander has dealt a seat.
# Arguments: arg0
PlateCommander = { $arg0 } commander damage

# The monarch designation (CR 724), beside the crown.
PlateMonarch = Monarch

# A seat that is out of the game.
PlateLost = Out of the game

# The seat whose turn it is (the ☀ tag's words).
PlateTurn = Their turn

# The seat the table waits for (the ⌛ tag's words).
PlateWaiting = The table waits for them

# Badge on the player whose turn is in progress (not merely priority).
ActiveTurn = Turn

# The creature type publicly named for this permanent.
# Arguments: arg0
ChosenType = Chosen: { $arg0 }

# The card name publicly chosen for this permanent (Pithing Needle).
# Arguments: arg0
ChosenName = Named: { $arg0 }

# A Room's doors still locked, by name.
# Arguments: arg0
LockedDoors = Locked: { $arg0 }

# Tap for {0}, spendable only on some spells
#
# Cavern of Souls and its kin. The label has to say *both* halves: a
# button reading only "Tap for WUBRG" beside another reading "Tap for
# {C}" is two offers a player cannot tell apart, and the restricted one
# is usually the reason the land is in the deck.
# Arguments: arg0
TapForRestricted = Tap for { $arg0 } (restricted)

# Tap for X mana ({0})
#
# Harabaz Druid: *"add X mana of any one color, where X is the number of
# Allies you control"*. The `X` is the card's own letter and is left
# standing, because what it comes to is the engine's arithmetic and no
# client counts it — what the row can say honestly is that the amount is
# a count, and which colours it is a count of. Without it the ability
# fell back to its cost and drew a row reading "{T}" beside a cost
# column reading "{T}".
# Arguments: arg0
TapForVariable = Tap for X mana ({ $arg0 })

# to confirm
#
# The footer while a row is armed, beside a keycap carrying that row's
# digit — the key that armed it is the key that sends it, and there is
# no second confirm key to learn. The digit used to be `{0}` inside this
# sentence; it is drawn as the key it is now, so the phrase is the word
# that goes with the cap and nothing else.
#
# It read "again", which is true of the gesture and silent about the
# consequence. This half of the footer is the only place the sheet says
# that the next press *sends* something, so it says that instead.
SheetPressAgain = to confirm

# A digit picks
#
# The same footer with nothing armed, and the one half that carries no
# cap: the sentence is about *any* digit, and a key drawn there would
# name the first row rather than the gesture. It names the gesture
# rather than a range, because how many rows there are is on the sheet
# already.
SheetDigitPicks = A digit picks

# close
#
# The other end of the footer, beside the cap for the key itself. The
# key's own name is never translated and never spelled here: it comes
# from `prefs::Chord::display`, the same reader the settings screen
# draws every binding with.
SheetCloses = close

# More — page {0} of {1}
#
# The tenth row, which exists only for a permanent with more than nine
# things to do: a land under a Chromatic Lantern is granted a mana
# ability on top of whatever it prints. It carries `0` rather than a
# digit, being the one key on that row of the keyboard that names no
# ability.
# Arguments: arg0, arg1
SheetMorePage = More — page { $arg0 } of { $arg1 }

# Tap for {0}
# Arguments: arg0
TapFor = Tap for { $arg0 }

# any color
#
# What the *cards* say. Five discs in a row is not "any colour" — it is
# five claims a player has to add up — and a land that makes all five is
# making one offer, not five.
AnyColor = any color

# {0} or {1}
#
# The tail of a list of colours a source may choose between. Two or
# three symbols read at a glance where five do not, which is why this
# exists beside [`Phrase::AnyColor`] rather than instead of it.
# Arguments: arg0, arg1
OrLast = { $arg0 } or { $arg1 }

# Granted ability
#
# An ability another permanent handed this one. It has no position on
# the card to be numbered by, so "Ability 1" would be a lie about a
# printed ability that is also there.
GrantedAbility = Granted ability

# The head of the sheet under a permanent's preview that lists what
# other permanents have granted it, one grantor's sentence a paragraph.
GrantedAbilities = Granted abilities

# Rules text unavailable
NoRulesTextHere = Rules text unavailable

# Startup and game preparation use the same original portal scene.
ArrivalLogin = At the threshold

# The table is being prepared beneath the cover.
ArrivalTable = One table. A new story.

# Preparation failed; the player can leave the table.
ArrivalHeld = The journey is on hold

# The prepared destination is being revealed.
ArrivalPortal = The portal opens

# Counts human seats that have acknowledged render readiness.
# Arguments: arg0, arg1
ArrivalPlayers = Waiting for the table  ·  { $arg0 } / { $arg1 } ready

# Required fonts and local assets.
ArrivalWorld = Awakening the world

# All initial gateway probes, including failed replies.
ArrivalGateways = Checking gateways

# Opening snapshot and visible card textures.
ArrivalCards = Preparing cards and table

# The render thread is preparing the covered scene.
ArrivalGraphics = Setting the scene

# Local preparation has completed.
ArrivalReady = Everything is ready

# The three chapters of the game entrance.
ArrivalTableSteps = PREPARE    ·    GATHER    ·    ENTER

# The three chapters of startup.
ArrivalLoginSteps = WORLD    ·    CONNECTION    ·    ARRIVAL

# Cancel preparation, including a failed connection.
ArrivalLeave = Return to lobby  ·  Esc

# Direct seat launches have no lobby to return to.
ArrivalLeaveTable = Leave table  ·  Esc

# A recoverable preparation timeout.
ArrivalTimeout = Loading could not finish. Check the connection and try again.

# Required local files are missing; the default font keeps this readable.
ArrivalAssetsFailed = Required interface files could not load. Please restart or reinstall the client.

# A failed render pipeline cannot safely reveal the scene.
ArrivalGraphicsFailed = The graphics could not be prepared. Please restart the client.

# Taking your seat
VeilTakingSeat = Taking your seat

# Talking to the gateway
VeilTalking = Talking to the gateway

# The same veil offline, where there is nobody to talk to.
VeilWorking = One moment

# A chair that plays for nobody but itself.
SeatSideNone = no team

# A chair's team, by number.
# Arguments: arg0
SeatSide = team { $arg0 }

# The game is over and this seat won it.
YouWon = You won

# The game is over and this seat did not.
YouLost = You lost

# The game is over and this seat's team won it.
# Arguments: arg0
YourTeamWon = Team { $arg0 } wins — yours

# The game is over and another team won it.
# Arguments: arg0
TheirTeamWon = Team { $arg0 } wins

# Nobody won.
TheGameIsADraw = The game is a draw

# The end screen's verdict for a game an admin ended (beta.8): nobody
# won or lost.
GameClosedVerdict = The game was ended

# The line under it: `{0}` the admin's reason.
# Arguments: arg0
GameClosedReason = An admin ended it: { $arg0 }

# Why a game ended: one seat outlived every other
# (`EndReason::LastPlayerStanding`).
EndedLastPlayer = Only one player left in the game

# Why a game ended: one team outlived every other
# (`EndReason::LastTeamStanding`).
EndedLastTeam = Only one team left in the game

# Why a game ended: a card or an emblem declared a winner
# (`EndReason::EffectWin`).
EndedByEffect = An effect decided it

# Why the reading seat lost: its life total (`LossCause::Life`,
# CR 104.3b).
#
# Every loss line is in the past tense and says only what
# `SeatView::loss` records: the rule that took the seat out, never how
# it got there.
LostLifeYou = Your life fell to 0 or less

# Why another seat lost: its life total. `{0}` is the seat's name.
# Arguments: arg0
LostLifeOther = { $arg0 }'s life fell to 0 or less

# Why the reading seat lost: a draw from an empty library
# (`LossCause::EmptyDraw`, CR 104.3c).
LostEmptyDrawYou = You tried to draw from an empty library

# Why another seat lost: a draw from an empty library.
# Arguments: arg0
LostEmptyDrawOther = { $arg0 } tried to draw from an empty library

# Why the reading seat lost: poison (`LossCause::Poison`, CR 104.3d).
LostPoisonYou = You had ten or more poison counters

# Why another seat lost: poison.
# Arguments: arg0
LostPoisonOther = { $arg0 } had ten or more poison counters

# Why the reading seat lost: one commander's combat damage
# (`LossCause::CommanderDamage`, CR 903.10a).
LostCommanderDamageYou = You took 21 or more combat damage from one commander

# Why another seat lost: one commander's combat damage.
# Arguments: arg0
LostCommanderDamageOther = { $arg0 } took 21 or more combat damage from one commander

# Why the reading seat lost: it conceded (`LossCause::Conceded`,
# CR 104.3a).
LostConcededYou = You conceded

# Why another seat lost: it conceded.
# Arguments: arg0
LostConcededOther = { $arg0 } conceded

# Why the reading seat lost: an effect said so (`LossCause::Effect`,
# CR 104.3e), such as a pact left unpaid.
LostEffectYou = An effect made you lose

# Why another seat lost: an effect said so.
# Arguments: arg0
LostEffectOther = An effect made { $arg0 } lose

# The reading seat's clock answered its last decision
# (`HouseAnswer::Clock`): the socket was there and the time ran out.
# Beside a loss, this is a game lost to the clock.
HouseClockYou = Your time ran out, and the house answered your last decision

# Another seat's clock answered its last decision.
# Arguments: arg0
HouseClockOther = { $arg0 }'s time ran out, and the house answered their last decision

# The house answered the reading seat's last decision while it had no
# socket (`HouseAnswer::StandIn`).
HouseStandInYou = You were not connected, and the house answered your last decision

# The house answered another seat's last decision while it had no
# socket.
# Arguments: arg0
HouseStandInOther = { $arg0 } was not connected, and the house answered their last decision

# The gateway did not answer at all. `{0}` is the transport's word.
# Arguments: arg0
GatewayNoAnswer = the gateway did not answer: { $arg0 }

# It answered, but with a bare status. `{0}` is that status.
# Arguments: arg0
GatewayAnswered = the gateway answered { $arg0 }

# The offline duel could not be opened.
NoOfflineDuel = could not start the offline duel

# Leaving the builder would drop an edit.
UnsavedChanges = unsaved changes — press again to leave

# The deck already holds as many of that card as it may.
NoRoomForCopy = no room for another copy of that

# The table could not be reached. `{0}` is what went wrong.
# Arguments: arg0
CouldNotReachTable = could not reach the table: { $arg0 }

# Cards the engine is showing this seat — a search, a scry, a reveal.
BrowseLooking = Shown

# The zone sheet's name for a card still in this seat's own hand.
BrowseHand = Hand

# Offered battlefield cards inside a mixed-zone question.
BrowseBattlefield = Battlefield

# A graveyard.
BrowseGraveyard = Graveyard

# A public exile pile.
BrowseExile = Exile

# A command zone.
BrowseCommand = Command

# The tab that shows every zone at once.
BrowseAll = All

# A zone with nothing in it, or a filter that matched nothing.
BrowseEmpty = nothing here

# The search box above the list.
#
# It said "by name" until the box learned the query language, which was
# a placeholder telling the player the box could do less than it can:
# a bare word reaches the type line too, and `t:creature pow>=4` works
# here. What it names is what a *bare* word looks at, because that is
# the reading a player gets without being taught anything.
BrowseFilter = Name, type…

# How much of the answer is assembled, when the question takes a range.
# `{0}` is how many are chosen, `{1}` the most it will take.
# Arguments: arg0, arg1
BrowseTallyUpTo = { $arg0 } of up to { $arg1 } chosen

# The same, for a question that names one number. `{0}` of `{1}`.
# Arguments: arg0, arg1
BrowseTallyExact = { $arg0 } of { $arg1 } chosen

# Sends the answer the dialog has assembled.
BrowseConfirm = Confirm

# The gear inside a search box, and the heading over what it opens.
FilterBuild = Set up the filter

# The row at the foot of the list that adds another condition.
FilterAdd = Add a condition…

# Closes the builder. The string is already in the box, so there is
# nothing for this to confirm.
FilterDone = Done

# Empties the builder, and with it the box.
FilterClear = Clear

# Beside a branch the controls cannot take apart.
FilterAsTyped = as typed

# One line under the rows when any of them asks something this list
# cannot answer. One line and not one per row: it is the same fact.
FilterUnanswerable = A condition cannot be checked here — the list stays empty while it stands.

# Chips are narrowing this list as well: {0}
# Arguments: arg0
FilterAlsoChips = Chips are narrowing this list as well: { $arg0 }

# Playable only
FilterChipPlayable = playable only

# mana value {0}
# Arguments: arg0
FilterChipCmc = mana value { $arg0 }

# mana value {0} or more
# Arguments: arg0
FilterChipCmcUp = mana value { $arg0 } or more

# The five kinds of condition, which are the five kinds of control.
FilterKindText = Text

# A condition about colours.
FilterKindColor = Colour

# A condition about a number.
FilterKindNumber = Number

# A condition about a yes-or-no property.
FilterKindFlag = Property

# A condition about a mana cost.
FilterKindCost = Cost

# A bare word, which looks at every line of the card.
FilterKeyLoose = Anywhere

# The card's name.
FilterKeyName = Name

# The whole name and nothing else.
FilterKeyExact = Exact name

# What the card says.
FilterKeyOracle = Rules text

# The type line.
FilterKeyType = Type

# The card's own colours.
FilterKeyColor = Colour

# Its colour identity (CR 903.4).
FilterKeyIdentity = Colour identity

# The symbols of its mana cost.
FilterKeyMana = Mana cost

# Its mana value.
FilterKeyManaValue = Mana value

# Printed power.
FilterKeyPower = Power

# Printed toughness.
FilterKeyToughness = Toughness

# Printed starting loyalty.
FilterKeyLoyalty = Loyalty

# A colour reading: the card has these and may have others.
FilterAtLeast = at least

# A colour reading: the card has these and no others.
FilterExactly = exactly

# A colour reading: the card has no colour outside these.
FilterAtMost = at most

# What a colon means on a mana cost.
FilterContains = contains

# Swaps the colour pips for a number of them.
FilterCount = Count

# More than one colour, whatever they are.
FilterMulticolor = multicoloured

# An even mana value.
FilterEven = even

# An odd one.
FilterOdd = odd

# A card this build can actually play.
FlagPlayable = playable

# A card that may lead a commander deck.
#
# Not `IsCommander`, which is the deck builder saying that *this* card
# is the one — it carries a tick for that reason and a filter label
# must not.
FlagCommander = commander

# A basic land.
FlagBasic = basic land

# A card with two faces.
FlagDfc = double-faced

# Sends the *empty* answer, which is the only way out a question whose
# minimum is zero has — there is no cancel action on the wire.
#
# It said `Cancel` / `Abbrechen` until somebody read it out loud. The
# word was a small lie and the code already knew it: the handler clears
# the selection and then **confirms**, so a player who believed they had
# backed out of the question had in fact answered it, and the game had
# moved on without them. Naming the answer instead of the gesture is the
# fix — a button on a question says what it sends.
BrowseNone = None

# How an ordering is answered.
BrowseOrderHint = tap a card, then the card it goes in front of or a pile's end

# An arrangement's pile that goes back on top of the library, its
# first card the new top card.
ArrangeLibraryTop = On top of the library

# An arrangement's pile that goes under the library, its last card the
# new bottom card.
ArrangeLibraryBottom = Under the library

# An arrangement's pile that goes into the graveyard.
ArrangeGraveyard = Into the graveyard

# The cards of an arrangement not yet put in any pile.
ArrangeUnplaced = Not placed yet

# The end of a pile, while a card is held: a tap puts it there.
ArrangePutHere = Put it here

# A pile nothing has been put in yet, while no card is held.
ArrangeEmptyPile = Empty

# A zone belonging to a seat. `{0}` is the zone, `{1}` the seat.
# Arguments: arg0, arg1
BrowseZoneOf = { $arg0 } · { $arg1 }

# A zone tab with how many cards are in it. `{0}` is the zone's name,
# `{1}` the count.
#
# Bracketed on purpose: the sheet's own typography greys what a sentence
# says in brackets, and a count is exactly that kind of aside — a fact
# the pile itself already shows, riding along beside the name rather
# than being part of it. It is also the one thing the deleted pile chips
# said that nothing else on the sheet did.
# Arguments: arg0, arg1
BrowseTabCount = { $arg0 } ({ $arg1 })

# A card a seat may not see.
LogACard = a card

# A face-down card a seat may not look at (CR 708.5).
LogAFaceDownCard = a face-down card

# A planeswalker attacked that the reading seat's view no longer shows.
LogAPlaneswalker = a planeswalker

# A battle attacked that the reading seat's view no longer shows.
LogABattle = a battle

# Public designation, independent of a battle's controller.
# Arguments: arg0
BattleProtectedBy = Protected by { $arg0 }

# Current defense, including zero while its defeat trigger is pending.
# Arguments: arg0
BattleDefense = Defense { $arg0 }

# The last two names of a list, joined.
# Arguments: arg0, arg1
LogAnd = { $arg0 } and { $arg1 }

# The reading seat inside a list of winners, never first.
LogYouInList = you

# A line that happened more than once in a row. `{0}` is the line, `{1}`
# how many times.
# Arguments: arg0, arg1
LogRepeated = { $arg0 } (×{ $arg1 })

# The heading of the reading seat's own turn. `{1}` is the turn number.
# Arguments: arg0, arg1
LogTurnYou = Turn { $arg1 } · your turn

# The heading of another seat's turn.
# Arguments: arg0, arg1
LogTurn = Turn { $arg1 } · { $arg0 }

# The reading seat took a mulligan.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogMulliganYou = { $arg7 } took a mulligan

# Another seat took a mulligan.
# Arguments: arg0
LogMulligan = { $arg0 } took a mulligan

# The reading seat kept one card. `{1}` is the count.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogKeptCardYou = { $arg7 } kept { $arg1 } card

# The reading seat kept its hand. `{1}` is the count.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogKeptCardsYou = { $arg7 } kept { $arg1 } cards

# Another seat kept one card.
# Arguments: arg0, arg1
LogKeptCard = { $arg0 } kept { $arg1 } card

# Another seat kept its hand.
# Arguments: arg0, arg1
LogKeptCards = { $arg0 } kept { $arg1 } cards

# The decision clock passed priority for the reading seat.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogTimedPassedYou = { $arg7 } ran out of time and passed

# The decision clock passed priority for another seat.
# Arguments: arg0
LogTimedPassed = { $arg0 } ran out of time and passed

# The decision clock kept the reading seat's opening hand.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogTimedKeptYou = { $arg7 } ran out of time and kept your hand

# The decision clock kept another seat's opening hand.
# Arguments: arg0
LogTimedKept = { $arg0 } ran out of time and kept their hand

# The decision clock declared no attackers for the reading seat.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogTimedNoAttackersYou = { $arg7 } ran out of time and did not attack

# The decision clock declared no attackers for another seat.
# Arguments: arg0
LogTimedNoAttackers = { $arg0 } ran out of time and did not attack

# The decision clock declared no blockers for the reading seat.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogTimedNoBlockersYou = { $arg7 } ran out of time and did not block

# The decision clock declared no blockers for another seat.
# Arguments: arg0
LogTimedNoBlockers = { $arg0 } ran out of time and did not block

# The decision clock said no for the reading seat.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogTimedDeclinedYou = { $arg7 } ran out of time and declined

# The decision clock said no for another seat.
# Arguments: arg0
LogTimedDeclined = { $arg0 } ran out of time and declined

# The house chose for the reading seat when its clock ran out.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8
LogTimedChosenYou = { $arg7 } ran out of time and the house chose for { $arg8 }

# The house chose for another seat when its clock ran out.
# Arguments: arg0
LogTimedChosen = { $arg0 } ran out of time and the house chose for them

# The house sat down at the reading seat's chair.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogStandInYou = { $arg7 } went away and the house took over your seat

# The house sat down at another seat's chair.
# Arguments: arg0
LogStandIn = { $arg0 } went away and the house took over their seat

# The reading seat lost its connection; `{1}` is how long the table
# waits (m:ss) before the house takes the chair.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogConnectionLostYou = { $arg7 } lost the connection – the table waited { $arg1 } before the house would take over

# Another seat lost its connection; `{1}` as above.
# Arguments: arg0, arg1
LogConnectionLost = { $arg0 } lost the connection – waiting { $arg1 } for them to reconnect

# The reading seat lost its connection, and nobody else is at the
# table: the game is paused.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogConnectionLostPausedYou = { $arg7 } lost the connection – the game was paused until the connection was back

# Another seat lost its connection with nobody else at the table.
# Arguments: arg0
LogConnectionLostPaused = { $arg0 } lost the connection – the game is paused until they reconnect

# The reading seat is back at its chair.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogReturnedYou = { $arg7 } came back

# Another seat is back at its chair.
# Arguments: arg0
LogReturned = { $arg0 } came back

# The reading seat played a land. `{1}` is the land.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLandPlayedYou = { $arg7 } played { $arg1 }{ $arg2 }

# Another seat played a land.
# Arguments: arg0, arg1, arg2
LogLandPlayed = { $arg0 } played { $arg1 }{ $arg2 }

# The reading seat cast a spell. `{1}` is the spell.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogCastYou = { $arg7 } cast { $arg1 }{ $arg2 }

# Another seat cast a spell.
# Arguments: arg0, arg1, arg2
LogCast = { $arg0 } cast { $arg1 }{ $arg2 }

# An ability the reading seat controls went on the stack. `{1}` is its
# source. Activated or triggered, which the line cannot tell: its
# controller puts either on the stack (CR 602.2a, CR 603.3).
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogAbilityYou = { $arg7 } put an ability of { $arg1 } on the stack

# An ability another seat controls went on the stack.
# Arguments: arg0, arg1
LogAbility = { $arg0 } put an ability of { $arg1 } on the stack

# The reading seat activated an ability (CR 602). `{1}` is its source.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogActivatedYou = { $arg7 } activated { $arg1 }

# Another seat activated an ability.
# Arguments: arg0, arg1
LogActivated = { $arg0 } activated { $arg1 }

# An ability the reading seat controls triggered (CR 603). `{1}` is its
# source.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8
LogTriggeredYou = { $arg1 } triggered for { $arg8 }

# An ability another seat controls triggered.
# Arguments: arg0, arg1
LogTriggered = { $arg1 } triggered for { $arg0 }

# Under a line dealing damage to a player: where it left their life,
# 1. `{2}` is it, `{1}` what it was.
# Arguments: arg0, arg1, arg2
LogLifeNowPoint = now at { $arg2 } life (from { $arg1 })

# The same, at any other total.
# Arguments: arg0, arg1, arg2
LogLifeNowPoints = now at { $arg2 } life (from { $arg1 })

# A spell was countered. `{1}` is the spell.
# Arguments: arg0, arg1
LogCountered = { $arg1 } was countered

# A spell or ability left the stack without resolving. `{1}` is it.
# Arguments: arg0, arg1
LogDidNotResolve = { $arg1 } did not resolve

# The reading seat drew cards it may name. `{1}` is the list.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogDrewYou = { $arg7 } drew { $arg1 }

# Another seat drew cards the reading seat may name.
# Arguments: arg0, arg1
LogDrew = { $arg0 } drew { $arg1 }

# The reading seat drew one card it may not name. `{1}` is the count.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogDrewCardYou = { $arg7 } drew { $arg1 } card

# The reading seat drew cards it may not name.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogDrewCardsYou = { $arg7 } drew { $arg1 } cards

# Another seat drew one card.
# Arguments: arg0, arg1
LogDrewCard = { $arg0 } drew { $arg1 } card

# Another seat drew cards.
# Arguments: arg0, arg1
LogDrewCards = { $arg0 } drew { $arg1 } cards

# The reading seat discarded a card. `{1}` is the card.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogDiscardedYou = { $arg7 } discarded { $arg1 }

# Another seat discarded a card.
# Arguments: arg0, arg1
LogDiscarded = { $arg0 } discarded { $arg1 }

# A card went into a library, and the library was shuffled in the
# same action (#300). `{1}` is the card, `{2}` where from, `{3}` which
# library ([`Phrase::LogIntoLibrary`]).
# Arguments: arg0, arg1, arg2, arg3
LogMovedShuffled = { $arg1 } was shuffled { $arg2 } { $arg3 }

# A card went on top of the reading seat's library.
LogOntoLibraryTopYou = on top of your library

# A card went on top of another seat's library. `{0}` is the seat.
# Arguments: arg0
LogOntoLibraryTop = on top of { $arg0 }'s library

# A card went on the bottom of the reading seat's library.
LogOntoLibraryBottomYou = on the bottom of your library

# A card went on the bottom of another seat's library.
# Arguments: arg0
LogOntoLibraryBottom = on the bottom of { $arg0 }'s library

# A card went into the reading seat's library at a place counted from
# the top. `{1}` is the ordinal: "3rd" in English, "3" in German, whose
# sentence writes the full stop.
# Arguments: arg0, arg1
LogIntoLibraryAtYou = { $arg1 } from the top of your library

# A card went into another seat's library at a place counted from the
# top.
# Arguments: arg0, arg1
LogIntoLibraryAt = { $arg1 } from the top of { $arg0 }'s library

# Where another seat played a land or cast a spell from, when the zone
# is its own (#300). The reading seat's own is [`Phrase::LogFromHandYou`].
LogFromOwnHand = from their hand

# The same, from its own library.
LogFromOwnLibrary = from their library

# The same, from its own graveyard.
LogFromOwnGraveyard = from their graveyard

# "You" as the subject of a log sentence (#300): `{7}` in every one, so
# a panel can set the reading seat in bold like any other seat.
LogYouSubject = you

# "You" as a direct object: `{8}`.
LogYouObject = you

# "You" as an indirect object: `{9}`.
LogYouIndirect = you

# The reading seat lost by drawing from an empty library, as the log
# says it; [`Phrase::LostEmptyDrawYou`] is the end screen's.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLostEmptyDrawYou = { $arg7 } tried to draw from an empty library

# The reading seat lost to poison.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLostPoisonYou = { $arg7 } had ten or more poison counters

# The reading seat lost to a commander's damage.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLostCommanderDamageYou = { $arg7 } took 21 or more combat damage from one commander

# The reading seat conceded.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLostConcededYou = { $arg7 } conceded

# An effect made the reading seat lose.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8
LogLostEffectYou = An effect made { $arg8 } lose

# An object changed zones. `{1}` is the object, `{2}` where it came
# from and `{3}` where it went, each one of the `LogFrom…`/`LogInto…`
# phrases.
# Arguments: arg0, arg1, arg2, arg3
LogMoved = { $arg1 } moved { $arg2 } { $arg3 }

# Out of the reading seat's library.
LogFromLibraryYou = from your library

# Out of another seat's library. `{0}` is the seat.
# Arguments: arg0
LogFromLibrary = from { $arg0 }'s library

# Out of the reading seat's hand.
LogFromHandYou = from your hand

# Out of another seat's hand.
# Arguments: arg0
LogFromHand = from { $arg0 }'s hand

# Out of the reading seat's graveyard.
LogFromGraveyardYou = from your graveyard

# Out of another seat's graveyard.
# Arguments: arg0
LogFromGraveyard = from { $arg0 }'s graveyard

# Off the battlefield.
LogFromBattlefield = from the battlefield

# Out of exile.
LogFromExile = from exile

# Out of the command zone.
LogFromCommand = from the command zone

# Into the reading seat's library.
LogIntoLibraryYou = into your library

# Into another seat's library.
# Arguments: arg0
LogIntoLibrary = into { $arg0 }'s library

# Into the reading seat's hand.
LogIntoHandYou = into your hand

# Into another seat's hand.
# Arguments: arg0
LogIntoHand = into { $arg0 }'s hand

# Into the reading seat's graveyard.
LogIntoGraveyardYou = into your graveyard

# Into another seat's graveyard.
# Arguments: arg0
LogIntoGraveyard = into { $arg0 }'s graveyard

# Onto the battlefield.
LogIntoBattlefield = onto the battlefield

# Into exile.
LogIntoExile = into exile

# Into the command zone.
LogIntoCommand = into the command zone

# The reading seat created a token. `{1}` is the token.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogCreatedYou = { $arg7 } created { $arg1 }

# Another seat created a token.
# Arguments: arg0, arg1
LogCreated = { $arg0 } created { $arg1 }

# Combat damage to the reading seat. `{1}` is the amount, `{2}` the
# source.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9
LogCombatDamageYou = { $arg2 } dealt { $arg1 } combat damage to { $arg9 }

# Damage to the reading seat.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9
LogDamageYou = { $arg2 } dealt { $arg1 } damage to { $arg9 }

# Combat damage to another seat or a permanent, `{0}`.
# Arguments: arg0, arg1, arg2
LogCombatDamage = { $arg2 } dealt { $arg1 } combat damage to { $arg0 }

# Damage to another seat or a permanent, `{0}`.
# Arguments: arg0, arg1, arg2
LogDamage = { $arg2 } dealt { $arg1 } damage to { $arg0 }

# Damage to the reading seat from a source the line cannot name.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogDamageUnsourcedYou = { $arg7 } were dealt { $arg1 } damage

# Damage to another seat or a permanent from a source the line cannot
# name.
# Arguments: arg0, arg1
LogDamageUnsourced = { $arg0 } was dealt { $arg1 } damage

# The reading seat's life total is 1. `{2}` is it, `{1}` what it was.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLifePointYou = { $arg7 } went to { $arg2 } life (from { $arg1 })

# The reading seat's life total changed.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogLifePointsYou = { $arg7 } went to { $arg2 } life (from { $arg1 })

# Another seat's life total is 1.
# Arguments: arg0, arg1, arg2
LogLifePoint = { $arg0 } went to { $arg2 } life (from { $arg1 })

# Another seat's life total changed.
# Arguments: arg0, arg1, arg2
LogLifePoints = { $arg0 } went to { $arg2 } life (from { $arg1 })

# The reading player's public counter total changed.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogPlayerCountersYou = { $arg7 } went to { $arg1 } { $arg2 } (from { $arg3 })

# Another player's public counter total changed.
# Arguments: arg0, arg1, arg2, arg3
LogPlayerCounters = { $arg0 } went to { $arg1 } { $arg2 } (from { $arg3 })

# A permanent that had no counters of a kind got some: object, count,
# counter noun.
# Arguments: arg0, arg1, arg2, arg3
LogCountersGot = { $arg1 } got { $arg2 } { $arg3 }

# Counters on a permanent changed: object, new count, counter noun, old count.
# Arguments: arg0, arg1, arg2, arg3, arg4
LogCounters = { $arg1 } went to { $arg2 } { $arg3 } (from { $arg4 })

# One +X/+Y counter. `{0}` is X and `{1}` is Y.
# Arguments: arg0, arg1
LogCounterPlus = +{ $arg0 }/+{ $arg1 } counter

# +X/+Y counters.
# Arguments: arg0, arg1
LogCountersPlus = +{ $arg0 }/+{ $arg1 } counters

# One -X/-Y counter.
# Arguments: arg0, arg1
LogCounterMinus = -{ $arg0 }/-{ $arg1 } counter

# -X/-Y counters.
# Arguments: arg0, arg1
LogCountersMinus = -{ $arg0 }/-{ $arg1 } counters

# One loyalty counter.
LogCounterLoyalty = loyalty counter

# Loyalty counters.
LogCountersLoyalty = loyalty counters

# A bore counter.
LogCounterBore = bore counter

# Bore counters.
LogCountersBore = bore counters

# A shield counter.
LogCounterShield = shield counter

# Shield counters.
LogCountersShield = shield counters

# Defense counter, singular.
LogCounterDefense = defense counter

# Defense counters, plural.
LogCountersDefense = defense counters

# One lore counter (CR 714.3).
LogCounterLore = lore counter

# Lore counters.
LogCountersLore = lore counters

# One time counter.
LogCounterTime = time counter

# Time counters.
LogCountersTime = time counters

# One age counter (CR 702.24a).
LogCounterAge = age counter

# Age counters (CR 702.24a).
LogCountersAge = age counters

# One charge counter.
LogCounterCharge = charge counter

# Charge counters.
LogCountersCharge = charge counters

# One poison counter.
LogCounterPoison = poison counter

# Poison counters.
LogCountersPoison = poison counters

# One energy counter.
LogCounterEnergy = energy counter

# Energy counters.
LogCountersEnergy = energy counters

# One rad counter.
LogCounterRad = rad counter

# Rad counters.
LogCountersRad = rad counters

# One flying counter.
LogCounterFlying = flying counter

# Flying counters.
LogCountersFlying = flying counters

# One shadow counter.
LogCounterShadow = shadow counter

# Shadow counters.
LogCountersShadow = shadow counters

# One indestructible counter.
LogCounterIndestructible = indestructible counter

# Indestructible counters.
LogCountersIndestructible = indestructible counters

# One hone counter.
LogCounterHone = hone counter

# Hone counters.
LogCountersHone = hone counters

# One lifelink counter.
LogCounterLifelink = lifelink counter

# Lifelink counters.
LogCountersLifelink = lifelink counters

# One level counter.
LogCounterLevel = level counter

# Level counters.
LogCountersLevel = level counters

# One counter of a kind this client has no name for.
LogCounterOther = counter

# Counters of a kind this client has no name for.
LogCountersOther = counters

# A creature attacks the reading seat. `{1}` is the attacker.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8
LogAttackedYou = { $arg1 } attacked { $arg8 }

# A creature attacks another seat or a planeswalker, `{0}`.
# Arguments: arg0, arg1
LogAttacked = { $arg1 } attacked { $arg0 }

# A creature blocks. `{1}` is the blocker, `{2}` the attacker.
# Arguments: arg0, arg1, arg2
LogBlocked = { $arg1 } blocked { $arg2 }

# An attacker joined a band (banding). `{1}` joined the band of `{2}`.
# Arguments: arg0, arg1, arg2
LogBanded = { $arg1 } attacks in a band with { $arg2 }

# The reading seat gained control of a permanent. `{1}` is it.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogControlYou = { $arg7 } gained control of { $arg1 }

# Another seat gained control of a permanent.
# Arguments: arg0, arg1
LogControl = { $arg0 } gained control of { $arg1 }

# A permanent transformed. `{1}` is it.
# A face-down permanent becomes public.
# Arguments: arg0, arg1
LogTurnedFaceUp = { $arg1 } turned face up

# A permanent transforms.
# Arguments: arg0, arg1
LogTransformed = { $arg1 } transformed

# The reading seat chose which permanents to keep. `{1}` is the list.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogCardsKeptYou = { $arg7 } chose to keep { $arg1 }

# Another seat chose which permanents to keep.
# Arguments: arg0, arg1
LogCardsKept = { $arg0 } chose to keep { $arg1 }

# The reading seat revealed cards. `{1}` is the list.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogRevealedYou = { $arg7 } revealed { $arg1 }

# Another seat revealed cards.
# Arguments: arg0, arg1
LogRevealed = { $arg0 } revealed { $arg1 }

# The reading seat shuffled its library.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogShuffledYou = { $arg7 } shuffled your library

# Another seat shuffled its library.
# Arguments: arg0
LogShuffled = { $arg0 } shuffled their library

# The reading seat rolled a die. `{1}` is its sides, `{2}` the result.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogRolledYou = { $arg7 } rolled a d{ $arg1 } and got { $arg2 }

# Another seat rolled a die.
# Arguments: arg0, arg1, arg2
LogRolled = { $arg0 } rolled a d{ $arg1 } and got { $arg2 }

# The reading seat flipped a coin and won the flip.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogFlipWonYou = { $arg7 } flipped a coin and won the flip

# The reading seat flipped a coin and lost the flip.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogFlipLostYou = { $arg7 } flipped a coin and lost the flip

# Another seat won a coin flip.
# Arguments: arg0
LogFlipWon = { $arg0 } flipped a coin and won the flip

# Another seat lost a coin flip.
# Arguments: arg0
LogFlipLost = { $arg0 } flipped a coin and lost the flip

# The reading seat won.
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogWonYou = { $arg7 } won the game

# One other seat won. `{0}` is it.
# Arguments: arg0
LogWonOne = { $arg0 } won the game

# A team won. `{0}` is the list of its seats.
# Arguments: arg0
LogWonMany = { $arg0 } won the game

# Nobody won.
LogDrawn = The game was a draw

# The host found a loop and broke it.
LogLoopBroken = A loop was found and broken

# The host found a loop.
LogLoop = A loop was found

# It became day (CR 730).
LogDay = It became day

# It became night (CR 730).
LogNight = It became night

# The reading seat became the monarch (CR 724.3).
# Arguments: arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7
LogMonarchYou = { $arg7 } became the monarch

# Another seat became the monarch.
# Arguments: arg0
LogMonarch = { $arg0 } became the monarch

# The one mode chosen for a spell or an ability (CR 601.2b, 603.3c).
# `{1}` is it: the card's own words in quotes ([`Phrase::LogQuoted`]),
# or its number.
# Arguments: arg0, arg1
LogAimedMode = mode: { $arg1 }

# Several modes chosen at once. `{1}` is them, joined by " + ".
# Arguments: arg0, arg1
LogAimedModes = modes: { $arg1 }

# The one target of a spell or an ability (CR 601.2c). `{1}` is it.
# Arguments: arg0, arg1
LogAimedTarget = targeting { $arg1 }

# Several targets. `{1}` is the list.
# Arguments: arg0, arg1
LogAimedTargets = targeting { $arg1 }

# A spell's modes or targets, on a line of their own (changed while it
# was on the stack). `{1}` is the spell, `{2}` what it was announced
# with: [`Phrase::LogAimedMode`] and [`Phrase::LogAimedTarget`] parts.
# Arguments: arg0, arg1, arg2
LogAimedSpell = { $arg1 }: { $arg2 }

# The same for an ability. `{1}` is its source.
# Arguments: arg0, arg1, arg2
LogAimedAbility = Ability of { $arg1 }: { $arg2 }

# A card's own words, quoted. `{0}` is them.
# Arguments: arg0
LogQuoted = “{ $arg0 }”

# A turn's phase, as the log groups a turn's lines under it.
LogPhaseBeginning = Beginning

# The precombat main phase.
LogPhaseFirstMain = Main phase

# The combat phase.
LogPhaseCombat = Combat

# The postcombat main phase.
LogPhaseSecondMain = Second main phase

# The ending phase.
LogPhaseEnding = End of turn

# The button, the form's title and the keymap row that opens it.
ReportButton = Report a problem

# A kind of report: something did the wrong thing.
ReportKindBug = Bug

# A kind of report: something could be better.
ReportKindImprovement = Suggestion

# A kind of report: anything said about the game.
ReportKindFeedback = Feedback

# A kind of report the client sends by itself.
ReportKindCrash = Crash

# A kind of report: none of the others.
ReportKindOther = Other

# The empty text box's prompt.
ReportTextHint = What happened, and what did you expect?

# Under the text box. `{0}` characters written of `{1}`.
# Arguments: arg0, arg1
ReportChars = { $arg0 } / { $arg1 } characters

# The text is too long to send. `{0}` is the limit.
# Arguments: arg0
ReportTextTooLong = The text is longer than { $arg0 } characters.

# Above the boxes.
ReportIncludeHeading = Also send (remembered on this device):

# What is always sent, whatever is ticked.
ReportAlways = Always sent: your text, this client's version and the game's id. The gateway adds its own record of the game, which names no one.

# The same, for a report written at no networked table: no game id,
# and no record but one the player ticks below.
ReportAlwaysLocal = Always sent: your text and this client's version.

# A box: system and hardware.
ReportCatSystem = System and hardware

# Under it.
ReportCatSystemHint = Platform, processor count, graphics adapter, window size, language.

# A box: the table.
ReportCatGame = The table as you see it

# Under it.
ReportCatGameHint = Your view of the board, the open question and what you had selected. Nothing that is hidden from you.

# A box: the seat's log.
ReportCatLog = Your game log

# Under it.
ReportCatLogHint = Other players' names are replaced by Player A, Player B, …

# A box: settings.
ReportCatSettings = Settings

# Under it.
ReportCatSettingsHint = Language, display settings, key bindings and standing answers. No names, addresses or tokens.

# A box: the picture.
ReportCatScreenshot = Picture of the Baylee window

# Under it.
ReportCatScreenshotHint = Only Baylee's own window, as it was when you opened this form: never your desktop, other programs or other windows. It can show other players' names.

# A box with nothing behind it.
ReportCatNothing = (nothing to send here)

# Beside the screenshot box in a browser, which takes no picture.
ReportCatNoShotOnWeb = (the browser version takes no picture)

# Under the screenshot box in a browser.
ReportCatScreenshotHintWeb = The browser version of the client cannot take a picture of its window, so a report from here never carries one. Ticked, the box applies when you report from the desktop client.

# The picture's size. `{0}` × `{1}` pixels, `{2}` kilobytes.
# Arguments: arg0, arg1, arg2
ReportShotSize = { $arg0 } × { $arg1 }, { $arg2 } KB

# The box for crash reports, which the client sends by itself.
ReportCrashesBox = Send crash reports automatically

# Opens the preview.
ReportPreviewShow = Show what is sent

# Closes it.
ReportPreviewHide = Hide what is sent

# Sends.
ReportSend = Send

# Closes the form.
ReportClose = Close

# While it goes.
ReportSending = Sending …

# Received. `{0}` is the report's id.
# Arguments: arg0
ReportSent = Thank you. Report { $arg0 } was received.

# 503.
ReportsUnavailable = This gateway does not take reports right now.

# 401.
ReportSignInAgain = Your session has ended. Sign in again, then send the report.

# 413, or too large even without the picture.
ReportTooLarge = The report is too large. Leave out the window picture or the log and try again.

# 429.
ReportTooMany = Too many reports in a short time. Wait a little, then send again.

# No answer at all.
ReportUnreachable = The gateway could not be reached. Your report is still here.

# 502: the gateway took it and its feedback service did not.
ReportNotPassedOn = The gateway could not pass the report on. Your report is still here; try again later.

# Any other answer.
ReportFailed = The report could not be sent. Try again later.

# No session to send it with, and no feedback service to send it to.
ReportNeedsSession = Sign in to a gateway to send a report. This client knows no feedback service to send it to otherwise.

# Signed in nowhere, the report goes straight to the service. `{0}` is
# its address.
# Arguments: arg0
ReportGoesDirect = You are not signed in, so this report goes straight to the Baylee feedback service at { $arg0 }.

# 503 from the service itself.
ReportsDirectUnavailable = The feedback service takes no reports straight from a client right now. Sign in to a gateway to send it.

# No answer from the service.
ReportDirectUnreachable = The feedback service could not be reached. Your report is still here.

# The box for a local game's record, ticked per report.
ReportRecordBox = Attach the whole record of this game

# Under it. `{0}` is its size in kilobytes.
# Arguments: arg0
ReportRecordHint = Every move of the game this device hosted, from the shuffle on ({ $arg0 } KB). It shows every seat's cards, the hidden ones too: hands, libraries, face-down cards. It names nobody. Asked again for each report.

# The standing "never" for records.
ReportRecordNever = Never offer to attach a game's record

# The confirmation's heading.
ReportConfirmTitle = Send this report?

# Where it goes. `{0}` is the address.
# Arguments: arg0
ReportConfirmTo = To: { $arg0 }

# The words. `{0}` is how many characters.
# Arguments: arg0
ReportConfirmText = Your text ({ $arg0 } characters) and this build's version

# A ticked box, before its label.
# Arguments: arg0
ReportConfirmPart = and: { $arg0 }

# The record. `{0}` is its size in kilobytes.
# Arguments: arg0
ReportConfirmRecord = and the whole record of the game this device hosted ({ $arg0 } KB): every seat's cards, the hidden ones too

# The device id, on a direct report.
ReportConfirmDevice = and a random id this device made for reports, so that reports from one device can be told apart; never your name, account or address

# Sends after the confirmation.
ReportConfirmSend = Send now

# Back to the form.
ReportConfirmBack = Back

# The record was too large and stayed home.
ReportRecordLeftOut = The game's record was too large to attach and was left out.

# `seal` found a secret. `{0}` names which, never its value.
# Arguments: arg0
ReportLeaked = Not sent: the report contains your { $arg0 }. Remove it and try again.

# The report was cut to fit.
ReportTrimmed = To fit the size limit, the window picture or older log lines were left out.

# Asked once, at the start after a crash.
CrashAskTitle = Baylee closed unexpectedly last time.

# Under it.
CrashAskBody = May the client send crash reports to the gateway you play on? A crash report holds the error, where in the program it happened (the backtrace) and this client's version, and system details if you allow them in the report form. Nothing about your games or your account. You can change this in the report form at any time.

# The yes.
CrashAskSend = Send crash reports

# The no.
CrashAskNever = Don't send

# The settings screen's panel for language-model seats (`docs/llm-seat.md`).
SeatPanelTitle = Language-model seats

# Under its title.
SeatPanelAbout = Which model the seat bridge (baylee-seat) plays at a table, and what it may spend. No key is ever in this file: each profile names the environment variable its key is read from, or keeps it in this machine's credential store.

# Where the file is. `{0}` its path.
# Arguments: arg0
SeatFileAt = File: { $arg0 }

# A browser or a phone runs no bridge.
SeatDesktopOnly = Language-model seats are set up on a desktop, where the seat bridge runs.

# No directory for the file.
SeatNoConfigDir = This system names no configuration directory, so there is no settings file for the seat bridge.

# No file yet.
SeatEmpty = No settings file yet: the seat bridge plays with its built-in defaults, and nothing caps its spend across games. Add a profile to make one.

# A file with no profiles.
SeatNoProfiles = No profiles: without --mind or --profile, the bridge plays the house AI.

# The file cannot be used. `{0}` why, in the file's own words.
# Arguments: arg0
SeatFileRefused = The settings file cannot be used and is left as it is: { $arg0 }. Mend it by hand; this panel reads it again when it changes.

# The file changed while there were edits here.
SeatChangedOnDisk = The file changed on disk since it was read. Saving writes over it; Discard changes shows it.

# Adds a profile.
SeatAdd = Add a profile

# Adds a copy of the shown profile.
SeatDuplicate = Duplicate

# Removes the shown profile.
SeatRemove = Remove

# The switch that makes the shown profile the default.
SeatDefault = Default

# A profile's chip when it is the default. `{0}` its name.
# Arguments: arg0
SeatChipDefault = { $arg0 } · default

# What the default is.
SeatDefaultAbout = The default profile plays when the bridge is given neither --mind nor --profile.

# No default.
SeatDefaultNone = No default profile: given neither --mind nor --profile, the bridge plays the house AI.

# A profile's name.
SeatName = Name

# A profile's provider.
SeatProvider = Protocol

# The Anthropic API.
SeatProviderAnthropic = Anthropic

# Any OpenAI-compatible endpoint.
SeatProviderOpenAi = OpenAI-compatible

# An agent CLI on this machine, played on the player's subscription.
SeatProviderCli = Agent CLI (subscription)

# Under a CLI profile's model box.
SeatCliModel = A CLI's model is its tool, then the tool's own model if you name one: claude, or claude:opus.

# A profile's model.
SeatModel = Model

# A priced model offered for the model box. `{0}` the model, `{1}` input and `{2}` output price.
# Arguments: arg0, arg1, arg2
SeatPriced = { $arg0 }: { $arg1 } in, { $arg2 } out

# Under the suggestions.
SeatPricedAbout = Prices in US dollars per million tokens, as this build knows them.

# How hard the model thinks.
SeatEffort = Effort

# How the model answers.
SeatAnswer = Answers with

# Optional endpoint-managed game conversations.
SeatSessionApi = Managed game sessions

# Explicit opt-in; ordinary compatible endpoints need no extra API.
SeatSessionApiAbout = Enable only if this server supports the session extension. Keeps a game conversation and closes it when the seat leaves.

# Managed sessions are not part of other provider protocols.
SeatFaultSessionApi = Managed sessions require an OpenAI-compatible server with session support.

# Session lifecycle disabled.
SeatSessionOff = Off

# Session lifecycle enabled.
SeatSessionOn = On

# The build's way of answering.
SeatAnswerBuild = Default

# Answering by calling a tool.
SeatAnswerTools = Tools

# Answering with one JSON object.
SeatAnswerJson = JSON

# Answering with one JSON object held to a schema.
SeatAnswerJsonSchema = JSON schema

# The most one reply may take.
SeatMaxTokens = Max output tokens

# Input price over the build's.
SeatPriceIn = Price in ($ per million)

# Output price over the build's.
SeatPriceOut = Price out ($ per million)

# This build's price for the model. `{0}` input, `{1}` output.
# Arguments: arg0, arg1
SeatBuildPrice = This build's price: { $arg0 } in, { $arg1 } out per million tokens. A price given here replaces it.

# No price for the model.
SeatNoBuildPrice = This build has no price for this model: give one here, or limit its games in tokens.

# The most a game may spend in dollars.
SeatGameUsd = Per game ($)

# The most a game may spend in tokens.
SeatGameTokens = Per game (tokens)

# The longest one answer may take.
SeatThinkSecs = Think seconds

# The variable the key is read from.
SeatKeyEnv = Key variable

# The variable is set. `{0}` its name.
# Arguments: arg0
SeatKeySet = { $arg0 } is set in this program's environment.

# The variable is not set. `{0}` its name.
# Arguments: arg0
SeatKeyUnset = { $arg0 } is not set in this program's environment. The bridge reads it from its own.

# Over the ready-made adapters a profile can be added from.
SeatPresets = Add an adapter (a protocol and its address, both editable afterwards):

# The key box's caption.
SeatKeyBox = Key

# The empty key box, no key kept.
SeatKeyHint = paste the key here

# The empty key box, a key kept.
SeatKeyReplaceHint = paste a new key to replace it

# Hands the typed key to the credential store.
SeatKeyKeep = Keep

# Forgets the kept key.
SeatKeyForget = Forget

# The store is being asked.
SeatKeyAsking = Asking this machine's credential store…

# A key is kept. `{0}` the host it goes to.
# Arguments: arg0
SeatKeyKept = A key is kept in this machine's credential store, for { $arg0 } only. It is never shown; the variable wins where it is set.

# No key is kept. `{0}` the host.
# Arguments: arg0
SeatKeyNoneKept = No key kept for { $arg0 }. One pasted here goes to this machine's credential store, never to the file.

# No store to keep a key in. `{0}` why.
# Arguments: arg0
SeatKeyStoreUnavailable = No credential store to keep a key in ({ $arg0 }): set the key's variable in the environment.

# The key's variable or address is not one yet.
SeatKeyNoEntry = A key can be kept once the key variable and the address read.

# Where the API is.
SeatBaseUrl = Base URL

# A box's value when left empty. `{0}` the value.
# Arguments: arg0
SeatByDefault = { $arg0 } by default

# The base URL when left empty. `{0}` the variable, `{1}` the address.
# Arguments: arg0, arg1
SeatBaseByDefault = { $arg0 }, else { $arg1 }

# An effort left to an OpenAI-compatible endpoint.
SeatEffortEndpoint = the endpoint's own

# An effort left to a CLI.
SeatEffortCli = the CLI's own

# The most calls a game may make.
SeatGameCalls = Per game (calls)

# No call limit for an API's games.
SeatNoCallLimit = no limit

# The program a CLI profile runs.
SeatCommand = Program

# The program when left empty. `{0}` the tool's name.
# Arguments: arg0
SeatCommandByDefault = { $arg0 }, found on PATH

# A CLI's games have no price.
SeatCliNoPrice = A CLI plays on your subscription, which has no price: its games are limited in tokens and calls, and count under the token caps.

# The caps' heading.
SeatCaps = Caps across all games

# Under it.
SeatCapsAbout = Dollars count the games of models with a price, tokens the games of models without one. An empty box is no cap.

# Dollars a day.
SeatDayUsd = Per day ($)

# Dollars a month.
SeatMonthUsd = Per month ($)

# Tokens a day.
SeatDayTokens = Per day (tokens)

# Tokens a month.
SeatMonthTokens = Per month (tokens)

# A profile with no price under a daily dollar cap and no daily token cap.
SeatUnpricedDay = The caps count dollars per day, and this model has no price: the bridge refuses its games until it has a price or the caps a daily token limit.

# The same for a month.
SeatUnpricedMonth = The caps count dollars per month, and this model has no price: the bridge refuses its games until it has a price or the caps a monthly token limit.

# The spend's heading.
SeatSpent = Spent

# Periods on the local clock. `{0}` its offset, "UTC+02:00".
# Arguments: arg0
SeatZoneLocal = Counted in local time, { $arg0 }.

# Periods on UTC.
SeatZoneUtc = Counted in UTC.

# Today's spend. `{0}` the day, `{1}` what it counts.
# Arguments: arg0, arg1
SeatSpentDay = Today, { $arg0 }: { $arg1 }

# This month's spend. `{0}` the month, `{1}` what it counts.
# Arguments: arg0, arg1
SeatSpentMonth = This month, { $arg0 }: { $arg1 }

# Spent against a cap. `{0}` spent, `{1}` the cap.
# Arguments: arg0, arg1
SeatOf = { $arg0 } of { $arg1 }

# One token.
# Arguments: arg0
SeatTokensOne = { $arg0 } token

# Tokens.
# Arguments: arg0
SeatTokensMany = { $arg0 } tokens

# How many games a sum is over. `{0}` games, `{1}` of them still open.
# Arguments: arg0, arg1
SeatGames = games: { $arg0 }, still open: { $arg1 }

# The book cannot be read. `{0}` why.
# Arguments: arg0
SeatBookUnreadable = The spend book cannot be read: { $arg0 }

# Writes the file.
SeatSave = Save

# Drops every edit.
SeatRevert = Discard changes

# There are edits.
SeatUnsaved = Unsaved changes.

# Save waits for the faults.
SeatFixFirst = Mend what is marked, then save.

# Written.
SeatSavedNote = Saved.

# Not written. `{0}` why.
# Arguments: arg0
SeatNotSaved = Not saved: { $arg0 }

# A box for a whole number holds something else.
SeatFaultWhole = A whole number, in digits.

# A box for dollars holds something else.
SeatFaultDollars = An amount of US dollars, not below zero, such as 2.5.

# Half a price.
SeatFaultHalfPrice = Give both prices, input and output, or neither.

# Two profiles of one name.
SeatFaultNameTaken = Another profile has this name.

# A field named like a key.
SeatFaultKeyNamed = Named like a key. A key never goes in the settings file: set it in the environment and name its variable under Key variable.

# A value shaped like a key.
SeatFaultKeyShaped = This looks like an API key. A key never goes in the settings file: set it in the environment and name its variable under Key variable.

# The default names no profile.
SeatFaultNoDefault = The default names no profile.

# Not a profile's name.
SeatFaultName = A name is up to 32 letters, digits, - and _, starting with a letter or digit.

# Not a model id.
SeatFaultModel = A model id is up to 100 letters, digits and - _ . : / @.

# A CLI's model that names no tool this build plays.
SeatFaultTool = Name the tool first: claude, or claude:<model>.

# A CLI asked to answer with tools.
SeatFaultCliAnswer = A CLI has none of our tools: it answers with JSON or a JSON schema.

# A key or an address on a CLI profile.
SeatFaultCliKey = A CLI reads no key and has no address: it plays as you are signed in to it.

# A price on a CLI profile.
SeatFaultCliPrice = A subscription has no price: limit a CLI's games in tokens and calls.

# A program on a profile that is not a CLI's.
SeatFaultCliOnly = Only a CLI profile runs a program.

# A program named by a path that is not absolute.
SeatFaultCommand = The program's absolute path, such as /opt/homebrew/bin/claude.

# No calls at all.
SeatFaultGameCalls = The most calls a game may make: at least 1.

# Not an effort.
SeatFaultEffort = An effort is one lowercase word, such as low, medium or high.

# JSON asked of Anthropic.
SeatFaultJson = JSON answers are for OpenAI-compatible endpoints and CLIs; Anthropic's models answer with tools.

# No reply at all.
SeatFaultMaxTokens = The most one reply may take: at least 1.

# No time at all.
SeatFaultThinkSecs = The longest one answer may take: at least 1 second.

# A dollar budget with no price.
SeatFaultUnpriced = This build has no price for this model, so a dollar limit cannot be held: give it a price, or make its tokens per game the limit.

# Not a variable's name. `{0}` an example.
# Arguments: arg0
SeatFaultKeyEnv = The name of an environment variable, such as { $arg0 }: capitals, digits and _. Never the key itself.

# An address a key may not go to.
SeatFaultAddress = An https:// address; http:// only on this machine.

# Settings that cannot be written at all.
SeatFaultFile = These settings cannot be written.

# Navigation: the play/lobby screen.
ShellPlay = Lobby

# Navigation: the decks screen.
ShellDecks = Decks

# A settings row kept on this device (`ClientSettings`).
ShellThisDevice = This device

# A settings row kept with the account, on every device (`Preferences`).
ShellYourAccount = Your account

# An error line's action.
ShellRetry = Retry

# A toast's action after a deletion.
ShellUndo = Undo

# The tour bubble's chapter line: chapter number, how many, its name.
# Arguments: arg0, arg1, arg2
TourChapterLine = Chapter { $arg0 } of { $arg1 } · { $arg2 }

# The tour bubble's primary.
TourNext = Next

# The tour bubble's primary on a tour's last step.
TourDone = Done

# The tour bubble's way back one step.
TourBack = Back

# The tour bubble's way past the chapter (the button says less than it
# does: "Kapitel überspringen" is over the button budget).
TourSkip = Skip

# The reason line under Skip: what it skips.
TourSkipReason = Skips this chapter

# The reason a try-it step's primary is off until the player did it.
TourTryFirst = Try it first, or skip

# The try-it step's line saying the keys are the screen's now.
TourKeysFree = Your keys go to the screen now

# The try-it step's line at the table.
TourKeysTable = Your keys go to the table now

# The bubble's checkbox: every tour and tip off (one box, as World of Warcraft's).
TourNeverAgain = Don't show tours again

# The bubble's fold control (its tooltip).
TourFold = Fold away

# The folded pill's line while a question waits for this seat.
TourTableWaits = The table is waiting for you

# The toast after the bubble's checkbox, with Undo.
TourOffToast = Tours off · Settings › Display

# The toast after Settings › Restart tours, with Undo.
TourRestartedToast = Tours start over

# L14's primary: the practice game.
TourStartPractice = Start practice game

# L14's secondary.
TourLater = Later

# The game menu's row that opens the table tour's chapters.
TourMenuRow = Tour

# The game menu's row at a networked table, where it is off.
TourMenuRowAway = Tour (practice game only)

# Settings › Display: the lobby & room tour.
RowTourLobby = Lobby & room

# Its help line.
HelpTourLobby = Shows you the front door, the Play screen and a room, each on your first visit.

# Settings › Display: the deck builder tour.
RowTourBuilder = Deck builder

# Its help line.
HelpTourBuilder = Shows you your decks, the deck builder, import, export and history.

# Settings › Display: the table tour.
RowTourTable = Table

# Its help line.
HelpTourTable = Shows you the table, in a practice game against the house.

# Settings › Display: the just-in-time tips.
RowTourTips = In-game tips

# Its help line.
HelpTourTips = A short tip the first time you see something, once on this device.

# Settings › Display: every tour from the start again.
RowTourRestart = Restart tours

# Its help line.
HelpTourRestart = Forgets what you have seen here and shows every tour again.

# The lobby tour's chapters.
TourChapterDoor = The door

# The lobby tour's chapters.
TourChapterPlay = Play

# The lobby tour's chapters.
TourChapterCreate = Create a table

# The lobby tour's chapters.
TourChapterRoom = The room

# The lobby tour's chapters.
TourChapterClosing = Wrapping up

# The builder tour's chapters.
TourChapterShelf = Your decks

# The builder tour's chapters.
TourChapterEditing = Editing

# The builder tour's chapters.
TourChapterLists = Import and export

# The builder tour's chapters.
TourChapterHistory = History

# The table tour's chapters (TOURS.md §2.3).
TourChapterArrival = Getting started

# The table tour's chapters.
TourChapterPlace = Your place

# The table tour's chapters.
TourChapterCasting = Playing cards

# The table tour's chapters.
TourChapterStack = The stack

# The table tour's chapters.
TourChapterDecisions = Questions

# The table tour's chapters.
TourChapterPhases = Phases and passing

# The table tour's chapters.
TourChapterDial = The middle of the table

# The table tour's chapters.
TourChapterSeeing = The camera

# The table tour's chapters.
TourChapterReveals = Reveals, log, zones

# The table tour's chapters.
TourChapterKeys = Keys and menu

# The table tour's chapters.
TourChapterReporting = Reporting a problem

# Tour step L1's title (TOURS.md §2).
TourL1Title = The gateway

# Tour step L1's words (TOURS.md §2).
TourL1Body = A gateway is the server that keeps your decks and tables. The coloured dot shows whether it can be reached, the line under it whether your version matches it. ‹ takes you to the list of gateways, where you can also type in an address.

# Tour step L2's title (TOURS.md §2).
TourL2Title = Account or guest

# Tour step L2's words (TOURS.md §2).
TourL2Body = As a guest you only need a name. In return you can't upload pictures, and about 30 days after your last visit the guest and its decks are deleted. With an account your decks and settings stay on the gateway. In a closed beta, the button you pick asks for your key.

# Tour step L3's title (TOURS.md §2).
TourL3Title = Play offline

# Tour step L3's words (TOURS.md §2).
TourL3Body = Play offline needs no gateway: you play right here on your computer against the house AI. Next to it are Settings and About, where you find the licences and the source code.

# Tour step L4's title (TOURS.md §2).
TourL4Title = Three places

# Tour step L4's words (TOURS.md §2).
TourL4Body = At the top you switch between Play, Decks and Settings, or press 1, 2 or 3. On the right you see your gateway with how many tables and players are on it, the bell with what you missed, and your account. Press ? on any screen to see every key that works right now.

# Tour step L4's title on a desktop, which has a fourth place: Replays.
TourL4TitleDesktop = Four places

# Tour step L4's words on a desktop (Replays is `4`, Settings stays `3`).
TourL4BodyDesktop = At the top you switch between Play, Decks, Replays and Settings, or press 1, 2, 4 or 3. On the right you see your gateway with how many tables and players are on it, the bell with what you missed, and your account. Press ? on any screen to see every key that works right now.

# Tour step L5's title (TOURS.md §2).
TourL5Title = Your next game

# Tour step L5's words (TOURS.md §2).
TourL5Body = This is the deck you take to your next table. Change deck opens the picker, Edit opens the deck builder. If you have no deck of your own yet, three house decks stand here: Add and use copies one into your account and picks it right away.

# Tour step L6's title (TOURS.md §2).
TourL6Title = Play the house

# Tour step L6's words (TOURS.md §2).
TourL6Body = Play the house starts a game against the house AI, right here in the client and without a clock. The small arrow on the button lets you pick how strong it plays. The table tour runs in a game like this too.

# Tour step L7's title (TOURS.md §2).
TourL7Title = The tables

# Tour step L7's words (TOURS.md §2).
TourL7Body = Here you see live every table on the gateway: who hosts it, the format, how many seats are free and whether it takes a password. Join sits you down; a password goes right into the row. / searches, the chips above filter. If you already sit somewhere, Return to game stands here.

# Tour step L8's title (TOURS.md §2).
TourL8Title = Create table

# Tour step L8's words (TOURS.md §2).
TourL8Body = Create table opens a table of your own; the key C does the same. Open it now.

# Tour step L9's title (TOURS.md §2).
TourL9Title = Setting up a table

# Tour step L9's words (TOURS.md §2).
TourL9Body = Pick how many players (2 to 8) and a template: Commander, Duel 20 or Five-land start. Adjust changes life, mulligan and hand size. A password is optional. The clock is the thinking time per decision, from blitz to untimed. Open table takes you into the room.

# Tour step L10's title (TOURS.md §2).
TourL10Title = The seats

# Tour step L10's words (TOURS.md §2).
TourL10Body = Every seat is a row. In yours you pick your deck and say Ready. On an empty seat, AI seats the house AI at a strength you choose, and Language model a model on your own key. The other players' rows are read-only.

# Tour step L11's title (TOURS.md §2).
TourL11Title = Seat menu and teams

# Tour step L11's words (TOURS.md §2).
TourL11Body = As the host, the three dots on a seat hold more: Make host, Next team, Life override, a Starting position for testing, and Make it an open seat. Set teams in the rules on the left turns teams on; each seat then shows its team.

# Tour step L12's title (TOURS.md §2).
TourL12Title = Rules and invite

# Tour step L12's words (TOURS.md §2).
TourL12Body = The table's rules stand on the left; Edit rules opens the dialog again. Copy invite puts the table's name, the gateway and a hint about the password on your clipboard. You leave the room only with Leave, never with Esc. If you go elsewhere, your seat stays; the strip at the top or R brings you back.

# Tour step L13's title (TOURS.md §2).
TourL13Title = Ready and Start

# Tour step L13's words (TOURS.md §2).
TourL13Body = Everyone clicks Ready for themselves. Start belongs to the host (also Ctrl/Cmd+Enter) and stays grey until everyone is ready; the reason stands next to it. The table opens once everyone has loaded it; only then do the clocks run.

# The room chapter's last bubble: leave the table the tour had the
# player open, a real one on the gateway (09.10.).
TourLeaveTable = Leave table

# Tour step L14's title (TOURS.md §2).
TourL14Title = Practice game?

# Tour step L14's words (TOURS.md §2).
TourL14Body = That was the lobby. The best place to show you the table is a practice game: you against three house AIs, no clock, with your deck. Start it now? Later you find it under Play › Play the house and then in the menu under Tour.

# Tour step D1's title (TOURS.md §2).
TourD1Title = Your decks

# Tour step D1's words (TOURS.md §2).
TourD1Body = Your decks and the house decks live here. A tile's picture is your commander, or else the most expensive card that isn't a land. Under it: format, card count, sideboard and when you last saved. Badges mark your next deck and cards that aren't playable yet.

# Tour step D2's title (TOURS.md §2).
TourD2Title = What a tile can do

# Tour step D2's words (TOURS.md §2).
TourD2Body = Use for next game makes this your next deck. Edit opens the deck builder. Behind the three dots: Duplicate, Rename, Export, History, Favourite and Delete. Delete doesn't ask; you get six seconds of Undo, also with Ctrl/Cmd+Z.

# Tour step D3's title (TOURS.md §2).
TourD3Title = House decks

# Tour step D3's words (TOURS.md §2).
TourD3Body = Ready-made decks from the gateway; every card in them is playable. Add copies one to your decks, Add and use also picks it for your next game. Preview shows the list; hover a card or press Space to see it large.

# Tour step D4's title (TOURS.md §2).
TourD4Title = New or imported

# Tour step D4's words (TOURS.md §2).
TourD4Body = New deck opens an empty deck builder. Import reads Baylee text, JSON, YAML or Moxfield text and spots the format itself; Ctrl/Cmd+V or a dropped file works too. A Moxfield link doesn't load; the dialog says what to copy. Open a deck to continue in the builder.

# Tour step D5's title (TOURS.md §2).
TourD5Title = The header

# Tour step D5's words (TOURS.md §2).
TourD5Body = ‹ Decks takes you back and asks first if something isn't saved. The pencil renames the deck. Save (Ctrl/Cmd+S) shows a dot while something is unsaved. Esc never throws you out of the deck builder; it only closes whatever is open on top.

# Tour step D6's title (TOURS.md §2).
TourD6Title = The card pool

# Tour step D6's words (TOURS.md §2).
TourD6Body = On the left are all the cards Baylee can play already, not the whole catalogue. Playable only is on and hides placeholders. Cards that only partly work stay in and are marked; the card view shows why. The line under the count tells you what else is narrowing the list.

# Tour step D7's title (TOURS.md §2).
TourD7Title = Searching

# Tour step D7's words (TOURS.md §2).
TourD7Body = The search field understands Scryfall syntax, such as t:creature, o:draw, c:rg or mv<=3, with or, brackets and minus too. A German card name finds the card just as well. The gear builds the filter for you as a form. Type something now.

# Tour step D8's title (TOURS.md §2).
TourD8Title = Adding

# Tour step D8's words (TOURS.md §2).
TourD8Body = + puts a card into the tab that is open, with Shift into the other one; Enter and Shift+Enter do the same. Behind the three dots: Sideboard, Choose printing and Open card. Hover a row and you see the card large. Add a card now.

# Tour step D9's title (TOURS.md §2).
TourD9Title = The deck side

# Tour step D9's words (TOURS.md §2).
TourD9Body = Your deck is on the right. The commander slot at the top, then the tabs Main, Sideboard and Stats, grouped by type, mana value or colour. In each row − and + count down and up; the three dots move the card, pick its printing or remove all copies. Stats shows the curve, the colours and Draw seven.

# Tour step D10's title (TOURS.md §2).
TourD10Title = Notes at the bottom

# Tour step D10's words (TOURS.md §2).
TourD10Body = Two kinds of notes stand at the bottom. What the gateway refuses greys out Save: no name, an empty deck, more than 250 lines, or cards that are gone. Everything else is just a sentence, like 60 cards or 15 in the sideboard. If the button is lit, you can save.

# Tour step D11's title (TOURS.md §2).
TourD11Title = Saving

# Tour step D11's words (TOURS.md §2).
TourD11Body = Every save makes a version you can find again later in History. If nothing changed, nothing is written. Press Ctrl/Cmd+S now.

# Tour step D11b's title (TOURS.md §2).
TourD11bTitle = Phone: one pane

# Tour step D11b's words (TOURS.md §2).
TourD11bBody = On a phone you see the pool or the deck; the switch at the top flips between them. Filters gathers the chips behind one button. Enter in the search adds the first hit.

# Tour step D12's title (TOURS.md §2).
TourD12Title = Import

# Tour step D12's words (TOURS.md §2).
TourD12Body = Paste your list with Ctrl/Cmd+V; Baylee recognises the format, and if it isn't sure you pick it with the arrow keys. Enter imports, afterwards Enter closes. Lines it couldn't find stay listed and are named; nothing is dropped quietly.

# Tour step D13's title (TOURS.md §2).
TourD13Title = Export

# Tour step D13's words (TOURS.md §2).
TourD13Body = The arrow keys pick the format. Ctrl/Cmd+C copies the list, Ctrl/Cmd+S saves it as a file (not in the browser). Baylee text remembers each card's printing and finish, Moxfield text only count and name.

# Tour step D14's title (TOURS.md §2).
TourD14Title = History

# Tour step D14's words (TOURS.md §2).
TourD14Body = All saved versions stand on the left; on the right you see what changed against the previous or the current one. Restore version turns the old version into a new one; nothing is lost, and for six seconds you can undo it.

# Tour step T1's title (TOURS.md §2).
TourT1Title = The practice game

# Tour step T1's words (TOURS.md §2).
TourT1Body = This is a practice game: three house AIs, no clock, nothing counts. Every bubble has Next, Back and Skip; Esc folds it to a small strip at the top, and the chapter line lets you jump. When the table waits for you, the bubble folds by itself. ? shows you every key at any time.

# Tour step T2's title (TOURS.md §2).
TourT2Title = Your mat

# Tour step T2's words (TOURS.md §2).
TourT2Body = Your mat has three rows: creatures at the front, towards the middle, the rest behind them, lands at the back. At the edge lie your library, graveyard, exile and command zone, each with a count. Identical cards stack as ×N; a click spreads them out. If a row gets too full, it scrolls sideways.

# Tour step T3's title (TOURS.md §2).
TourT3Title = The hand

# Tour step T3's words (TOURS.md §2).
TourT3Body = Your hand lies at the bottom. You can sort it by cost, A–Z or type, and scroll it with the mouse wheel.

# Tour step T4's title (TOURS.md §2).
TourT4Title = Reading a card

# Tour step T4's words (TOURS.md §2).
TourT4Body = Hover a card and it lifts and grows. Hold Cmd or Alt while you do and you see its text as a text face; T keeps that on, V opens the preview from the keyboard. You always see the text the card has in the game right now; a copy shows the copied card's text.

# Tour step T5's title (TOURS.md §2).
TourT5Title = The bar at the bottom

# Tour step T5's words (TOURS.md §2).
TourT5Body = The bar at the bottom tells you what the table wants from you and shows the answers with their keys: Space passes, Enter clicks, Tab goes to the next phase, Shift+Tab to the next turn; every decision is still yours. On the right are the log, the zones and the menu.

# Tour step T6's title (TOURS.md §2).
TourT6Title = Lit cards

# Tour step T6's words (TOURS.md §2).
TourT6Body = Cards you can play right now light up: gold means your mana already covers it, blue means the client would tap lands for it. The first click picks the card, the second (or Enter, E) plays it; Esc unpicks it. Mana abilities take one click. The client never pays life by itself and never guesses an X.

# Tour step T7's title (TOURS.md §2).
TourT7Title = Play first, pay after

# Tour step T7's words (TOURS.md §2).
TourT7Body = Unlike Arena, a click plays the card first and pays after. The card goes on the stack, then its questions come (mode, X, targets), then the paying. The client taps the lands it planned, but you can tap any land yourself. What's still missing stands as Owed in the mana pool.

# Tour step T8's title (TOURS.md §2).
TourT8Title = Cancel cast

# Tour step T8's words (TOURS.md §2).
TourT8Body = While the questions or the paying are still open, the last answer in the bar is Cancel cast (Esc): the card comes back to your hand and tapped lands untap. Play a card now and cancel it; with nothing castable yet, go on with Next.

# Tour step T9's title (TOURS.md §2).
TourT9Title = Several abilities

# Tour step T9's words (TOURS.md §2).
TourT9Body = If a card has several abilities or several ways to play it, a sheet opens next to it: the cost, the printed text and a key from 1 to 9; 0 turns the page. A mana row goes at once; any other you pick first and then confirm. Alternative costs are asked here, before any land is tapped.

# Tour step T10's title (TOURS.md §2).
TourT10Title = The stack

# Tour step T10's words (TOURS.md §2).
TourT10Body = Top right you see the stack, each entry as a card: the next one large with its targets, the rest smaller below. The line at the top says who the table is waiting for. A long text scrolls inside its box. The button folds the whole thing away.

# Tour step T11's title (TOURS.md §2).
TourT11Title = Answers in advance

# Tour step T11's words (TOURS.md §2).
TourT11Body = Click an entry to set a stop: Resolve up to selection passes until it is on top, then halts. For this card's ability pick Always pass, Always ask, Always yes or Always no; that holds for every copy, stays on your account, and resets under Settings › Gameplay. Targets and costs are always asked.

# Tour step T12's title (TOURS.md §2).
TourT12Title = F6 and F7

# Tour step T12's words (TOURS.md §2).
TourT12Body = F6 lets the stack run through, F7 stops the questions for this turn. Both switch themselves off as soon as something new happens, and stand visibly in the bar until then; the same key again ends them. Only what you turn on under Settings › Gameplay stays automatic; none of it is on from the start.

# Tour step T13's title (TOURS.md §2).
TourT13Title = Questions with choices

# Tour step T13's words (TOURS.md §2).
TourT13Body = When a question has several answers, a window unfolds from the bar: at the top the question and the picture of the card it is about (hover to see it large), at the bottom the answers with their keys. Whatever you may choose lights up on the table. The window never closes by itself.

# Tour step T14's title (TOURS.md §2).
TourT14Title = Folding the window

# Tour step T14's words (TOURS.md §2).
TourT14Body = The window covers the table. X or the button at its top folds it to a small strip at the right edge; then you see the whole table and can click any target. The strip or X brings it back, and the question stays. Fold it now and open it again.

# Tour step T15's title (TOURS.md §2).
TourT15Title = Other questions

# Tour step T15's words (TOURS.md §2).
TourT15Body = You pick a colour as a symbol, not a word. You type a number or step it with the arrow keys; Backspace clears. For a creature type, your deck's types come first, then all of them by letter; typing filters. A card name comes from the same field, over every card.

# Tour step T16's title (TOURS.md §2).
TourT16Title = The phases

# Tour step T16's words (TOURS.md §2).
TourT16Body = On each mat's edge stand the twelve steps of a turn; the current one is lit. Green: the table asks you here. Red: it passes for you here. Nothing is red at the start. A click toggles a step, Shift+W and Shift+S walk them. Presets: Settings › Gameplay. Your own attack and block steps never go red.

# Tour step T17's title (TOURS.md §2).
TourT17Title = Passing

# Tour step T17's words (TOURS.md §2).
TourT17Body = Space passes. Tab runs to the next phase, Shift+Tab to the next turn; you still get every question in between. With a possible attacker, Space does not mean "no attack"; that is O. At a gateway your clock stands in the button: red and audible from 60 seconds; when it runs out, it presses for you.

# Tour step T18's title (TOURS.md §2).
TourT18Title = Attacking and blocking

# Tour step T18's words (TOURS.md §2).
TourT18Body = A click on a creature sends it at the opponent in focus; another click takes it back. If several opponents are possible, C and Shift+C switch between them. Attack, Block or Declare nothing sends your answer. For your combat questions the camera comes home by itself.

# Tour step T19's title (TOURS.md §2).
TourT19Title = The dial in the middle

# Tour step T19's words (TOURS.md §2).
TourT19Body = The turn number stands in the middle of the table, with a jewel per player around the rim in their colour. The long pale hand shows whose turn it is, the short blue one who the table waits for. When the blue hand reaches you, the middle pulses and you hear a sound (menu › Priority sound).

# Tour step T20's title (TOURS.md §2).
TourT20Title = Name plates

# Tour step T20's words (TOURS.md §2).
TourT20Body = Every mat carries a plate: name, life, cards in hand, library, poison, commander damage; a crown marks the monarch, a badge the house AI. The line along its top glows pale when that player is on turn and blue when the table waits for them. Bottom left you see all players once more as small chips.

# Tour step T21's title (TOURS.md §2).
TourT21Title = The arrangement

# Tour step T21's words (TOURS.md §2).
TourT21Body = Top left you choose how the table is laid out: Ring, Upright ring, Turntable, Arc rail, Pods, Spotlight, Turntable+ or Focus ring. P opens the list, Shift+P takes the next one. The tick remembers your choice for this number of players. A duel has only one. Press Shift+P now, then once more.

# Tour step T22's title (TOURS.md §2).
TourT22Title = Visiting a player

# Tour step T22's words (TOURS.md §2).
TourT22Body = Click a chip at the bottom left or press F, and the camera swings behind that player; their board stands upright and large in front of you, while nothing on the table moves. Shift+F goes to the previous one; H, Esc or your own chip brings you home. Targets work from there too. Press F now, then H.

# Tour step T23's title (TOURS.md §2).
TourT23Title = Following the turn

# Tour step T23's words (TOURS.md §2).
TourT23Body = By itself the camera follows only your turn and your combat questions. If you want it to swing to whoever is on turn, switch on Table follows the turn under Settings › Graphics. It never swings while a question is open for you or your pointer rests on the table.

# Tour step T24's title (TOURS.md §2).
TourT24Title = On a phone

# Tour step T24's words (TOURS.md §2).
TourT24Body = On a phone you see your mat and the middle of the table at home. You visit the other players through the chips; the arrangement is in the menu.

# Tour step T25's title (TOURS.md §2).
TourT25Title = Reveals

# Tour step T25's words (TOURS.md §2).
TourT25Body = When another player reveals cards, you see them large in a window: who reveals, the cards, Close. Hover one and it grows further. You can fold it to a strip at the top; Esc or a click closes it. It stays for a few seconds and doesn't vanish just because you pass.

# Tour step T26's title (TOURS.md §2).
TourT26Title = The game log

# Tour step T26's words (TOURS.md §2).
TourT26Body = L or the scroll in the bar opens the log on the right: the time, players in bold, and every card as a link that opens its preview, except cards you weren't allowed to see. It jumps to the newest line until you scroll up. At the end of the game, all of it stands on the result window. Press L now.

# Tour step T27's title (TOURS.md §2).
TourT27Title = The zones

# Tour step T27's words (TOURS.md §2).
TourT27Body = G, the zones door in the bar or a click on a pile at a mat's edge opens the zones: graveyards, exile and the stack. The search field understands the deck builder's syntax. When the table asks you about a pile, it opens in the middle: Space ticks, Enter sends.

# Tour step T28's title (TOURS.md §2).
TourT28Title = Every key

# Tour step T28's words (TOURS.md §2).
TourT28Body = ? shows you every key that works at the table right now. F8 reports a problem, Ctrl/Cmd with +, − and 0 changes the text size. You can rebind any key under Settings › Controls; that stays with your account. With W, A, S, D and E you play without a mouse at all.

# Tour step T29's title (TOURS.md §2).
TourT29Title = The menu

# Tour step T29's words (TOURS.md §2).
TourT29Body = The menu at the bottom right holds everything you need during a game: Offer a draw, Concede, Arrangement, Priority sound, Music, Report a problem and Tour. There are no other settings at the table; those wait in the lobby.

# Tour step T30's title (TOURS.md §2).
TourT30Title = Conceding and the end

# Tour step T30's words (TOURS.md §2).
TourT30Body = Concede asks once more: Enter means Back; to concede you press → and then Enter, or click. At the end you see a window with the result, each player's reason and the whole log. At a gateway there is Play again, offline Back to the lobby; Esc always takes you to the lobby.

# Tour step T31's title (TOURS.md §2).
TourT31Title = Reporting a problem

# Tour step T31's words (TOURS.md §2).
TourT31Body = The corner at the top right, F8 or Report a problem in the menu opens a form over the table. Pick the kind, Bug, Suggestion, Feedback or Other, and write what's going on. Your text and the client's version are always sent; everything else only if you tick it. Open it now.

# Tour step T32's title (TOURS.md §2).
TourT32Title = Attachments

# Tour step T32's words (TOURS.md §2).
TourT32Body = Every row under Attachments is a choice your device remembers: system, the table as you see it, your log (names replaced), settings (no keys), a picture of the Baylee window only, never your desktop. All start ticked; untick what you'd rather keep before you send. Show what is sent shows it all.

# Tour step T33's title (TOURS.md §2).
TourT33Title = Where it goes

# Tour step T33's words (TOURS.md §2).
TourT33Body = Your report goes to the gateway you are signed in to. Signed in nowhere, it goes to this build's feedback service; if there is none, you can only Copy. With a record, a window asks you once more. At a gateway your clock keeps running meanwhile, so better report after the question.

# Tour step T34's title (TOURS.md §2).
TourT34Title = Done

# Tour step T34's words (TOURS.md §2).
TourT34Body = That was the table. You can see any chapter again in any game against the house: menu › Tour. Tours off or on again: Settings › Display & Interface › Tours. Have fun!

# A toast's action that opens more.
ShellDetails = Details

# A deck tile's primary.
ShellUseForNextGame = Use for next game

# A deck tile's second action.
ShellEdit = Edit

# A house deck: copy it and make it the next game's deck.
ShellAddAndUse = Add and use

# The gold primary while the player holds a seat.
ShellReturnToGame = Return to game

# Why a control is off while there is no gateway.
ShellNeedsGateway = Needs the gateway

# Why the starts are off while the player holds a seat. `{0}` the table.
# Arguments: arg0
ShellSeatedAt = You are seated at { $arg0 }

# An error line. `{0}` the gateway's name.
# Arguments: arg0
ShellCouldNotReach = Couldn't reach { $arg0 }

# Play's empty tables list.
ShellNoOneWaiting = No one is waiting

# Gateway context and table-discovery actions.
DiscoveryGateway = YOUR GATEWAY

# Localized table-discovery interface text.
DiscoveryOnline = Online

# Localized table-discovery interface text.
DiscoveryWaiting = Open tables

# Localized table-discovery interface text.
DiscoveryPlaying = In play

# Localized table-discovery interface text.
DiscoveryLive = Live updates connected

# Localized table-discovery interface text.
DiscoveryConnecting = Connecting live updates…

# Localized table-discovery interface text.
DiscoveryRefresh = Refresh tables

# Localized table-discovery interface text.
DiscoveryDetails = Gateway details

# Localized table-discovery interface text.
DiscoveryChoose = Find your next table

# Localized table-discovery interface text.
DiscoveryHelp = Join a table, then choose your deck in the room. The host sets the format and pace.

# Localized table-discovery interface text.
DiscoveryLoading = Updating tables

# Localized table-discovery interface text.
# Arguments: arg0
DiscoveryResults = { $arg0 } tables shown

# Localized table-discovery interface text.
DiscoveryProtected = Password protected

# Localized table-discovery interface text.
DiscoveryOpen = Open to join

# A list read a moment ago.
ShellUpdatedJustNow = updated just now

# A table filter.
ShellOpenSeats = Open seats

# A table filter.
ShellNoPassword = No password

# The decks screen's first tab.
ShellMyDecks = My decks

# The decks screen's second tab.
ShellHouseDecks = House decks

# A deck was deleted; the toast offers Undo.
ShellDeckDeleted = Deck deleted

# The account popover.
ShellCopyHandle = Copy my handle

# The text-size row (Display & Interface).
ShellTextSize = Text size

# The text-size row's help.
ShellTextSizeHelp = Ctrl/Cmd + and − change it anywhere outside a game.

# The master volume row.
ShellMasterVolume = Master volume

# An account row.
ShellHoldStill = Hold the table still

# A drawer of expert fields.
ShellAdvanced = Advanced

# The dev gallery's title.
ShellGallery = Shell components

# A shell key: focus the screen's search.
ShellKeySearch = Search

# A shell key: the shortcuts overlay.
ShellKeyOverlay = Keyboard shortcuts

# A shell key: the overlay in go-to mode.
ShellKeyQuickSwitch = Go to…

# A shell key: read this screen's data again.
ShellKeyRefresh = Refresh

# A shell key: text one step larger.
ShellKeyTextLarger = Larger text

# A shell key: text one step smaller.
ShellKeyTextSmaller = Smaller text

# A shell key: text back to the default size.
ShellKeyTextReset = Default text size

# A shell key: start the game (the room's host).
ShellKeyStart = Start the game

# A shell key: save the deck.
ShellKeySave = Save deck

# A shell key: export the deck.
ShellKeyExport = Export

# The shortcuts overlay's search field, while empty.
ShellOverlaySearch = Search shortcuts

# The shortcuts overlay's way to Settings › Controls.
ShellOverlayEditKeys = Edit keys…

# Closes a sheet.
ShellClose = Close

# The shortcuts overlay, when nothing matches the search.
ShellOverlayNoMatch = No shortcut matches

# The seated strip at a waiting table. `{0}` the table, `{1}` chairs
# taken, `{2}` chairs, `{3}` ready.
# Arguments: arg0, arg1, arg2, arg3
ShellSeatedStrip = Seated at “{ $arg0 }” · { $arg1 } of { $arg2 } seated · { $arg3 } ready

# The seated strip while the game is played. `{0}` the table.
# Arguments: arg0
ShellPlayingStrip = Playing at “{ $arg0 }”

# The strip's way back to the table.
ShellReturn = Return

# The strip's way to give the chair up (waiting tables only).
ShellLeave = Leave

# The gateway pill's counts. `{0}` tables waiting, `{1}` players online.
# Arguments: arg0, arg1
ShellTablesOnline = { $arg0 } tables · { $arg1 } online

# The gateway pill's count before `/lobby/stats` has answered. `{0}`
# tables waiting.
# Arguments: arg0
ShellTablesWaiting = { $arg0 } waiting

# The lobby feed is down and being dialled again.
ShellReconnecting = Reconnecting…

# Dial the lobby feed again now.
ShellRetryNow = Retry now

# The front door after a `401`: the session ended, nothing typed is lost.
ShellSessionEnded = Signed out — your session ended · Sign in again

# The account pill's menu: sign out.
ShellSignOut = Sign out

# The account pill's menu: back to the gateway picker.
ShellSwitchGateway = Switch gateway

# The account pill's menu: the report form.
ShellReportProblem = Report a problem

# The shell key that opens the friends drawer
ShellKeyFriends = Friends

# The shell key that opens the chat
ShellKeyChat = Open the chat

# The header's chat button
ShellChat = Chat

# The settings row that opens the chat's privacy sheet
ShellChatPrivacy = Chat & privacy

# The bell, with nothing in it.
ShellBellEmpty = Nothing new

# A bell line: somebody sat down at the player's table. `{0}` who.
# Arguments: arg0
ShellBellJoined = { $arg0 } sat down at your table

# A bell line: somebody left the player's table. `{0}` who.
# Arguments: arg0
ShellBellLeft = { $arg0 } left your table

# A bell line: every chair at the player's table is taken.
ShellBellFilled = Your table is full

# The gateway pill while there is no gateway.
ShellOfflinePill = Offline · local decks

# The gateway popover: this client's version follows.
ShellThisClient = This client

# The gateway popover's counts. `{0}` players online, `{1}` tables
# waiting, `{2}` games running.
# Arguments: arg0, arg1, arg2
ShellStatsLine = { $arg0 } online · { $arg1 } waiting · { $arg2 } playing

# The gateway now refuses this client's protocol: an update is needed.
ShellUpdateRequired = Update required — this gateway runs a newer version

# The badge beside a closed-beta gateway's name on the front door.
FrontClosedBeta = closed beta

# The front door's version verdict when nothing is wrong.
FrontCompatible = compatible

# The front door's line naming this client's own version. `{0}`.
# Arguments: arg0
FrontThisClient = this client { $arg0 }

# Under Create account and Play as guest on a closed beta.
FrontKeyHint = This gateway needs a beta key — the key box opens under the button you choose.

# Under the username when an account is made.
FrontUsernameHint = Signs you in; nobody else sees it

# Under a password while Caps Lock looks on.
FrontCapsLock = Caps Lock is on

# The gateway did not answer: the head of the sign-in face. `{0}` the
# gateway.
# Arguments: arg0
FrontUnreachable = Can’t reach { $arg0 }

# Asks the gateway again.
FrontRetry = Retry

# The guest's face's title and its button.
FrontGuestTitle = Play as guest

# The text row's door to the About sheet.
FrontAbout = About

# The one-line colophon's notice: the policy's words, shortened; the
# full notice is on the About sheet one press away (`docs/legal.md`).
ColophonNotice = Unofficial Fan Content

# The one-line colophon's Scryfall credit.
ColophonScryfall = Card data and images by Scryfall

# The one-line colophon's source link.
ColophonSource = Source (AGPL-3.0)

# The About sheet's title.
AboutTitle = About Baylee

# The About sheet: the licence of this program.
AboutLicence = Baylee is free software: you may run, study, share and change it under the GNU Affero General Public License, version 3. Whoever runs a changed gateway offers its players that version’s source. A changed version says so and does not pass itself off as the official Baylee (NOTICE).

# The About sheet: the heading over the third-party licences.
AboutThirdParty = Third-party licences

# The About sheet: the interface's and the card text's typefaces.
AboutFonts = Alegreya Sans (Juan Pablo del Peral, Huerta Tipográfica) and Faustina (Omnibus-Type): SIL Open Font License 1.1.

# The About sheet: the icon font.
AboutIcons = Font Awesome Free 6.7.2 (Fonticons, Inc.): the font files under the SIL Open Font License 1.1.

# The About sheet: the mana font.
AboutMana = Mana 1.18 (Andrew Gioia): SIL Open Font License 1.1. The symbols it draws are Wizards of the Coast’s.

# The About sheet: the engine and the libraries.
AboutCrates = Bevy and the other Rust crates this build links: each under its own licence (MIT, Apache-2.0 and the like), held to an allow-list by cargo-deny; NOTICE in the source names the rest.

# The About sheet: the preconstructed deck lists.
AboutPrecons = Preconstructed deck lists from MTGJSON (Zach Halpern): MIT License.

# Closes a sheet.
SheetClose = Close

# The terms sheet's title. `{0}` the gateway.
# Arguments: arg0
TermsTitle = Before you play at { $arg0 }

# The terms sheet's caption. `{0}` the version.
# Arguments: arg0
TermsVersion = Terms of this gateway · version { $arg0 }

# After the version: when the operator last changed them. `{0}`.
# Arguments: arg0
TermsUpdated = updated { $arg0 }

# While the text is asked for.
TermsLoading = Loading the terms…

# The text could not be read; nothing is accepted.
TermsLoadFailed = Couldn’t load the terms

# Asks for the text again.
TermsRetry = Retry

# Signs out, nothing stored (a guest is asked first).
TermsNotNow = Not now

# Accepts the terms shown.
TermsAccept = Accept & continue

# Why Accept waits.
TermsScrollToAccept = Read to the end to accept

# While the acceptance is on its way.
TermsSending = Sending…

# What accepting stores, and where.
TermsStoredNote = Read to the end to accept. Your acceptance (version and time) is stored with your account on this gateway; this device remembers it too.

# A guest pressed Not now.
TermsGuestAsk = A guest that signs out loses its decks. Sign out anyway?

# The guest stays and reads on.
TermsStay = Stay

# The guest signs out after all.
TermsSignOut = Sign out

# Declines the terms and deletes the account (its own confirmation
# follows). The terms text names this button word for word.
TermsDecline = Decline and delete account

# Decides later: signs out of the gateway, nothing accepted.
TermsLater = Later (sign out)

# For a Baylee account: deletes its data on this gateway (decks,
# settings, friends); the Baylee account itself stays.
TermsDeleteHere = Delete my data on this gateway

# Under the text, for a Baylee account: where the acceptance is kept.
TermsStoredCentral = Read to the end to accept. Your acceptance (version, text and time) is kept with your Baylee account at Baylee Central, so another device does not ask again.

# The heading of what the client itself says about chat, after a
# gateway's terms on the same sheet.
TermsChatTitle = Chat

# What the client itself says about chat: true of every gateway that
# offers chat, since the client does the encrypting, so no gateway's
# text can weaken it.
TermsChatNote = Where a gateway offers chat, messages to friends and to your table are end-to-end encrypted: your device encrypts them, only the recipients’ devices decrypt them, and the gateway passes them on without being able to read them. It sees only who writes to whom, when, and how much. If you report a message, your device sends its text to this gateway’s moderators. Computer opponents and spectators take no part in chat.

# The sheet's title for Baylee Central's own terms.
CentralTermsTitle = Terms of use of Baylee Central

# The caption for Baylee Central's own terms. `{0}` version.
# Arguments: arg0
CentralTermsVersion = Your Baylee account · version { $arg0 }

# The hint of the deletion's confirmation for a Baylee account on a
# gateway: only its data here goes.
DeleteGatewayDataHint = Your decks, settings and friends on this gateway go for good. Your Baylee account stays.

# A clock's span in minutes. `{0}` minutes.
# Arguments: arg0
ClockMinutes = { $arg0 } min

# How long a running table has gone, under an hour. `{0}` minutes.
# Arguments: arg0
RunningMinutes = { $arg0 } min

# How long a running table has gone. `{0}` hours, `{1}` minutes.
# Arguments: arg0, arg1
RunningHours = { $arg0 } h { $arg1 } min

# A clock's span in seconds. `{0}` seconds.
# Arguments: arg0
ClockSeconds = { $arg0 } s

# The `classic` clock's name: three minutes, the default.
ClockClassic = classic

# The `casual` clock's name.
ClockCasual = casual

# The `standard` clock's name.
ClockStandard = standard

# The `blitz` clock's name.
ClockBlitz = blitz

# The `untimed` clock's name.
ClockUntimed = untimed

# The `bullet` clock's name: fifteen seconds.
ClockBullet = bullet

# The `rapid` clock's name: forty-five seconds.
ClockRapid = rapid

# The `quick` clock's name: one minute.
ClockQuick = quick

# The `brisk` clock's name: ninety seconds.
ClockBrisk = brisk

# The `relaxed` clock's name: five minutes.
ClockRelaxed = relaxed

# The `leisurely` clock's name: fifteen minutes.
ClockLeisurely = leisurely

# The `patient` clock's name: twenty minutes.
ClockPatient = patient

# The `unhurried` clock's name: half an hour.
ClockUnhurried = unhurried

# The `marathon` clock's name: an hour.
ClockMarathon = marathon

# The `bullet` clock's help line.
ClockBulletHelp = Fifteen seconds a decision: no time to think twice.

# The `rapid` clock's help line.
ClockRapidHelp = Forty-five seconds a decision.

# The `quick` clock's help line.
ClockQuickHelp = One minute a decision.

# The `brisk` clock's help line.
ClockBriskHelp = Ninety seconds a decision.

# The `relaxed` clock's help line.
ClockRelaxedHelp = Five minutes a decision.

# The `leisurely` clock's help line.
ClockLeisurelyHelp = Fifteen minutes a decision.

# The `patient` clock's help line.
ClockPatientHelp = Twenty minutes a decision.

# The `unhurried` clock's help line.
ClockUnhurriedHelp = Half an hour a decision.

# The `marathon` clock's help line.
ClockMarathonHelp = An hour a decision, the longest there is.

# The `casual` clock's help line.
ClockCasualHelp = Ten minutes a decision.

# The `classic` clock's help line.
ClockClassicHelp = Three minutes a decision; the default.

# The `standard` clock's help line.
ClockStandardHelp = Two minutes a decision.

# The `blitz` clock's help line.
ClockBlitzHelp = Thirty seconds a decision.

# The `untimed` clock's help line.
ClockUntimedHelp = No decision clock; an absent player can still be stood in for.

# A table filter: leave out tables of house AIs only.
PlayHideAiOnly = Hide AI-only

# A table filter: two-chair tables.
PlayDuel = Duel

# The tables' sort: the gateway's order.
PlaySortNewest = newest

# The tables' sort: most open chairs first.
PlaySortOpenSeats = open seats

# The tables' sort: by name.
PlaySortName = name

# Before a Join: the next game's deck is of another format than the
# host's. `{0}` mine, `{1}` the host's.
# Arguments: arg0, arg1
PlayFormatWarning = Your next-game deck is { $arg0 }; this table's host plays { $arg1 }. You can pick another deck in the room.

# A template card: Commander.
TemplateCommander = Commander

# A template card: a twenty-life duel.
TemplateDuel = Duel 20

# A template card: every seat starts with five lands.
TemplateFiveLand = Five-land start

# A template card: decks of every format at one table.
TemplateMixed = Mixed

# The rules line: starting life. `{0}` life.
# Arguments: arg0
RulesLife = { $arg0 } life

# The rules line: one free mulligan. `{0}` the count.
# Arguments: arg0
RulesMulliganOne = { $arg0 } free mulligan

# The rules line: free mulligans. `{0}` the count.
# Arguments: arg0
RulesMulliganMany = { $arg0 } free mulligans

# The rules line: the five-land start.
RulesFiveLands = five lands in play

# The rules line: a mixed table takes a deck of any format.
RulesMixed = decks of any format

# The footer hint: Enter on a table row.
PlayHintJoin = on a row to join

# Why the starts are off while the player's game runs. `{0}` the table.
# Arguments: arg0
PlayPlayingAt = You are playing at { $arg0 }

# Why the starts are off without a game host (`POST /lobby/games` 503).
PlayNoHost = This gateway has no game host right now

# The hero on first run, and why Play the house waits.
PlayPickToStart = Pick a deck to start

# The hero's caption.
PlayYourNextGame = Your next game

# The hero: choose another deck.
PlayChangeDeck = Change deck

# Recent games' caption.
PlayRecentGames = Recent games · this session

# A recent game against the house alone.
PlayVsHouse = vs the house

# A recent game's opponents. `{0}` their names.
# Arguments: arg0
PlayVs = vs { $arg0 }

# A recent game won.
PlayWon = won

# A recent game lost.
PlayLost = lost

# A recent game drawn.
PlayDrawn = draw

# A recent game again, one press.
PlayRematch = Rematch

# A recent game's settings in the Create-table sheet.
PlayEditAndRematch = Edit and rematch

# Over the tables while the player holds a chair. `{0}` the table.
# Arguments: arg0
PlayJoinOff = Join is off while you are seated at { $arg0 }

# A table being played.
PlayPlaying = playing

# A table's chairs. `{0}` taken, `{1}` chairs.
# Arguments: arg0, arg1
PlaySeats = { $arg0 }/{ $arg1 } seats

# A table's house chairs. `{0}` the count.
# Arguments: arg0
PlayAiCount = AI ×{ $arg0 }

# A locked table's inline password box, while empty.
PlayPassword = Password

# The sheet's name field.
SheetName = Name

# The sheet's players control.
SheetPlayers = Players

# The sheet's templates.
SheetTemplate = Template

# The sheet's rules drawer.
SheetAdjust = Adjust

# The sheet's password field.
SheetPassword = Password · optional

# The sheet's empty password box.
SheetNoPassword = No password

# The sheet's clock control.
SheetClock = Clock

# The sheet's way out.
SheetCancel = Cancel

# The sheet's primary over a room.
SheetApply = Apply

# The sheet's primary.
SheetOpenTable = Open table

# The sheet's title over a room.
SheetEditRules = Edit rules

# A new table's name. `{0}` the host's name.
# Arguments: arg0
SheetDefaultName = { $arg0 }’s table

# The room's badge, and its rules line, for a locked room.
RoomPasswordSet = password set

# The room's Copy invite.
RoomCopyInvite = Copy invite

# Copy invite, once copied.
RoomInviteCopied = Invite copied

# The rules rail's first line. `{0}` the format, `{1}` the chairs.
# Arguments: arg0, arg1
RoomRulesPlayers = { $arg0 } · { $arg1 } players

# The rail: no sides.
RoomTeamsOn = Teams: on

# The rail: sides.
RoomTeamsOff = Teams: off

# The rail: sides for each chair.
RoomSetTeams = Set teams

# A chair nobody holds.
RoomEmptySeat = — empty —

# Why Start waits. `{0}` who.
# Arguments: arg0
RoomWaitingFor = waiting for { $arg0 } to be ready

# Why Start waits, said by the gateway's listing.
RoomNotStartable = not every chair is settled yet

# A seat's menu: hand the room on.
RoomMakeHost = Make host

# A seat's menu: the next side.
RoomNextTeam = Next team

# A seat's menu: its own starting life.
RoomLifeOverride = Life override…

# A seat's menu: its starting board.
RoomStartingPosition = Starting position…

# A seat's menu: the house or a model out, a person's chair again.
RoomMakeOpen = Make it an open seat

# Under the house AI's name.
RoomHouseAi = house AI

# A language model's chair, and the button that seats one.
RoomLanguageModel = Language model

# Under the host's name.
RoomHost = Host

# Under a delegated chair. `{0}` whose.
# Arguments: arg0
RoomFor = for { $arg0 }

# An open chair's AI menu button.
RoomAi = AI

# Before another player's deck.
RoomDeckWord = deck

# The seat's deck plays the host's format.
RoomFits = fits

# The seat's deck plays another format. `{0}` the host's.
# Arguments: arg0
RoomOtherFormat = { $arg0 } table

# Over a seat's drawer.
RoomForTesting = For testing and puzzles

# The chair sheet's primary.
RoomSeat = Seat

# The chair sheet without a model to seat.
RoomNoModel = No language model is set up on this computer yet. Under Settings › Language models, add a profile: the provider and model a chair plays, and its key, which stays in this computer's keychain.

# The room's Language model button, off: the install lacks its bridge.
RoomNoBridge = This install lacks the seat bridge (baylee-seat) beside the game; install the latest release again.

# The chair sheet's way to set one up.
RoomSetUpModel = Set up a model…

# The chair sheet's title. `{0}` the chair.
# Arguments: arg0
RoomChairTitle = Chair { $arg0 } · language model

# The host's Leave, asked (DESIGN-v5 §5). `{0}` the table's name.
# Arguments: arg0
LeaveHostingQuestion = Leave “{ $arg0 }”?

# Under it: what leaving does to the table.
LeaveHostingHint = You host this table: it passes to the player seated longest, or closes if nobody else is seated.

# Why Start waits: one chair nobody holds. `{0}` the count.
# Arguments: arg0
RoomOpenSeatOne = { $arg0 } seat is open — a player, the house or a model takes it

# Why Start waits: chairs nobody holds. `{0}` the count.
# Arguments: arg0
RoomOpenSeatMany = { $arg0 } seats are open — players, the house or models take them

# The decks' sort: newest save first.
DecksSortSaved = last saved

# The decks' sort: by name.
DecksSortName = name

# The decks' sort: by format.
DecksSortFormat = format

# The decks' sort: by colour identity.
DecksSortColours = colours

# A deck's format: Commander.
FormatCommander = Commander

# A deck's format: anything without a commander.
FormatFreeform = Freeform

# A table's format: decks of every format welcome.
FormatMixed = Mixed

# A deck saved under a minute ago.
SavedJustNow = saved just now

# A deck saved minutes ago. `{0}` minutes.
# Arguments: arg0
SavedMinutesAgo = saved { $arg0 } min ago

# A deck saved hours ago. `{0}` hours.
# Arguments: arg0
SavedHoursAgo = saved { $arg0 } h ago

# A deck saved yesterday.
SavedYesterday = saved yesterday

# A deck saved days ago. `{0}` days.
# Arguments: arg0
SavedDaysAgo = saved { $arg0 } days ago

# A tile's meta line. `{0}` cards, `{1}` sideboard cards.
# Arguments: arg0, arg1
DeckMeta = { $arg0 } · SB { $arg1 }

# A tile's badge: the deck the next game is played with.
DecksNextGame = next game

# A tile's badge: one card will not play. `{0}` the count.
# Arguments: arg0
DecksUnplayableOne = { $arg0 } card won’t play

# A tile's badge: cards that will not play. `{0}` the count.
# Arguments: arg0
DecksUnplayableMany = { $arg0 } cards won’t play

# A tile's badge: the deck holds a card blocked on this gateway.
# `{0}` the card's name.
# Arguments: arg0
DecksBlockedOne = { $arg0 } blocked here

# A tile's badge: the deck holds several cards blocked on this
# gateway. `{0}` the count.
# Arguments: arg0
DecksBlockedMany = { $arg0 } cards blocked here

# Why a deck cannot be played on this gateway, or a card is marked in
# the builder: a block without an end. `{0}` the card, `{1}` the
# operator's reason.
# Arguments: arg0, arg1
CardBlocked = { $arg0 } is blocked on this server: { $arg1 }

# The same, for a block that ends. `{0}` the card, `{1}` the reason,
# `{2}` when it ends (`2026-10-20 18:00 UTC`).
# Arguments: arg0, arg1, arg2
CardBlockedUntil = { $arg0 } is blocked on this server until { $arg2 }: { $arg1 }

# The deck builder's mark on a card blocked on this gateway.
BuildBlocked = blocked here

# The shelf's search box, while empty.
DecksSearchHint = Search decks…

# Play's table sort button. `{0}` the order (German: the order alone,
# which says it).
# Arguments: arg0
PlaySortBy = Sort: { $arg0 }

# The sort button. `{0}` the sort.
# Arguments: arg0
DecksSortBy = Sort: { $arg0 }

# The shelf, when the search matched nothing. `{0}` the search.
# Arguments: arg0
DecksNoMatch = No deck matches “{ $arg0 }”

# A tile's menu: a copy of the deck.
DecksDuplicate = Duplicate

# A tile's menu: the deck's saved versions.
DecksHistory = History…

# A tile's menu: star the deck.
DecksFavourite = Favourite

# A tile's menu: unstar it.
DecksUnfavourite = No longer a favourite

# The empty shelf.
DecksEmptyTitle = Your shelf is empty — start from a house deck, build one, or import a list.

# The New deck tile's line under a pointer, natively.
DecksNewTileHint = or import a list — Ctrl/Cmd+V or drop a file

# The New deck tile's line under a finger, or in a browser.
DecksNewTileTouch = or import a list ›

# A house deck: a copy to the shelf, staying here.
HouseAdd = Add

# A house deck: its cards, in a sheet.
HousePreview = Preview

# The artist's credit under a picture. `{0}` the artist.
# Arguments: arg0
ArtCredit = Art · { $arg0 }

# The history sheet's title. `{0}` the deck.
# Arguments: arg0
HistoryTitle = History · { $arg0 }

# A past version. `{0}` the number.
# Arguments: arg0
HistoryRow = v{ $arg0 }

# The current version. `{0}` the number.
# Arguments: arg0
HistoryCurrentRow = v{ $arg0 } · current

# Over the changes: what this version holds against the deck now.
HistoryAgainstCurrent = Against the current version

# Over the changes: this version is the deck as it is.
HistoryIsCurrent = This is the deck as it is now

# List every card of the version.
HistoryShowAll = Show all cards

# The history sheet's primary.
HistoryRestore = Restore version

# Why Restore is off: the version shown is the deck as it is.
HistoryRestoreCurrent = This is the current version

# The toast after a restore, with Undo.
HistoryRestored = Version restored

# The compare bar: the selected version against the one before it.
HistoryPrevious = Previous

# The compare bar: against the deck as it is now.
HistoryCurrent = Current

# The compare bar: against a second version picked from the list.
HistoryPick = Pick…

# Under the compare bar while the list waits for the pick.
HistoryPicking = Pick the other version in the list.

# The oldest version kept: its start is not known.
HistoryFirstSave = first save

# A version's day: today (its time follows).
HistoryToday = today

# A version's day: yesterday (its time follows).
HistoryYesterday = yesterday

# A version's change line: copies added and removed. `{0}` `{1}`.
# Arguments: arg0, arg1
HistoryDeltaCopies = +{ $arg0 } −{ $arg1 }

# A version's change line: printings changed. `{0}` how many.
# Arguments: arg0
HistoryDeltaPrinting = { $arg0 }× printing

# A version's change line: finishes changed. `{0}` how many.
# Arguments: arg0
HistoryDeltaFinish = { $arg0 } foil

# A version's change line: languages changed. `{0}` how many.
# Arguments: arg0
HistoryDeltaLanguage = { $arg0 }× language

# A version's change line: notes changed. `{0}` how many.
# Arguments: arg0
HistoryDeltaNote = { $arg0 }× note

# A diff row's tag: only on the newer side.
HistoryTagAdded = added

# A diff row's tag: only on the older side.
HistoryTagRemoved = removed

# A diff row's tag: another count of the same printing.
HistoryTagCount = count

# A diff row's tag: another printing.
HistoryTagPrinting = printing

# A diff row's tag: another finish.
HistoryTagFinish = finish

# A diff row's tag: another language.
HistoryTagLanguage = language

# A diff row's tag: another note.
HistoryTagNote = note

# A diff row: the note changed. `{0}` the card.
# Arguments: arg0
HistoryNoteChanged = { $arg0 } · note changed

# The title's tag on a house deck's history.
HistoryHouseTag = House deck · read-only

# The history sheet's footer: the selected version in the export
# dialog.
HistoryExport = Export…

# The builder, offline, where History would be.
HistoryNeedsGateway = History needs the gateway

# The builder's title while the deck has no name yet.
BuildUntitled = New deck

# The header's save button.
BuildSave = Save

# Why Save is off: nothing has changed since the last save.
BuildNothingToSave = Nothing to save

# The save state: one edit not saved yet.
BuildUnsavedOne = Unsaved · 1 change

# The save state: `{0}` edits not saved yet.
# Arguments: arg0
BuildUnsavedMany = Unsaved · { $arg0 } changes

# The save state while the request is out.
BuildSaving = Saving…

# The save state a moment after a save.
BuildSavedNow = Saved · just now

# The save state `{0}` minutes after a save.
# Arguments: arg0
BuildSavedAgo = Saved · { $arg0 } min ago

# The save state of a deck opened and not changed.
BuildSaved = Saved

# The save state of a new deck nothing was done to yet.
BuildNotSaved = Not saved yet

# The save state after the gateway refused or did not answer.
BuildSaveFailed = Couldn't save

# The header menu: rename the deck.
BuildRename = Rename

# The header menu: empty the deck (it asks first).
BuildEmptyDeck = Empty the deck…

# The header menu: the settings screen.
BuildSettings = Builder settings

# Back with unsaved changes: the question.
BuildDiscardQuestion = Discard changes?

# Back with unsaved changes: what is at stake.
BuildDiscardBody = This deck has unsaved changes. A draft stays on this device until you save or discard it.

# Back with unsaved changes: stay.
BuildKeepEditing = Keep editing

# Back with unsaved changes: leave without saving.
BuildDiscard = Discard

# A kept draft came back when its deck was opened again.
BuildDraftRestored = Your unsaved draft is back

# The search box's syntax help, beside the box.
BuildSyntax = syntax

# The syntax help's title.
BuildSyntaxTitle = What the search understands

# Syntax help: `t:`.
BuildSyntaxType = type line contains

# Syntax help: `o:`.
BuildSyntaxText = rules text contains

# Syntax help: `c:`.
BuildSyntaxColour = at least these colours

# Syntax help: `id<=`.
BuildSyntaxIdentity = colour identity within

# Syntax help: `mv`.
BuildSyntaxMana = mana value

# Syntax help: `pow`.
BuildSyntaxPower = power (also tou, loy)

# Syntax help: `m:`.
BuildSyntaxCost = mana cost holds these symbols

# Syntax help: `is:commander`.
BuildSyntaxCommander = can lead a deck

# Syntax help: `is:partial`.
BuildSyntaxPartial = plays with a known gap

# Syntax help: a minus.
BuildSyntaxNot = not

# Syntax help: `!"…"`.
BuildSyntaxExact = exactly this name

# Syntax help: `or`.
BuildSyntaxOr = either

# The pool's filter disclosure. Chip budget 14.
BuildFilters = Filters

# The filter rail's type section.
BuildRailType = Card type

# The filter rail's mana value section.
BuildRailMana = Mana value

# The filter rail's switches.
BuildRailShow = Show

# The pool's count line, then what narrows it: `{0}`.
# Arguments: arg0
BuildAlsoNarrowing = also narrowing: { $arg0 }

# A pool row's menu: choose a printing.
BuildChoosePrinting = Choose printing…

# A pool row's menu: open the card sheet.
BuildOpenCard = Open card

# The card window's primary through a row door (windows-b6 §A.3).
CardAddToDeck = Add to deck

# The card window's primary through a picture door on a deck row.
CardApplyPrinting = Apply printing

# The card window: set this card as the commander.
CardSetCommander = Set as commander

# The card window: this card is the commander.
CardIsCommander = Is commander

# The card window: turn a two-faced printing over.
CardFlip = Flip

# The card window: unlock the finishes this printing was not sold in.
CardAnyFinish = any finish

# The card window: the rules text shown is the compiled English Oracle.
CardEnglishText = English text

# The card window offline, or with no catalog: one printing only.
CardMorePrintings = More printings need the gateway

# The card window's printing section heading.
CardHeadPrinting = PRINTING

# The card window's count section heading.
CardHeadInDeck = IN THIS DECK

# The card window's main-deck stepper.
CardMain = Main

# The card window's sideboard stepper.
CardSide = Sideboard

# The card window's language chips' label.
CardLanguage = Language

# The card window's finish chips' label.
CardFinish = Finish

# A pool row's menu, or the card sheet: add to the main deck.
BuildAddToMain = Add to main

# A pool row's menu, or the card sheet: add to the sideboard.
BuildAddToSide = Add to sideboard

# A deck row's menu: move one copy to the sideboard.
BuildMoveToSide = Move to sideboard

# A deck row's menu: move one copy to the main deck.
BuildMoveToMain = Move to main

# A deck row's menu: take the whole row out.
BuildRemoveAll = Remove all

# The key-hint footer: arrows.
BuildHintMove = move

# The key-hint footer: Enter.
BuildHintAdd = add

# The key-hint footer: Shift+Enter.
BuildHintOther = other list

# The key-hint footer: `/`.
BuildHintSearch = search

# The key-hint footer: Space.
BuildHintPreview = card

# The commander slot's label.
BuildCommander = Commander

# The commander slot with nobody in it.
BuildNoCommander = none

# The commander slot's way to choose one.
BuildChoose = Choose

# The commander slot's way to add a partner.
BuildPartner = Partner

# Deck tab: the main deck. Tab budget 16.
BuildTabMain = Main

# Deck tab: the sideboard.
BuildTabSide = Sideboard

# Deck tab: the statistics.
BuildTabStats = Stats

# The deck's grouping: by card type.
BuildByType = By type

# The deck's grouping: by mana value.
BuildByManaValue = By mana value

# The deck's grouping: by colour.
BuildByColour = By colour

# Fold every section of the deck list.
BuildCollapseAll = Collapse all

# Open every section of the deck list.
BuildExpandAll = Expand all

# A mana-value section. `{0}` the value.
# Arguments: arg0
BuildManaValue = Mana value { $arg0 }

# The last mana-value section. `{0}` the value.
# Arguments: arg0
BuildManaValueUp = Mana value { $arg0 }+

# A colour section: two colours or more.
BuildMulticolour = Multicoloured

# The deck's footer: one card the engine does not fully play.
BuildShakyOne = 1 card is not fully implemented

# The deck's footer: `{0}` cards the engine does not fully play.
# Arguments: arg0
BuildShakyMany = { $arg0 } cards are not fully implemented

# Stats: the curve's heading.
BuildManaCurve = Mana curve

# Stats: what the curve counts.
BuildCurveNote = nonland cards by mana value

# Stats: which type the curve lights. `{0}` the type.
# Arguments: arg0
BuildCurveLit = { $arg0 } highlighted

# Stats: the colour pips' heading.
BuildColourPips = Colour pips

# Stats: what the pips count.
BuildPipsNote = share of coloured symbols in the deck

# Stats: the land share. `{0}` lands, `{1}` cards.
# Arguments: arg0, arg1
BuildLandShare = { $arg0 } of { $arg1 }

# Stats: draw a sample hand.
BuildDrawSeven = Draw seven

# Stats: what Draw seven does.
BuildDrawNote = a fresh shuffle each press · no card leaves the deck

# Stats: nothing to count yet.
BuildStatsEmpty = Add cards to see the deck's numbers

# The phone's deck rail: the newest additions.
BuildLastAdded = Last added

# The narrow builder's tab bar: the pool.
BuildPanePool = Pool

# The narrow builder's tab bar: the deck.
BuildPaneDeck = Deck

# The builder with no card search on its gateway.
BuildNoCatalog = This gateway has no card search — the pool shows the compiled cards only

# The settings section: graphics.
SectionGraphics = Graphics

# The settings section: sound.
SectionAudio = Audio

# The settings section: language, text, preview.
SectionDisplay = Display & Interface

# The settings section: keys.
SectionControls = Controls

# The settings section: automation.
SectionGameplay = Gameplay

# The settings section: the account.
SectionAccount = Account

# The settings section: the gateway.
SectionNetwork = Network & Gateway

# The settings section: the language-model seat.
SectionLanguageModels = Language models

# The settings section: the updater.
SectionUpdates = Updates

# The settings section: what is kept and sent.
SectionPrivacy = Privacy & Data

# A row's storage tag: this device.
TagDevice = this device

# A row's storage tag: the account, on every device.
TagAccount = your account

# A row's storage tag: this machine's seat file.
TagMachine = this machine

# The settings search field's hint.
SettingsSearch = Search settings…

# The settings search found nothing.
SettingsNoMatch = Nothing matches

# Puts the shown section's rows back.
SettingsResetSection = Reset this section

# Display & Interface, on a chip.
SectionDisplayShort = Display

# Network & Gateway, on a chip.
SectionNetworkShort = Network

# Language models, on a chip.
SectionModelsShort = Models

# Privacy & Data, on a chip.
SectionPrivacyShort = Privacy

# An account row before sign-in.
SettingsSignInToChange = Sign in to change

# The rule over Graphics' account rows.
SettingsAccountRule = Your account's rows hold on every device; a device row never raises them

# The save line under a section.
SettingsSaveLine = Saved at once · tagged rows go with your account

# The save line before sign-in.
SettingsSaveLineOffline = Saved on this device · sign in and the tagged rows follow your account

# Graphics: the preset.
RowPreset = Preset

# Graphics: what the preset does.
HelpPreset = Writes this device's rows below; Custom once one of them differs

# Graphics: the GPU the preset was picked for. {0} its name.
# Arguments: arg0
HelpPresetAuto = Picked for { $arg0 }

# Graphics: windowed, borderless or fullscreen.
RowDisplayMode = Display mode

# Graphics: the display mode's revert.
HelpDisplayMode = A change asks to be kept and reverts in 15 s

# A display mode.
DisplayWindowed = Windowed

# A display mode.
DisplayBorderless = Borderless

# A display mode.
DisplayFullscreen = Fullscreen

# The 15-s revert's question. {0} seconds left.
# Arguments: arg0
KeepDisplayMode = Keep these settings? Reverts in { $arg0 } s

# Keeps a changed display mode.
KeepIt = Keep

# Puts the display mode back at once.
RevertIt = Revert

# Graphics: which monitor.
RowMonitor = Monitor

# Graphics: the monitor row.
HelpMonitor = Which screen the window stands on

# Graphics: edge smoothing's cost.
HelpAntiAliasing = Smooths card edges and lettering · GPU ↑

# Graphics: anti-aliasing on a phone.
AntiAliasingLocked = Off on phones — tilers pay most for it

# Graphics: vsync's effect.
HelpVSync = Waits for the display: no tearing, one frame of delay · power ↓

# Graphics: the frame limit's effect.
HelpFrameLimit = Caps frames per second · lower is cooler and quieter

# Graphics: the rest limit's effect.
HelpRestLimit = Frames per second at a table where nothing has happened for two seconds · power ↓↓

# Graphics: the background limit's effect.
HelpBackgroundLimit = Frames per second while another window has the focus · power ↓↓

# Graphics: the frame-time counter.
RowShowFrameRate = Show frame rate

# Graphics: the counter.
HelpShowFrameRate = A small counter in a corner; a report carries it too

# Graphics: the painting behind the panels.
RowBackdrop = Backdrop

# Graphics: the backdrop's effect.
HelpBackdrop = The painting behind every screen; Plain is the easiest to read and the cheapest frame

# A backdrop.
BackdropPainting = Painting

# A backdrop.
BackdropDimmed = Dimmed

# A backdrop.
BackdropPlain = Plain

# Graphics: ambient detail's effect.
HelpAmbient = Drifting light and dust; never above your account's Atmosphere · GPU ↑

# Ambient detail: standing still.
AmbientStill = Still

# Ambient detail: the lighter one.
AmbientSoft = Soft

# Ambient detail: everything.
AmbientFull = Full

# Graphics: the account's atmosphere.
HelpAtmosphere = The ceiling for ambient detail on every device

# Graphics: the account's reduce-motion switch.
RowHoldStill = Hold the table still

# Graphics: holding the table still.
HelpHoldStill = Cards jump instead of gliding, everywhere; every rule stays

# Graphics: the sky.
HelpSky = The light behind the table: follows your clock, or stays at day or night

# Audio: the master volume.
HelpMaster = Every sound, the music included

# Audio: the music.
RowMusic = Music

# Audio: the music's theme.
RowMusicTheme = Music theme

# Audio: five complete original suites and automatic rotation.
HelpMusicTheme = Fifteen original suites: five in Bb Dorian and ten contrasting styles. Rotating picks a different suite each game. Transitions flow into the next musical pulse

# Audio: suite one.
MusicThemeEmber = Ember Path

# Audio: suite two.
MusicThemeGlass = Moon Glass

# Audio: suite three.
MusicThemeThorn = Thorn Crown

# Audio: suite four.
MusicThemeTide = Mist Harbour

# Additional suite: Velvet Night · Piano.
MusicThemeVelvet = Velvet Night · Piano

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeVelvetShort = Velvet Night

# Additional suite: Copperwork · Baroque.
MusicThemeCopper = Copperwork · Baroque

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeCopperShort = Copperwork

# Additional suite: Juniper · Folk.
MusicThemeJuniper = Juniper · Folk

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeJuniperShort = Juniper

# Additional suite: Lagoon Light · Bossa.
MusicThemeLagoon = Lagoon Light · Bossa

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeLagoonShort = Lagoon Light

# Additional suite: Lanterns · Jazz waltz.
MusicThemeLantern = Lanterns · Jazz waltz

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeLanternShort = Lanterns

# Additional suite: Neon Path · Synthwave.
MusicThemeNeon = Neon Path · Synthwave

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeNeonShort = Neon Path

# Additional suite: Pixelstorm · Chiptune.
MusicThemeCircuit = Pixelstorm · Chiptune

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeCircuitShort = Pixelstorm

# Additional suite: Mosaic · Marimba.
MusicThemeMosaic = Mosaic · Marimba

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeMosaicShort = Mosaic

# Additional suite: Orbit · Ambient.
MusicThemeOrbit = Orbit · Ambient

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeOrbitShort = Orbit

# Additional suite: Iron Pulse · Breakbeat.
MusicThemeIron = Iron Pulse · Breakbeat

# Compact suite button; the full name and genre remain in the selection hint.
MusicThemeIronShort = Iron Pulse

# Audio: suite five.
MusicThemeStar = Starfall

# Audio: a different suite each game.
# Recorded instrument bank.
RowMusicSamples = Instrument sound

# Source and playback rates of the banks.
HelpMusicSamples = Both banks use CC0 recordings made at 44.1 kHz. Studio prepares a high-quality 48 kHz bank; the alternative plays the original bank. Our synthesizers always run at 48 kHz

# Prepared 48-kHz sound.
MusicSamplesStudio = Studio

# Original 44.1-kHz recordings.
MusicSamplesOriginal = Original

# Change suite between games.
MusicThemeRotating = Rotating

# Audio: the music's volume.
HelpMusic = Music throughout the title screen, lobby and game

# Audio: the table's sounds.
HelpEffects = The table's sounds, under your account's Table sounds

# Audio: silence behind other windows.
HelpMuteUnfocused = Silence while another window has the focus

# Audio: the account's table sounds.
RowTableSounds = Table sounds

# Audio: the account's ceiling.
HelpTableSounds = Off, half or full on every device; Off wins over every slider

# Display: the language.
HelpLanguage = The interface's language, and the card text's where the gateway has it

# Display: the five text steps.
RowTextSize = Text size

# Display: the text steps.
HelpTextSize = Five steps for every screen outside the table · Ctrl/Cmd + = − 0

# Display: the card preview's size.
RowPreviewSize = Card preview size

# Display: the preview.
HelpPreviewSize = How large a card shows when pointed at

# Display: the text face.
RowTextFace = Prefer the text face

# Display: the text face's effect.
HelpTextFace = Draw cards as text instead of their print; the modifier key swaps for as long as it is held

# Controls: the shell's shortcuts.
RowShellKeys = Shortcuts

# Controls: rebinding a shortcut.
HelpShellKeys = The keys of every screen outside the table; a key already in use is refused, a second press takes it

# Controls: the table's keys.
RowTableKeys = Table keys

# Controls: the table's keys.
HelpTableKeys = The keys while a game is open

# Gameplay: automation.
HelpAutomation = What the client answers for you; no rule changes

# Gameplay: kept ability answers.
RowAbilityAnswers = Ability answers

# Gameplay: kept ability answers.
HelpAbilityAnswers = Answers you asked the client to keep giving

# Account: the handle.
RowHandle = Handle

# Account: the handle.
HelpHandle = What other players see you as

# Account: copies the handle.
CopyHandle = Copy my handle

# Account: signing out.
RowSignOut = Sign out

# Account: what signing out does.
HelpSignOut = Ends this session here and on the gateway

# Account: what deleting does.
HelpDeleteAccount = The account, its decks and its settings, for good

# Network: the gateway.
RowGateway = Gateway

# Network: the gateway row.
HelpGateway = Where this client plays, and which version it runs

# Network: back to the list.
HelpSwitchGateway = Back to the list of gateways

# Network: the connection's state.
RowDiagnostics = Diagnostics

# Network: diagnostics.
HelpDiagnostics = The connection's state as text, to copy into a report

# Copies diagnostics.
CopyAsText = Copy as text

# Network: playing offline.
NetworkOffline = Offline · local decks, no gateway

# Language models: the profiles.
RowProfiles = Profiles

# Opens a profile's sheet.
ProfileOpen = Edit

# The report's attachments, a disclosure.
ReportAttachments = Attachments

# Beside it: how many are ticked of how many this report has.
# Arguments: arg0, arg1
ReportAttachmentsCount = { $arg0 } of { $arg1 }

# Under the report's text: `{0}` characters of `{1}`, and how to name
# a card or a player (window B).
# Arguments: arg0, arg1
ReportSmallLine = { $arg0 } / { $arg1 } characters · # a card · @ a player

# Why the report's Send is off when no gateway or service is known: the
# route line above it says the rest.
ReportNowhere = Nowhere to send it

# The report's footer: shows the body as it goes.
ReportWhatIsSent = What is sent

# A `#` suggestion's zone: the seat's own hand.
ReportRefHand = Hand

# A `#` suggestion's zone.
ReportRefBattlefield = Battlefield

# A `#` suggestion's zone.
ReportRefStack = Stack

# A `#` suggestion's zone.
ReportRefGraveyard = Graveyard

# A `#` suggestion's zone.
ReportRefExile = Exile

# A `#` suggestion's zone.
ReportRefCommand = Command

# A `#` suggestion's zone: shown for the question being answered.
ReportRefShown = Shown

# A `#` suggestion's zone: a revealed library top.
ReportRefLibraryTop = Library top

# A suggestion's place: a card seen earlier in the game, named by the
# game log and no longer in sight (#14).
ReportRefLogged = Seen this game

# Whose a `#` suggestion is, when it is the reader's.
ReportRefYours = yours

# The confirmation: how many cards the text names, one.
# Arguments: arg0
ReportConfirmCardRef = { $arg0 } card reference

# The same, several.
# Arguments: arg0
ReportConfirmCardRefs = { $arg0 } card references

# The confirmation: how many players the text names, one.
# Arguments: arg0
ReportConfirmPlayerName = { $arg0 } player name

# The same, several.
# Arguments: arg0
ReportConfirmPlayerNames = { $arg0 } player names

# Where a report goes, signed in: the gateway, by name or address.
# Arguments: arg0
ReportGoesGateway = Goes to { $arg0 } with your session

# Updates: checking on its own.
RowCheckUpdates = Check automatically

# Updates: what checking sends.
HelpCheckUpdates = Asks GitHub at start and every six hours; GitHub sees your IP address and this version. Off: no request at all

# Updates: installing on its own.
RowInstallUpdates = Install automatically

# Updates: installing.
HelpInstallUpdates = Downloads a signed update and installs it when you quit

# Updates: asking now.
RowCheckNow = Check now

# Updates: what was found, and why it does not install.
HelpCheckNow = What the last check found, and why an update does not install itself

# Privacy: what is kept and sent.
RowWhatIsKept = What is kept and sent

# Privacy: the true list.
HelpWhatIsKept = The gateway keeps your account, decks, settings and which terms you accepted. This client sends, on its own, only update checks to GitHub (desktop, while checking is on), crash reports once you agree, while an offline game runs an anonymous still-playing count (unless switched off below), and asks Scryfall for card images in a browser

# Privacy: what a report may carry.
RowReportConsent = What a report may carry

# Privacy: what a report carries, chosen on the form before it is sent
# (an opt-out, not a consent; crash reports are asked for apart).
HelpReportConsent = Ticked here, ticked on the report form; a report shows exactly what is sent

# Privacy: crash reports.
RowCrashReports = Crash reports

# Privacy: crash reports.
HelpCrashReports = After a crash: send a report, ask each time, or never

# Privacy: the anonymous offline count.
RowUsageCount = Send anonymous usage count

# Privacy: the anonymous offline count.
HelpUsageCount = While you play offline, says once a minute that a game is running: no account, no device id, nothing about the game

# Crash reports: always.
CrashSend = Send

# Crash reports: ask.
CrashAsk = Ask

# Crash reports: never.
CrashNever = Never

# Privacy: the report form.
HelpReportProblem = Opens the report form (F8)

# Help for the chat's privacy row
HelpChatPrivacy = Who may whisper you, friend requests, chat history, sounds and this account's chat devices

# Opens the report form.
ReportOpen = Report a problem

# Language models on a browser or phone build.
LanguageModelsElsewhere = Language models are set up on a desktop client

# Controls: a key no shortcut may take.
KeyFixed = This key is fixed and cannot be bound

# Controls: a key another shortcut holds. `{0}` the holder.
# Arguments: arg0
KeyHeldBy = Already { $arg0 } — press the key again, or take it

# Controls: takes the refused key.
KeyTake = Take it

# Gameplay: no ability answers kept.
NoAbilityAnswers = None kept yet

# A volume at nothing.
SliderOff = off

# Replays: the keymap's group (the rest of a replay's words are
# `replay::words::ReplayPhrase`).
GroupReplay = Replays

# Replays: a turn back.
ActReplayTurnBack = Replay: a turn back

# Replays: a turn on.
ActReplayTurnForward = Replay: a turn on

# Replays: to the start.
ActReplayStart = Replay: to the start

# Replays: to the end.
ActReplayEnd = Replay: to the end

# Replays: slower.
ActReplaySlower = Replay: slower

# Replays: faster.
ActReplayFaster = Replay: faster

# Replays: the next perspective.
ActReplayPerspective = Replay: switch perspective

# Notification title: general information.
ToastTitleInfo = Notice

# Notification title: error or warning.
ToastTitleAlert = Alert

# Notification title: table event.
ToastTitleTable = Table

# Notification title: social / chat event.
ToastTitleSocial = Social

# Gateway popover: players online.
GatewayOnlinePlayers = Players online

# Gateway popover: players in lobby.
GatewayInLobby = In lobby

# Gateway popover: players in games.
GatewayInGames = Playing

# Gateway popover: waiting tables.
GatewayWaitingTables = Open tables

# Gateway popover: running games.
GatewayRunningGames = Active games

# Gateway popover: server version.
GatewayServerVersion = Server version

# Gateway pill compact inline stats: `{0}` online, `{1}` in lobby, `{2}` playing, `{3}` tables.
# Arguments: arg0, arg1, arg2, arg3
ShellGatewayInlineStats = { $arg0 } online ({ $arg1 } lobby · { $arg2 } playing) · { $arg3 } tables

# Quick settings dropdown heading.
SettingsQuickTitle = Quick settings

# Quick settings audio section.
SettingsAudioSection = Audio

# Quick settings display section.
SettingsDisplaySection = Display & Motion

# Quick settings text size label.
SettingsTextSize = Text size
