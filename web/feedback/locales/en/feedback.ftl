### The feedback service's web UI in English (web/feedback, docs/feedback.md).
### English decides which messages exist and which arguments each takes;
### every other language translates them (docs/translations.md).

nav-reports = Reports
nav-admin = Admin
nav-label = Sections
nav-signOut = Sign out
nav-contact = Contact
nav-skip = Skip to content
nav-signedIn = Signed in as { $name }

contact-title = Contact messages
contact-summary = { $open } open · from the landing page's form
contact-filter = Show
contact-open = Open
contact-done = Done
contact-all = All
contact-none = No messages here.
contact-anonymous = No name given
contact-noEmail = no address to reply to
contact-reply = Reply by e-mail
contact-replySubject = Your message to Baylee
contact-language = Page language
contact-received = Received
contact-doneAt = Marked done
contact-markDone = Mark done
contact-reopen = Reopen
contact-delete = Delete
contact-deleteConfirm = Delete this message for good? Only the audit line stays.
contact-deleteYes = Delete it
contact-deleteNo = Keep it
contact-deleted = The message was deleted.
contact-back = All messages
contact-history = History
contact-noHistory = Nothing done with it yet.
contact-kept = Kept 90 days after it is marked done, a year at most, then deleted by itself.
contact-shown = { $n } of { $total }

admin-title = Overview
admin-updated = Updated { $time }
admin-paused = Paused while the page is hidden
admin-refresh = Refresh
admin-loading = Loading…
admin-noAnswer = The service did not answer.

stats-players = Players
stats-registered = Registered accounts
stats-confirmed = with a confirmed address
stats-online = Online now
stats-inLobby = in the lobby
stats-seated = at a table
stats-sessions = Live sessions
stats-signedIn = accounts signed in
stats-guests = Guests
stats-guestCap = of { $cap }
stats-guestsOff = guests off
stats-noCap = no cap
stats-newAccounts = New accounts
stats-games = Games
stats-running = Running now
stats-local = on this machine
stats-waiting = Tables waiting
stats-started = Games started
stats-finished = Games finished
stats-total = in total
stats-today = today (UTC)
stats-week = 7 days
stats-month = 30 days
stats-server = Server
stats-agents = Agents
stats-capacity = Capacity
stats-unlimited = unlimited
stats-agentGames = games on agents
stats-version = Build
stats-registration = Registration
stats-keys = Beta keys
stats-activeKeys = Active keys
stats-usesLeft = uses left
stats-admitted = Accounts admitted
stats-usedUp = used up
stats-expired = expired
stats-revoked = revoked

invites-title = Closed beta keys
invites-create = Make keys
invites-count = How many
invites-uses = Accounts per key
invites-expires = Expires
invites-never = Never
invites-hours12 = In 12 hours
invites-day1 = In 1 day
invites-days7 = In 7 days
invites-days30 = In 30 days
invites-days90 = In 90 days
invites-note = Note (who it is for)
invites-notePlaceholder = e.g. Max
invites-making = Making…
invites-made =
    { $n ->
        [one] { $n } new key: shown only now
       *[other] { $n } new keys: shown only now
    }
invites-madeHint = Only a hash of each key is kept. Copy them before you leave this page.
invites-copy = Copy
invites-copyAll = Copy all
invites-copied = Copied
invites-share = Share
invites-done = Done
invites-list = All keys
invites-none = No keys yet.
invites-created = Made
invites-state = State
invites-usesLeft = Uses left
invites-admitted = Admitted
invites-expiresAt = Expires
invites-noteColumn = Note
invites-revoke = Revoke…
invites-revokeConfirm = Revoke this key? Nobody can use it after this.
invites-revokeYes = Revoke for good
invites-revokeNo = Keep it
invites-revoked = Key revoked.
invites-neverExpires = never
invites-noKeyShown = Keys are not listed: only their hashes are kept.

state-active = active
state-used_up = used up
state-expired = expired
state-revoked = revoked

audit-title = Recent changes
audit-none = Nothing changed here yet.

sec-label = Admin sections
sec-overview = Overview
sec-live = Live
sec-accounts = Accounts
sec-keys = Beta keys

admin-auto = Auto refresh
admin-retry = Try again

kpi-online = Online now
kpi-onlineSub = { $lobby } in the lobby · { $table } at a table
kpi-games = Games running
kpi-gamesSub =
    { $waiting ->
        [one] { $waiting } table waiting
       *[other] { $waiting } tables waiting
    }
kpi-accounts = Accounts
kpi-accountsSub = { $registered } registered · { $guests } guests
kpi-today = New today
kpi-todaySub = { $registered } registered · { $guests } guests
kpi-played = Games today
kpi-playedSub = { $finished } finished
kpi-reports = New reports
kpi-reportsSub = { $total } reports in total
kpi-offline = Offline now
kpi-offlineSub = anonymous, clients that opted in

