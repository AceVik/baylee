# Account entry, collection and deck history

## Ranked play

The illustrated Ranked activity sits beside the table list, with Tower, Daily
and Weekly (two rows of two equal tiles). Its tile unfolds nothing in place: it
opens the Ranked page, and its line under the title says how the queue stands
(searching, a host found, the last result) or, idle, the account's rank and Elo
in the queue's format. The page stands a Central account's desk beside the
public ladder on a wide window (above it on a narrow one, the whole page
scrolling; a guest reading the ladder from the front door sees the ladder
alone): the standing (rank badge, Elo, the season's record, a bar of the ten
placement games while unranked), once a game is played its figures in three
equal cells (the season's win rate, its games, the games of every season:
`queue::figures_row`, `parts::stat`, the cell the Daily and Weekly detail uses
too), the deck selector, the search and its result card. A Central account can queue for a duel, check its status and
cancel there; the desk keeps one width whatever it says, so the ladder beside
it never moves. Opening or leaving the page never enters or leaves the queue.
Without a legal deck in the format, the desk shows an empty state that leads to
the deck library.

The ladder is the same public seasonal ladder the front door opens; its first
three places wear gold, silver and bronze medals, an empty group shows the
shell kit's empty state, and the pager offers its arrows only when there is a
second page. Guests can read it immediately. One format choice, in the page's head,
serves the ladder and the desk (the queue keeps its own while an entry waits
or is seated). Standard, Highlander and Commander each
have separate player and deck standings. The league menu offers the
divisions that exist in that league. The season button also lists past seasons.
The list keeps its scroll position during a refresh and starts at the top
after a filter or page change. Each row opens its player's public profile.
An unnamed account stays anonymous in the profile.

Opening the optional deck-Elo preview sends Central the selected deck and
sideboard's hashed card identities with copy counts. The returned projection
names its source, edit deduction and placement progress. The worker binds
the result to the account, format, deck and saved revision. Looking at a
projection does not reserve a seat or change either rating.

The selected deck is frozen before a queue entry is sent. Central selects the
verified game host and the client opens a separate seat connection to it. The
home gateway and chat session continue on their original connections. The
queue card displays elapsed time, cancellation while waiting, deck recovery
and the option to resume a previously opened game. Once Central settles that
game, the receipt shows each rating change. A division, league or Champion
podium promotion plays once per locally entered match. Simultaneous player
and deck promotions play in sequence. Reduced motion keeps the presentation
still. Placement counters always belong to the selected format and track.

The assigned host receives a game-scoped admission. When it requires new
terms, the client shows the verified origin and the version published at
Central. Accept continues that assignment. Decline releases the reservation.
Long games renew their own admission while the original seat remains open.

On macOS the passive window tests selected by `cargo dev e2e ranked_`
exercise the public board against loopback data and drive the queue on the
Ranked page, opened from the native lobby's tile. The latter uses a synthetic account and queue replies
through an explicit development fixture. The gateway integration test covers
the separate network path through Central and the assigned game host.

## The Advent calendar

Signed-in players enter the calendar from the Play tab. Central selects the
campaign and supplies its clock. Its 24 doors stay in the same positions on
the tavern illustration. Every day opens another puzzle. Released doors
stay playable through 6 January. The button glows while a released door
still needs attention. Reduced motion keeps the light still. Each available
door has a keyboard stop and an accessible label with its number and state.

A puzzle shows both sides, fixed future draws and the opponent’s announced
combat assignments. The player can reveal two hints, retry and submit the
complete move sequence. Central verifies that sequence and writes a winning
reward to the account inventory in the same transaction. The reveal button
then opens the reward and its deck selection. Earned artwork stays in the
collection after the event closes. The client fetches the campaign illustration
and earned designs through authorized Central requests and keeps decoded
images in memory only.

## Tower Echoes run client

`baylee-client-core::echoes` is the new renderer-free state machine for the
separate global Tower service. It permits one account-worker operation at a time,
keeps an ambiguous mutation's exact request identity and applies no optimistic
gold, offers or progression. Read/navigation never opens a game. Only explicit
Reserve/Play/Retry can request a seat. A confirmed reservation whose subsequent
gateway admission fails resumes without another opponent reservation.

Historical retry receipts resolve uncertainty without replacing a newer displayed
revision. A read alone cannot identify which uncertain mutation committed. Old
worker responses are ignored; the native adapter must additionally discard the
account-scoped mailbox and connection on sign-out. Quarantine cannot become a
defeat or open the shop. The account SDK provides the confirmed gateway handover.
The selected run's immutable catalog can be read separately after enrollment
closes; fetching that catalog does not make the cohort available for a new run.
The native `lobby::echoes` adapter uses the existing serial account worker and a
private cached Tower connection, renewed before its five-minute expiry. Account
changes discard both the mailbox and the connection. A committed reservation's
response remains acknowledged even if seat admission subsequently fails. Only
the separate approved gateway handover reaches the table; opening a panel or
refreshing never reserves an opponent. Returning from a Tower duel rereads the
run and waits for the verifier; it never invents an outcome or awards gold.
The ordinary authenticated seat-recovery route also carries a separate `tower`
expedition marker. The client restores it after its gateway epoch check, so a
restarted client does not offer a multiplayer rematch or return to a Central task.
If technical recovery has replaced the game identity, the old game's seat cannot
be reclaimed; the saved expedition's explicit Resume follows its fresh authority
location instead.

After gateway selection, the authenticated Lobby places Ranked, Tower, Daily
and Weekly beside the table list as two rows of two equal tiles (Ranked beside
the Tower, the Daily beside the Weekly; one column on a phone), the Tower
only where the account's Central delegates one: the client asks once per
account, on the account worker, for a Tower grant (`echoes::native::reach`),
and a Central without a Tower answers `Unavailable` (owner, 27.11.2026: Tower
Echoes is not live for the open beta; a tile that opened onto an error must not
stand). Until it is known there is no tile, no Tower collection in the account
menu and Ranked takes its row alone; a yes keeps its connection for the first
visit. the Daily and
the Weekly counting down what is left of their task under its name. There is no
Tasks navigation tab. Each new Echoes run chooses one of five 39+1 Commander
starters and enters its own shop. The current shop displays six equipment
slots, private supplies, a duplicate-capable icon shelf and paginated reserves.
A purchase automatically combines three identical items across equipment and
reserves, preserving the first equipped slot. The shelf can be frozen across
rerolls and round transitions; purchased positions refill. Run-owned card
claims precede each return to the shop.

Purchases, equipment changes, rerolls and ordinary automatic fusion execute
their explicit control directly; enrollment, Mythic upgrades, retirement and
echo withdrawal retain confirmation. Confirmation is bound to the displayed run
revision; the pure domain validators determine which choices are available. Deck edits
stay local until saved, and an unsaved deck cannot enter a duel. A read of the
same revision retains that draft; a different confirmed revision invalidates old
choices. German/English phrases, icons, UTC creation timestamps and semantic
keyboard focus accompany these controls. Catalogs and owned inventory are bounded
and validated before the renderer receives them.
The retained header follows the visible body's keyboard order: Play for the
activity tiles, Challenges for Daily/Weekly, Echoes for Tower. Tower opens with
Back focused; its confirmations start on Cancel and trap Tab until dismissed.
The brand remains the last stop in the Tower page rather than selecting a second
competing page order.

The duel drawer reuses the shop's item art, rarity frames and localized rules.
Shift+I opens its modal keyboard focus order. It shows both players' public
equipment but only the local player's potion stock. Activating an item opens the
ordinary ability chooser; auxiliary run sources anchor that chooser without
pretending to be battlefield cards. Account-scoped permanent medals are read
directly from Central, independently of Tower service availability. Only confirmed
counts establish earned tiers; unavailable counts stay unknown, and refresh
failures preserve the last confirmed projection. A departed session loses its
collection and mailbox. Three original medal motifs share the bounded procedural
cache and offer explicitly labelled previews of all five tiers.