chart-games = Games started per day
chart-players = Players who played, per day
chart-accounts = New accounts per day
chart-registered = Registered
chart-guests = Guests
chart-started = Games
chart-finished = Finished
chart-playersSeries = Players
chart-table = Show as a table
chart-day = Day (UTC)
chart-sum = { $n } in 30 days
chart-peak = peak { $n }
chart-hint = 30 days, UTC. Use the arrow keys to step through the days.

stats-uptime = Up for
stats-central = Baylee Central
stats-authMode = Who comes in
stats-terms = Terms of use
stats-on = on
stats-off = off
stats-decks = Players' decks
stats-records = Game records
stats-recordBytes = Stored records
stats-avgGame = Average game (30 days)
stats-data = Data
stats-guestsNew = New guests
stats-withEmail = with an address
stats-byKey = admitted by a key
stats-awaitingEngine = seats awaiting an engine
stats-reports = Reports

live-players = Online now
live-noPlayers = Nobody is online right now.
live-tables = Tables
live-noTables = No table is open.
live-agents = Agents
live-noAgents = No agent is connected: no new game can start.
live-waiting = waiting
live-playing = playing
live-inLobby = lobby
live-atTable = at a table
live-inGame = in a game
live-host = Host
live-noHost = no host
live-open = open chair
# { $level } is the house AI's level, as the gateway names it.
live-ai = House AI · { $level }
live-ready = ready
live-notReady = not ready
# A tag on a waiting room that has a password.
live-locked = password
live-rematch = rematch
live-engine = engine connected
live-noEngine = waiting for its engine
live-local = this machine
live-opened = opened { $when }
live-all = All
live-search = Filter players
live-guest = guest
live-bridge = LLM seat { $name }, for { $by }
live-capacity = capacity { $n }
live-unlimited = no limit
live-agentGames =
    { $n ->
        [one] { $n } game
       *[other] { $n } games
    }
live-clock = { $secs } s per decision
live-noClock = no clock
live-team = team { $n }
live-seats = { $taken } of { $total } chairs taken
live-noMatch = No player matches.

acc-search = Name, username or #tag
acc-searchLabel = Search accounts
acc-kind = Kind
acc-kindAll = All
acc-kindRegistered = Registered
acc-kindGuest = Guests
acc-onlineOnly = Online only
acc-sort = Sort

sort-newest = Newest first
sort-oldest = Oldest first
sort-name = Name
sort-games = Most games
sort-active = Recently active

acc-count = { $from }–{ $to } of { $total }
acc-none = No account matches.
acc-name = Account
acc-created = Created
acc-active = Last active
acc-games = Games
acc-decks = Decks
acc-sessions = Sessions
acc-registered = registered
acc-guest = guest
acc-online = online
acc-inGame = in a game
acc-prev = Previous
acc-next = Next
acc-never = —
acc-activeHint = From the session's last renewal: accurate to 6 hours for an account, a day for a guest.
acc-byKey = beta key
acc-clear = Clear
acc-tagFilter = Only the account tagged #{ $tag }

det-back = All accounts
det-username = Username
det-handle = Shown as
det-id = Id
det-copy = Copy id
det-copied = Copied

mod-title = Act on this account
mod-hint = Every act is logged with your name. A reason is shown to the player.
mod-cancel = Cancel
mod-ban = Ban
mod-banTitle = Ban { $who }
mod-banReason = Reason (shown to the player)
mod-banForever = Permanently
mod-banUntil = Until
mod-banYes = Ban
mod-banNeedsReason = Give a reason.
mod-banNeedsEnd = Choose when the ban ends, in the future.
mod-banWhat = Their sessions end and they leave every table at once. A guest keeps its session, refused until the ban ends.
mod-banned = banned
mod-bannedUntil = Banned until { $when } by { $by }: { $reason }
mod-bannedForever = Banned permanently by { $by }: { $reason }
mod-unban = Lift ban
mod-unbanAsk = Lift the ban on { $who }? They can sign in again at once.
mod-delete = Delete account
mod-deleteTitle = Delete { $who }
mod-deleteWhat = The account, its decks, preferences and pictures go for good; its games stay in the records without its name. This cannot be undone.
mod-deleteType = Type { $handle } to confirm
mod-deleteYes = Delete for good
mod-endGame = End game
mod-endTitle = End this game
mod-endWhat = Every player is told the game was ended and why; nobody wins or loses.
mod-endReason = Reason (shown to the players)
mod-endDefault = Ended by an admin
mod-endYes = End game
mod-endAsked = The engine was told; the table closes in a moment.