The overview's Leaderboard opens the exact selected run/publication cohort,
with separate all-player-echo and house-opponent tabs. It presents a bounded
top 25, shared ranks, and the owner's best even outside that podium. Personal
bests default to private. Publishing requires an explicit explanation and
confirmation; hiding is immediate after the authority acknowledges it. The
client retains the same privacy revision across ambiguous retries and never
predicts scores, rank or publication success. Cohort/lane switches reset scroll
and have distinct keyboard stops. Display names are confirmed Central public
identities, not names supplied by the player client. See the operator guide for
retention and cross-service deletion limits.

Native rendered passes and real shop/duel/reward interactions are recorded in
`docs/proposals/tower-run-redesign.md`; headless tests alone do not establish
visual acceptance or gameplay balance.

## Global Daily and Weekly challenges

The native Lobby is the home directly after Central sign-in. After gateway
selection, equal Daily/Weekly tiles lead to task details, with the permanent
collection and archive accessible within that view. Both read the
shared calendar, archive, task rules, personal
progress, three independent rankings and permanent badges from the account's
existing worker. No gateway owns this state. Public leaderboard listing defaults
to off; revealing either hint requires confirmation before Central permanently
marks the task assisted. Abandonment also requires confirmation. Escape cancels
the confirmation. All controls use the shell kit and English/German Fluent words.
Its registered Tab order reaches task/ranking composites with arrow keys and
activates controls with Enter/Space. Confirmation traps Tab on Cancel/Confirm.

Both pages stand on the shell kit's panels (`challenges/draw/layout.rs`: a
section is `surfaces::panel`, a block inside it an inset on the control ground,
a group has a small-capitals caption, body copy is body size, a number stands
in a stat cell). The calendar reads, in order: the page's head with Refresh; an
attempt left open on a task that is no current card; the Daily and the Weekly
as two equal cards (landscape, kind, name, tactician goal, closing time and
what is left of it, one primary, gold when an attempt of it is open); the two
expeditions as two equal cards behind them (secondary, gold only for a run
begun); the permanent badges; the archive as one list. A narrow window stacks
each pair. State lines (first read, failure, pending result, no task) stand on
mist plates, never bare on the painting, and a minute's re-read shows none.

A task's page leads with Back. Its card reads as a player decides: landscape,
kind and state, name, instructions, the goal with the opening and closing times
and a countdown (rewritten once a minute, never by a rebuild), "Your progress"
as three stat cells, each behind its glyph, then Play (or gold Resume) with
Hint beside it and Abandon at the row's far end, the confirmation or a seen
hint directly under them, consent beside its switch, and the supplied deck and
the scoring rules folded at the foot. The rankings stand beside it (under it
below the Wide frame) as one table: rank (the first three behind gold, silver
and bronze medals, as on the ranked ladder), player, turns, failures, active
time, the player's own row tinted; with no score yet the shell kit's empty
state stands in its middle. The Tab order follows that reading order. Choosing
a badge shows its five tiers, exact requirements and earned status. Five
original procedural motifs gain bezels, laurels, gems and crowns through Bronze
→ Obsidian. Together with three retained Tower motifs and three current Echoes
motifs, sixty-six shared material slots bound the cache. Motion reads the GPU
clock, respects reduced motion and ambient-effects settings, and does not
rebuild the UI or upload uniforms on idle frames. Motion reduction remains
adjustable before gateway admission, stored locally by the existing preferences
store. Opaque reading surfaces and scrolling over both controls and empty space
support the minimum desktop and enlarged text.

The compact Connection control opens connection selection or its diagnostic
popover; connection details do not occupy the home. Multiplayer leads to the
existing table browser, asking for gateway selection when one is needed.

Play reserves with a retained UUIDv7, then re-reads the owned attempt and admits
the player at the official host Central names. An ambiguous reply retries that
identity; a known active attempt resumes without another reservation. The game
uses the ordinary network seat and existing choice interface. Its end screen
returns to Challenges and offers no unaccounted rematch. While the durable result
is pending, the view explains that progress is awaiting confirmation.
Ordinary authenticated seat grants also carry an optional public task identity.
The native client accepts it only with the current gateway response epoch and
binds it to that exact game, so update restarts and normal seat recovery retain
the challenge end screen and result refresh. Unmarked ordinary grants clear the
old context; old gateways may omit the field. It confers no scoring authority.

The catalogue refreshes on a visit and every minute while shown, or every ten
seconds with an active attempt. Responses are scoped to the exact Central worker:
sign-out or another account discards private state and the old response mailbox.
Idle frames retain the UI tree. A `baylee://challenge/<32 lowercase UUIDv7 hex>`
link opens details only; it never begins a game or reveals a hint.
Links wait while a challenge request or confirmation is outstanding.

The isolated native check is `cargo dev central client --monitor 1 --challenges`.
Build the dev-control client and the `challenge` publisher first. It runs the real
publisher twice against its owned PostgreSQL/Central fixture, checking that retry
publishes nothing twice. `--hold-seconds 3600` keeps that signed-in client available
for blind play; the printed `finish` marker ends the hold early. The Rust owner
always stops its client and backend and verifies the listeners closed. Private
proof files remain in the run's restricted evidence directory and must not be
read during a blind difficulty check. Monitor selection follows the current
workstation's dedicated-display requirement.

## Player profiles and ranked portrait frames

The native client opens a player's profile from the account menu, an occupied
human seat, the portrait in a contacts row, a chat author's menu or a named
leaderboard entry. Each format shows its independent placements, player standing,
best published deck and season and career records. The profile's listed honours
come from Central. An authenticated reader opening their own profile also sees
earned medals from Daily and Weekly tasks, the weekly Tower and Tower Echoes.
Those three collections use the owner's existing authenticated account worker.

Avatar discs and social hover cards share a bounded memory cache keyed by full
Central handle. The highest confirmed player standing across formats supplies
the portrait frame. Each public reply must match the complete name and tag.
Only one background public read runs at a time, and a confirmed match receipt
refreshes the owner's record. Account changes detach the previous mailboxes.
An unavailable read preserves the last confirmed record, while a rejected
identity removes its cached frame. Replacing the body of an open profile
preserves its scroll container, header and focus targets.

## Accounts and decks