det-lang = Language
det-email = E-mail
det-emailNone = none
det-emailUnconfirmed = given, not confirmed
det-emailConfirmed = confirmed { $when }
det-terms = Terms of use
det-termsNone = not accepted
det-termsAccepted = { $version }, { $when }
det-key = Beta key
det-keyNone = none
det-keyUsed = yes
det-now = Right now
det-offline = offline
det-openTable = Show on Live
det-decks = Decks
det-noDecks = No decks.
det-games = Latest games
det-noGames = No recorded games.
det-cards = { $n } cards
det-version = version { $n }
det-complete = finished
det-incomplete = no end recorded
det-seat = chair { $n }
det-house = House AI or no account
det-houseShort = House AI
det-notFound = There is no such account (any more).
det-lasted = lasted { $d }
det-updated = changed { $when }

keys-summary = { $active } active · { $left } uses left · { $admitted } admitted

status-new = New
status-triaged = Triaged
status-in_progress = In progress
status-resolved = Resolved
status-wont_fix = Won't fix
status-duplicate = Duplicate

kind-bug = bug
kind-crash = crash
kind-improvement = improvement
kind-feedback = feedback
kind-other = other
kind-chat = chat message

rep-title = Reports
rep-view-inbox = Inbox
rep-view-triaged = Triaged
rep-view-in_progress = In progress
rep-view-resolved = Resolved
rep-view-all = All
rep-views = Views
# The search box's placeholder; / is the key that moves to it.
rep-search = Search the text  ( / )
rep-searchLabel = Text
rep-filters = Filters
rep-filtersOn = Filters ({ $n })
rep-kind = Kind
rep-status = Status
rep-gateway = Gateway
rep-reporter = Reporter
rep-reporterHint = Pseudonym
rep-sender = Sender
rep-senderUnverified = not verified, so not named
rep-findSender = Find sender's account
rep-from = From
rep-to = To
rep-record = Game record
rep-any = Any
rep-withRecord = With record
rep-withoutRecord = Without
rep-clear = Clear
rep-apply = Search
rep-refresh = Refresh
rep-refreshing = Refreshing…
rep-updated = Updated { $time }
rep-arrived =
    { $n ->
        [one] { $n } new report since you looked
       *[other] { $n } new reports since you looked
    }
rep-show = Show
rep-loading = Loading…
rep-none = No reports match.
rep-range = { $from }–{ $to } of { $total }
rep-col-received = Received
rep-col-kind = Kind
rep-col-status = Status
rep-col-text = Text
rep-col-gateway = Gateway
rep-col-reporter = Reporter
rep-col-record = Record
rep-col-issue = Issue
rep-noText = (no text)
rep-unread = unread
rep-select = Select { $what }
rep-selectAll = Select every report on this page
rep-selected = { $n } selected
rep-setStatus = Set status
rep-copyIds = Copy ids
rep-exportCsv = Export CSV
rep-exported =
    { $n ->
        [one] { $n } report exported.
       *[other] { $n } reports exported.
    }
rep-markRead = Mark as read
rep-deselect = Deselect
rep-bulkDone =
    { $n ->
        [one] { $n } report set to { $status }.
       *[other] { $n } reports set to { $status }.
    }