Every container wears one glass (owner, 26.11.2026: the look of the sign-in
card and its dialogs, for the contacts, the chat, the list of tables and every
other container; `shellkit::glass`, docs/client.md §"One glass for every
container"). The leather the account screens once borrowed from the hand HUD
(`lobby/dock.rs`, seven material slots) is gone with it. Decorative children
ignore picking. Native/browser text editing, selection, paste and IME continue
through the existing text-buffer and soft-keyboard paths.

A native client starts at Baylee Central's sign-in, and its *Continue without an
account* opens the locally stored gateway list, which *Sign in with an account*
leaves again (`docs/central-login.md`). Adding an HTTP(S) address does
not select it; selecting one enables sign-in and probes its registration policy.
No account request can be submitted before selection, including by keyboard.
Gateway changes discard the previous account state and art-mirror configuration.
HTTP replies carry a session generation so responses from a departed gateway or
account cannot populate the new screen. Addresses are device settings, never
account preferences; tokens and passwords are not stored with them.

The signed-in home separates **Lobby**, **Multiplayer** and **Collection**.
Multiplayer (and offline Play) presents the selected deck,
table search and explicit table creation with a seat-count control. Collection
offers personal decks and house decks. The builder gives the working deck the
larger desktop pane, with the searchable card catalog beside it; narrow screens
use tabs. Quantity-weighted mana value, distinct cards, land share and opening
seven land probability complement the mana curve, colored pips and legality
feedback. Opening-hand statistics exclude the sideboard and apply before
mulligans.

Personal collections follow a signed-in Central account across gateways.
The account worker reads and writes encrypted deck snapshots, checks the
revision the editor opened and preserves a newer remote save on a conflict.
The first save of a visit can archive the previous head. Later autosaves
replace the working version. House decks come from the selected host or the
bundled catalog when playing without a gateway. Local gateway accounts,
guests and offline players use their existing collection.

A room says what it holds a deck to in its rules rail (`lobby/room/rules.rs`): the
list, or that every deck may sit down; a running game adds the version it froze. The
host's *Ban list…* opens a sheet of the official lists and their own, and a press
picks one (`POST …/list`); those at the table read its cards through *Read the list*.
To anyone not at the table an own list has no name, because a list is its owner's
until a table uses it. A deck a table's list turns away is told why in a sheet over
whatever screen it came from (`confirm::draw_refusal`, `Lobby::refusal_lines`), a
sentence a reason, in the same words the deck builder uses. Offline there are no
lists: they live on a gateway. The rules are `docs/banlists-and-formats.md`.

For a signed-in Central account, creating or joining a casual table and assigning a
deck to a chair resolve the revision shown in the shelf on the account worker.
An edit on another device during this read cannot change the submission. The client sends
the playable rows, commander names, deck name and format to the selected gateway.
A sleeve or playmat travels only to the gateway that owns it. Row notes, edit
messages, source links and library identities stay private. Gateway selection and
account session are checked again after the read. A gateway announces frozen deck
support with `submitted_decks` in `GET /auth/config`. A client selecting an account
deck at an older gateway can choose a compatible host.

Under **My decks → Import saved decks**, a Central user can copy the device's
saved decks or the selected gateway's private collection. The gateway source
requires an active session at that exact origin. Each Central worker job reads
one gateway page or publishes one deck. Cancel stops subsequent jobs. Repeating
an import reuses the per-source markers of already copied decks. The device
import keeps row notes, print choices, legacy commander names and original save
times. A damaged or unsupported source produces a visible failure and leaves
the original file untouched. Device-only cosmetics remain in the encrypted
collection and carry no online gateway authority.

House decks are read-only server publications. Copying one refreshes the account
deck list and opens its private copy. Only a persisted deck belonging to the
signed-in account offers history. Selecting a version reads a snapshot without
changing the working deck. Diffs cover main deck, sideboard and commanders,
aggregate quantities and retain printing/notes. Restoring requires an explicit
second click and uses the server's append-only revert: later versions stay
available and can themselves be restored. Historical dates identify when a
version was superseded; the current date identifies its save.

Implementation and regression coverage: #184, #185, #186 and #187. Core tests
cover gateway submission guards, library ownership, late responses, copy/load
sequencing, nonmutating previews, restore confirmation and zone diffs. Bevy tests
exercise controls at phone/tablet/desktop widths and retained idle frames. The
dev-control `/state` endpoint exposes `lobby_controls` with action names and hit
rectangles, without input values, for live UI verification. Live review uses a
separate client configuration and PostgreSQL schema, including a house-deck copy,
a sideboard edit and restoration in both directions.

The follow-up in #188–#193 keeps the editor's catalog and deck panes independent:
search edits replace the catalog pane, quantity edits replace the deck pane, and
printing changes replace only the dialog. Identical lobby listings and account
transport bookkeeping do not invalidate the UI. Settled hover colors are not
written each frame. Small card images load only inside a scroller's visible clip,
with four new requests per frame and a bounded 256-image reuse cache.

Catalog rows expose explicit main-deck and sideboard additions. Deck rows keep
quantity, printing and zone-transfer actions together. A printing edit preserves
all copies and notes in that row and records the exact image identity. If a gateway
has no catalog, the client can retrieve printing and finish metadata from Scryfall;
unavailable finishes are disabled. Deck name and search share the account fields'
text buffer, including selection, navigation and native clipboard shortcuts.

House decks are also available offline as copies of the bundled decks. Account
history is reachable from both the collection and the editor; offline/unsaved
editors explain why history is unavailable. Deleting a saved deck and emptying
both editor zones require a named confirmation dialog with Cancel/Escape.

Commander management (#194) has its own always-visible section: choose/replace,
remove a role without removing the card, and add a rules-compatible partner.
The gateway's pool exports partner compatibility computed by the existing deck
legality rules; older gateways safely default to no advertised partners.
Each commander is drawn as a deck row draws its card (#255): the picture of
the printing the deck holds, at the row's full height, with the same hover
preview. A click on the picture opens the printing picker on the commander's
own row in the deck (`DeckBuilder::commander_row`), even while the sideboard
is on screen; a click on the rest of the line reads the card. A commander
moved out of the deck keeps its line with the pool's picture and no picker.

The blue-hour ambient field is procedural. The gateway and sign-in faces
stand in a geode's cavity with a world in the same key beyond it (#295,
`vista.rs`, `docs/client.md` §"The front door stands in a scene"), and
choosing a gateway walks through it. Shared HUD buttons use the same
borders, bevels and interaction treatment as the table's draw/concede controls.
Statistics expand on demand. Card list row geometry stays mounted, while only
visible controls plus a small overscan are mounted; scrolling does not rebuild
the editor. Hover previews are larger, shadowed, and expire with their source row.
History opened from the collection compares against that deck's latest saved
snapshot, independently of any unrelated working draft.

September 2026 follow-up validation: repeated native debug runs at 1600×938,
scale 2 on M1 Max measured about 44 FPS before the follow-up and 58 FPS with
viewport row mounting (24 state samples, 250 ms apart; not a release benchmark).
Control identities stayed unchanged during idle measurement. Live checks covered
scrolling, primary commander selection/removal, printing/finish changes, offline
house copies, and signed-in house copy → sideboard save → historical restore →
restore a later version. The temporary account deck was deleted after testing;
existing account decks were not edited. Native tests cover retained pane identity,
virtual-row unmount/remount, confirmation cancellation, text-buffer edits,
printing preservation, and partner role save/load. Clippy and a wasm compile check
cover both client targets and the gateway's extended pool wire record.

The #195–#197 refinement adds compact quantity controls with separate main/side
counts, thumbnail-click printing selection, and a confirmed empty-deck action
inside the deck heading's overflow menu. Every row shows its type/subtypes and
larger mana symbols. Full-height thumbnails preserve the card aspect ratio.
Search offers six keyboard-selectable completions (arrows, Enter, Escape); choosing
a suggestion changes the query without adding a card. New action icons use the
existing Font Awesome asset. Muted primary buttons share one animated material
and respect reduced motion.

The entire catalog is scrollable, with native draggable scrollbar thumbs. Its
content is a single height placeholder with only visible rows plus overscan
mounted. Virtualization accounts for pending scroll offsets before layout;
returning rows receive cached image handles immediately, avoiding blank frames.
A live 1600×938 debug measurement after this refinement recorded about 75 FPS
and no idle control-identity changes. Large jumps reached the final catalog rows.

Changing language refreshes the pool while preserving the draft. Visible cards
can retrieve cached, rate-limited Scryfall translations when a gateway has no
localized catalog; responses are batched and stale-language replies discarded.
The existing type-name vocabulary supplies translated types/subtypes even when
an individual printing only supplies a translated name. Missing official card
translations retain the English name; saved card identities stay unchanged.

The printing picker now includes a five-image comparison strip, set-name filters,
language filters, left/right keyboard navigation, and a refresh action. Refresh
invalidates the selected card's metadata cache and requests current Scryfall
printings directly (including multilingual and variant editions), even when the
gateway catalog is stale. Existing metadata is retained on failure and an
unchanged selection survives successful refresh. Scryfall's default search had
omitted multilingual editions, explaining the sparse German choices.

An explicit **Enforce foil / etched** checkbox makes either cosmetic finish
available on any printing. The finish travels in the existing deck-row marker
and remains selected when the row is reopened. Animated artwork-aware materials
now render finished deck thumbnails as well as the picker and hover previews;
URL/finish material caching is bounded to 256 entries. Plain thumbnails retain
the inexpensive image path.