rep-bulkFailed = { $failed } of { $total } could not be changed: { $why }
rep-shortcuts = Keyboard shortcuts
rep-key-move = move the cursor
rep-key-open = open in the drawer
rep-key-page = open the page
rep-key-select = select
rep-key-selectAll = select all on this page
rep-key-status = set the status (new … duplicate)
rep-key-search = search
rep-key-refresh = refresh
rep-key-copy = copy the link
rep-key-export = export CSV
rep-key-close = close the drawer, clear the selection
rep-key-help = this list
rep-open = Open
rep-openPage = Open the page
rep-close = Close
rep-copyLink = Copy link
rep-copyId = Copy id
rep-copySummary = Copy summary
rep-copied = Copied.
rep-copyFailed = Could not copy.
rep-onlyBy = Only reports by { $who }
rep-cardImage = { $name }, as Scryfall pictures it
rep-scryfallCredit = Image: Scryfall
rep-seatN = seat { $n }
rep-dismiss = Dismiss
rep-direct = direct from a client, unauthenticated
rep-clientRecord = client-supplied, unverified
rep-received = Received
rep-changed = Changed
rep-gatewayBuild = Gateway build
rep-clientBuild = Client build
rep-game = Game
rep-wrote = What the player wrote
rep-refs = References
rep-card = card { $n }
rep-table = At the table
rep-noTable = The report was not written at a table.
rep-turn = Turn
rep-turnOf = turn { $n }, { $who }
# rep-ownTurn and rep-seatsTurn fill { $who } in rep-turnOf.
rep-ownTurn = their own
rep-seatsTurn = seat { $n }'s
rep-phase = Phase
rep-seat = Seat
rep-view = View
rep-viewSeq = sequence { $n }
rep-pending = Question
rep-noPending = none open
rep-awaiting = waiting for seat { $n }
rep-holding = In hand
rep-armed = armed: { $what }
rep-selectedN = { $n } selected
rep-outbox = { $n } unsent
rep-manaRun = tapping lands
rep-lastError = Last refusal
rep-players = { $n } players
rep-crash = Crash
rep-where = Where
rep-thread = Thread
rep-platform = Platform
rep-when = When
rep-backtrace = Backtrace
rep-screenshot = Screenshot
rep-screenshotAlt = The player's window when the report was written
rep-log = The player's game log
rep-logEmpty = The log is empty.
rep-tableJson = The table as the player saw it
rep-everything = Everything the client sent
rep-missing = The player did not include { $what }.
# rep-aScreenshot … rep-theSettings fill { $what } in rep-missing.
rep-aScreenshot = a screenshot
rep-theLog = the game log
rep-theTable = the table
rep-systemDetails = system details
rep-theSettings = settings
rep-recordTitle = Game record
rep-download = Download
rep-complete = complete
rep-partial = partial
rep-noRecord = No record is attached.
rep-recordHint = gzip, JSON Lines inside; replays with baylee_gamehost::record::replay on the build its header names.
rep-system = System
rep-settings = Settings
rep-history = History
rep-noHistory = Nobody has changed it yet.
rep-triage = Triage
rep-issue = GitHub issue
rep-notLinked = Not linked.
rep-unlink = Unlink
rep-link = Link
rep-issueLabel = Issue number or URL
rep-issuePlaceholder = #311 or issue URL
rep-badIssue = Give an issue number (311, #311) or its URL in AceVik/baylee.
rep-statusSet = Status: { $status }.
rep-linked = Linked to #{ $n }.
rep-unlinked = Unlinked.
rep-summaryLabel = Summary for a new issue
rep-summaryHint = Public on GitHub: no personal data, no quotes from the report. Describe the problem in your own words.
rep-category = Category
rep-openIssue = Open a new issue on GitHub
rep-delete = Delete report…
rep-deleteAsk = Delete this report and its record for good?
rep-deleteYes = Delete for good
rep-deleteNo = Keep it
rep-deleted = Report deleted.
rep-back = ← All reports
rep-reportersTop = Reporters by number of reports
rep-reportsN =
    { $n ->
        [one] { $n } report
       *[other] { $n } reports
    }
rep-drawerLabel = Report
rep-unknownBuild = unknown build

prio-critical = Critical
prio-high = High
prio-normal = Normal
prio-low = Low

rep-priority = Priority
rep-noPriority = None
rep-view-open = Open
rep-view-mine = Mine
rep-view-unassigned = Unassigned
rep-view-done = Done
rep-view-trash = Trash
rep-assignee = Assignee
rep-nobody = Nobody
rep-label = Label
rep-labels = Labels
rep-channel = Way in
rep-channelGateway = through a gateway
rep-channelDirect = direct from a client
rep-source = Sent by
rep-noSource = no source given
rep-sourceOthers = all but { $source }
rep-sourceTitle = What the client says sent it (its word, unchecked)
rep-onlyUnread = Only unread
rep-signature = Crash
rep-duplicatesOf = Duplicates of { $id }
rep-inTrashFilter = In the trash
rep-removeFilter = Remove the filter { $what }
rep-sort = Sort by
rep-sortBy = Sort by { $what }
rep-ascending = ascending
rep-descending = descending

sort-received = Received
sort-changed = Last changed
sort-priority = Priority
sort-status = Status
sort-kind = Kind
sort-reporter = Reporter
sort-gateway = Gateway

rep-pageSize = Per page
rep-page = Page { $n } of { $total }
rep-pages = Pages
rep-first = First
rep-prev = Previous
rep-next = Next
rep-last = Last
rep-saved = Saved views
rep-saveView = Save this view…
rep-viewName = Name of the view
rep-save = Save
rep-cancel = Cancel
rep-viewSaved = View “{ $name }” saved.
rep-viewRemoved = View removed.
rep-removeView = Remove the view { $name }
rep-noSaved = No saved views yet.
rep-groups = Crash groups
rep-listMode = Reports
rep-shownAs = Show as
rep-noGroups = No crash matches.
rep-showGroup = Show its reports
rep-col-crash = Crash
rep-col-count = Reports
rep-col-openN = Open
rep-col-first = First
rep-col-last = Last
rep-col-priority = Priority
rep-col-assignee = Assignee
rep-notesN =
    { $n ->
        [one] { $n } note
       *[other] { $n } notes
    }
rep-dupBadge = duplicate of { $id }
rep-setPriority = Set priority
rep-assign = Assign
rep-assignMe = Assign to me
# An entry in the assignee lists: the admin who is signed in.
rep-me = me ({ $name })
rep-addLabel = Add label
rep-removeLabel = Remove label
rep-mergeInto = Duplicates of…
rep-mergeHint = Mark the others as duplicates of the one picked
rep-toTrash = Move to trash
rep-restore = Restore
rep-purge = Delete for good…
rep-purgeAsk =
    { $n ->
        [one] Delete { $n } report and their records for good?
       *[other] Delete { $n } reports and their records for good?
    }
rep-markUnread = Mark as unread
rep-trashed =
    { $n ->
        [one] { $n } report moved to the trash.
       *[other] { $n } reports moved to the trash.
    }
rep-restored =
    { $n ->
        [one] { $n } report restored.
       *[other] { $n } reports restored.
    }
rep-purged =
    { $n ->
        [one] { $n } report deleted for good.
       *[other] { $n } reports deleted for good.
    }
rep-undo = Undo
rep-changedN =
    { $n ->
        [one] { $n } report changed.
       *[other] { $n } reports changed.
    }
rep-bulkRefused = Nothing was changed: { $why }
rep-exportAll = Export all { $n } matching
rep-exportSelected = Export the selected
rep-trashHint = Reports in the trash are deleted for good after { $days } days.
rep-key-assign = assign to me, or to nobody again
rep-key-read = mark as read or unread
rep-key-trash = move to the trash
rep-key-pages = previous or next page
rep-key-neighbours = previous or next report of the list
rep-duplicateOf = Duplicate of
rep-notDuplicate = Not a duplicate
rep-markDuplicate = Mark as duplicate
rep-duplicateLabel = The original's id or link
rep-badDuplicate = Give the original's whole id or its link.
rep-duplicateSet = Marked as a duplicate.
rep-duplicateCleared = No longer a duplicate.
rep-labelLabel = New label
rep-labelHint = lowercase, digits, dashes
rep-badLabel = A label is lowercase letters, digits and dashes, at most 32.
rep-removeLabelN = Remove the label { $label }
rep-labelsSet = Labels saved.
rep-assigned = Assigned to { $name }.
rep-unassigned = Assigned to nobody.
rep-prioritySet = Priority: { $priority }.
rep-conflict = Someone changed this report meanwhile; it is shown as it is now. Try again.
rep-notes = Notes
rep-notesHint = Internal: only admins read them. No personal data, no quotes from the report.
rep-addNote = Add note
rep-noteLabel = New note (Ctrl+Enter adds it)
rep-noNotes = No notes yet.
rep-removeNote = Remove the note
rep-related = Related
rep-sameGame = Reports from this game
rep-sameCrash = Reports of this crash
rep-itsDuplicates = Its duplicates
rep-theOriginal = The original
rep-nothingRelated = Nothing else is linked to it.
rep-inTrashSince = In the trash since
rep-trashDays = It is deleted for good { $days } days after it went in.
rep-trashedOne = Report moved to the trash.
rep-restoredOne = Report restored.
rep-position = { $n } of { $total }
rep-outsideList = not in the list it was opened from
rep-enlarge = Enlarge the screenshot

# A report's history: each audit-* line follows the admin's name and a colon,
# and the ones ending without an object are followed by the value (a status,
# an issue, a name, a priority, labels, a report's id).
audit-status = set the status to
audit-issue = linked
audit-unlinked = unlinked the issue
audit-assign = assigned it to
audit-unassign = assigned it to nobody
audit-priority = set the priority to
audit-unprioritised = cleared the priority
audit-labels = set the labels to
audit-nolabels = removed every label
audit-duplicate = marked it a duplicate of
audit-original = marked it no duplicate
audit-trash = moved it to the trash
audit-restore = restored it
audit-delete = deleted it for good

sec-sets = Sets
sec-models = Models
sec-access = Who comes in
entry-title = Who comes in
entry-lead = Whom this gateway lets in. A change applies to every sign-in from now on.
entry-legend = This gateway lets in
entry-central = Baylee accounts only
entry-centralHint = Players sign in with their Baylee account; no guests. The default.
entry-mixed = Baylee accounts and guests
entry-mixedHint = Players sign in with their Baylee account or play as a guest.
entry-guests = Guests only
entry-guestsHint = Anybody plays with a display name; no Baylee account signs in.
entry-fromEnvironment = Set by the operator (BAYLEE_AUTH_MODE): change it there and restart the gateway.
entry-fromDefault = Nobody has chosen yet, so the default applies: Baylee accounts only.
entry-fromConsole = Chosen by { $who }, { $when }.
entry-applies = Players already signed in stay until they leave.
entry-save = Save
entry-saved = Saved. The next sign-in follows it.
entry-nobody = Nobody can come in: this gateway lets in Baylee accounts only and is not connected to Baylee Central.
entry-pasteToken = Paste a Central token
entry-allowGuests = Allow guests
sec-central = Baylee Central
central-title = Link to Baylee Central
central-lead = With a token from baylee.app, this gateway signs its players in through Baylee Central and keeps its entry in the public gateway list.
central-fromFile = The operator's file (BAYLEE_CENTRAL_CONFIG_FILE) links this gateway; change it there.
central-none = Not linked. No Baylee account signs in here until a token links this gateway; guests come in where it lets them in.
central-rejected = Baylee Central no longer accepts this token: it was revoked or has expired. Make a new one on baylee.app.
central-unavailable = Baylee Central did not answer just now.
central-token = Token
central-setBy = Pasted by
central-gateway = Gateway
central-state = Approval
central-pending = Awaiting approval
central-approved = Approved
central-disabled = Disabled
central-allows = Allows
central-sso = sign-in
central-listing = list entry
central-expires = Valid until
central-pendingHint = Until an administrator of Baylee Central approves the gateway, it is not in the public list. Its token already signs players in.
central-paste = Paste a token
central-replace = Replace the token
central-step1 = Sign in on baylee.app and open Your gateways.
central-step2 = Register this gateway's address there, and make a token.
central-step3 = Paste the token here. The gateway checks it with Baylee Central before it keeps it.
central-link = Link
central-notToken = A token starts with bgt_ and has 64 hex digits after it.
central-linked = Linked. The gateway uses the token now.
central-unlink = Remove the token
central-unlinkConfirm = Players signed in through Baylee Central are signed out at their next check.
central-keep = Keep it
central-unlinked = The token is removed. Revoke it on baylee.app too if nobody needs it.
sec-cards = Blocked cards

cards-title = Blocked cards
cards-summary = { $n } blocked now. A deck holding a blocked card cannot be chosen, made ready or start a game on this gateway; games already running go on, and local games are not affected.
cards-search = Find a card
cards-searchLabel = Card name or index
cards-searchPlaceholder = e.g. Lightning Bolt
cards-searching = Searching…
cards-noHits = No card of this gateway's pool matches.
cards-hint = Only the gateway's pool is searched: a card it cannot play is in no deck here.
cards-block = Block…
cards-blockedNow = blocked
cards-reason = Reason (players see it)
cards-reasonPlaceholder = e.g. crashes the engine, fix in beta.9
cards-duration = How long
cards-forever = Without an end
cards-day1 = 1 day
cards-days7 = 7 days
cards-days30 = 30 days
cards-untilDate = Until a moment…
cards-until = Until (your time)
cards-blockConfirm = Block { $name } { $when }? Decks holding it can no longer start games on this gateway.
# cards-whenForever and cards-whenUntil fill { $when } in cards-blockConfirm.
cards-whenForever = without an end
cards-whenUntil = until { $at }
cards-blockYes = Block it
cards-blockNo = Cancel
cards-blocking = Blocking…
cards-blockedDone = { $name } is blocked.
cards-untilPast = Pick a moment in the future.
cards-active = In force
cards-noneActive = No card is blocked.
cards-history = Earlier blocks
cards-noHistory = None yet.
cards-by = By
cards-made = Blocked
cards-ends = Ends
cards-never = never
cards-lifted = Lifted
cards-unblock = Unblock…
cards-unblockConfirm = Lift the block on { $name }? Decks holding it can be played again at once.
cards-unblockYes = Lift the block
cards-unblockNo = Keep it
cards-unblocked = Block lifted.

cardstate-active = in force
cardstate-expired = ended
cardstate-lifted = lifted

llm-title = Language models
llm-summary =
    { $hosts ->
        [one] { $hosts } seat agent
       *[other] { $hosts } seat agents
    } · { $profiles ->
        [one] { $profiles } profile
       *[other] { $profiles } profiles
    } · { $games ->
        [one] { $games } game running
       *[other] { $games } games running
    }
llm-hosts = Seat agents
llm-noHosts = No seat agent is connected: no hosted model is offered.
llm-connected = connected
llm-local = local
llm-load = Load
llm-since = Connected for
llm-profiles = Profiles
llm-noProfiles = No seat agent holds a profile yet.
llm-profile = Profile
llm-games = Games
llm-today = Today
llm-month = This month
llm-key = Key
llm-key-kept = kept
llm-key-absent = absent
llm-key-none_needed = none needed
llm-key-unavailable = no key store
llm-kind-api = API
llm-kind-cli = CLI
llm-state-available = available
llm-state-busy = busy
llm-state-probing = probing
llm-state-exhausted = exhausted
llm-state-needs_login = needs login
llm-state-failing = failing
llm-state-disabled = disabled
llm-until = until { $when }
llm-lastOk = Last good check { $ago }
llm-on = On
llm-off = Off
llm-enabled = Enabled
llm-canary = Canary
llm-probe = Probe
llm-edit = Edit
llm-delete = Delete
llm-setKey = Key…
llm-keyFor = New key for { $id } on { $host }
llm-keySend = Send key
llm-keyForget = Forget key
llm-keyHint = Write-only: sent once, cleared here, never shown again.
llm-add = Add profile
llm-editTitle = Edit { $id } on { $host }
llm-host = Seat agent
llm-id = Id
llm-label = Label
llm-vendor = Vendor (game data goes to)
llm-maxGames = Games at once
llm-capDay = Cap per day (USD)
llm-capMonth = Cap per month (USD)
llm-noBound = no bound
llm-profileHint = Provider, model, address and the rest of the profile, in the settings file's format (docs/llm-seat.md). The budget above is set in its boxes, not here. Never a key: keys go through Key….
llm-section-profile = Profile
llm-section-load = Games and fallback
llm-section-game = Budget per game
llm-section-caps = Caps of its spend book
llm-section-model = Model
llm-hint-enabled = Off: this model starts no game; with a fallback, new games start there, else nobody is offered it. Games running go on.
llm-hint-canary = A probe also asks the model one paid question: the only way to see a spent credit.
llm-hint-maxGames = From this many on, a new game starts on the fallback if it is available; without one the profile is busy.
llm-fallback = Fallback profile
llm-fallbackNone = none
llm-fallbackMissing = { $id } (not on this seat agent)
llm-fallbackUnfit = { $id } (another vendor, or no API)
llm-badNumber = Every number is 0 or more; an empty box is no bound.
llm-hint-fallback = Takes a game over when this model is rate limited or busy, and new games start on it while this profile cannot take them (full, limited, failing, off). API profiles of this seat agent with the same vendor only. Who may order is decided for this profile alone.
llm-hint-fallbackCli = An agent CLI takes no fallback.
llm-gameUsd = Per game (USD)
llm-hint-gameUsd = The most one game may spend; needs the model's price. Default $5.
llm-gameTokens = Tokens per game
llm-hint-gameTokens = In and out together; the limit of a model without a price. Default 5,000,000 (20,000,000 for a CLI).
llm-gameCalls = Calls per game
llm-hint-gameCalls = Past it the house finishes the game. Default: none for an API, 500 for a CLI.
llm-price = Price (USD / M tokens, in · out)
llm-priceIn = Price in (USD / M tokens)
llm-priceOut = Price out (USD / M tokens)
llm-hint-price = Both or neither: for a model this build has no price for, or a better one.
llm-capDayTokens = Tokens per day
llm-capMonthTokens = Tokens per month
llm-hint-caps = All games of this profile together, in its own spend book (UTC day and month). At a cap the profile is exhausted until the next day or month; with a fallback, new games start there.
llm-badPrice = A price needs both: in and out.
llm-ask-save = Save { $what }? Games already running keep what they started with.
llm-refused = Not saved: { $why }
llm-fallbackIs = Fallback: { $id }
llm-diverts = new games on { $id }
llm-save = Save
llm-saving = Saving…
llm-cancel = Cancel
llm-badId = Seat agent and id: 1–64 of A–Z, a–z, 0–9, '.', '_' or '-'.
llm-badNames = Label and vendor: 1–40 characters each.
llm-badProfile = The profile is not a JSON object.
llm-assign = Assignment
llm-assignHint = Which profiles run on which seat agent. Ticking copies the profile's definition there; a key is not copied.
llm-unassign = Remove
llm-ask-delete = Delete { $what }? A kept key stays in the seat agent's store.
llm-ask-unassign = Remove { $what } from this seat agent?
llm-ask-forget = Forget the key of { $what }?
llm-done-write = Profile saved.
llm-done-delete = Profile removed.
llm-done-enable = Profile enabled.
llm-done-disable = Profile disabled.
llm-done-probe = Check started.
llm-done-key = Key sent.
llm-done-keyForgot = Key forgotten.
llm-done-assign = Profile copied to the seat agent.

access-title = Who may seat each model
access-hint = Per profile: every registered account, only the accounts on its list, or nobody. Guests never. A new profile is open to nobody until you open it. A change stops no game and no chair already ordered.
access-noProfiles = No profile is reported or has a policy yet.
access-unset = not set: nobody
access-listedN = { $n } on the list
access-isDefault = default for registered players
access-notReported = no seat agent reports it now
access-reported = reported by a seat agent
access-manage = Manage access
access-editTitle = Who may seat { $name }
access-policy = Policy
access-policy-registered = Every registered player
access-policy-listed = Only the players on the list
access-policy-nobody = Nobody
access-policyHint-registered = Any signed-in account that is not a guest sees and may seat it.
access-policyHint-listed = Only the accounts below see it; everybody else is shown nothing.
access-policyHint-nobody = Hidden from every player; nobody may seat it.
access-default = The default model for registered players
access-defaultHint = At most one profile is the default; marking this one takes the mark from any other. The default is always open to every registered player.
access-list = Allowed players
access-listIdle = The list is kept, and counts only while the policy is "only the players on the list".
access-find = Add a player
access-findPlaceholder = Username or display name…
access-typeMore = Type at least { $n } characters of a username or display name.
access-searching = Searching…
access-noMatch = No registered player starts with that.
access-matches = { $n } found: ↑ ↓ to choose, Enter to add.
access-already = already on the list
access-toAdd = To add ({ $n }):
access-dropAdd = Do not add { $who }
access-toRemove = To take off ({ $n }):
access-keep = Keep { $who }
access-empty = Nobody is on the list yet.
access-filter = Filter the list
access-sort = Order
access-sortName = by name
access-sortNewest = newest first
access-sortOldest = oldest first
access-selectShown = Select all shown
access-select = Select { $who }
access-removeSelected = Take off selected ({ $n })
access-noneShown = No player on the list matches the filter.
access-addedBy = added { $ago } by { $by }
access-remove = Take off
access-undo = Keep
access-saveEllipsis = Save…
access-review = Apply these changes?
access-reviewPolicy = Policy: { $policy }.
access-reviewDefault = It becomes the default for registered players.
access-reviewNotDefault = It is no longer the default.
access-reviewAdd =
    { $n ->
        [one] Add { $n } player:
       *[other] Add { $n } players:
    }
access-reviewRemove =
    { $n ->
        [one] Take { $n } player off:
       *[other] Take { $n } players off:
    }
access-reviewNote = Games already playing and chairs already ordered are not affected.
access-confirm = Apply
access-back = Back
access-saved = Access to { $name } saved.

srv-title = Server
srv-coresN = { $n } cores
srv-lastHour = last hour
srv-lastMinutes = last { $n } min
srv-noSample = No sample yet: the gateway has just started.
srv-cpu = CPU
srv-load = load { $l }
srv-memory = Memory
srv-disk = Disk
srv-ofTotal = of { $total }
srv-network = Network
srv-requests = Requests
srv-perSecond = { $n }/s
srv-sockets = { $n } open sockets
srv-socketsLabel = open sockets
srv-uptime = Gateway up
srv-gatewayProcess = this gateway process
srv-hostUp = host up { $d }
srv-engines = Engine processes on this machine
srv-noGames = No game is running on this machine.
srv-noProc = Only on Linux: this gateway cannot read its engines' processes here.
srv-cores = { $n } cores
srv-sparkOf = { $what }, the last hour
srv-gameSpark = CPU of game { $id }, the last hour
srv-services = Services
srv-feedback = Feedback service
srv-gateway = Gateway
srv-reachable = answered in { $ms } ms
srv-unreachable = did not answer
srv-checking = checking…

sets-title = Pool progress
sets-all = All sets
sets-wholePool = The whole pool
sets-ofCorpus = of { $total } cards playable ({ $pct } %)
sets-playable = Playable
sets-playableParts = { $implemented } implemented + { $generated } generated
sets-latestClient = Latest client
sets-implemented = Implemented
sets-generated = Generated (untested)
sets-partial = Partial
sets-unimplemented = Stub
sets-absent = Not started
sets-howCounted = A set counts the cards first printed in it, from this build's ledger and compiled pool; reprints count under their first set.
sets-search = Set code
sets-sort = Order
sets-sortRelease = release order
sets-sortSize = most cards
sets-sortDone = most complete
sets-sortOpen = most left to do
sets-onlyOpen = Unfinished only
sets-count = { $n } of { $total } sets
sets-set = Set
sets-cards = Cards
sets-progress = Progress
sets-filter = Show
sets-allCards = All
sets-searchCards = Card name
sets-cardCount = { $n } of { $total } cards
sets-noCards = No card matches.
sets-position = set { $n } in release order
sets-onScryfall = On Scryfall

chart-reports = Reports per day
chart-bugs = Bugs and crashes
chart-otherReports = Other reports
