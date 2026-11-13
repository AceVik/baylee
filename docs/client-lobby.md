# Account entry, collection and deck history

## Global Daily and Weekly challenges

The native Lobby is the home directly after Central sign-in, before selecting
a gateway. Equal Daily/Weekly cards lead to task details, with the permanent
collection below. The Challenges tab also holds the archive. Both read the
shared calendar, archive, task rules, personal
progress, three independent rankings and permanent badges from the account's
existing worker. No gateway owns this state. Public leaderboard listing defaults
to off; revealing either hint requires confirmation before Central permanently
marks the task assisted. Abandonment also requires confirmation. Escape cancels
the confirmation. All controls use the shell kit and English/German Fluent words.
Its registered Tab order reaches task/ranking composites with arrow keys and
activates controls with Enter/Space. Confirmation traps Tab on Cancel/Confirm.

Task details put play and consent beside independent rankings; scoring rules
expand on request. Choosing a badge shows its five tiers, exact requirements
and earned status. Five original procedural motifs gain bezels, laurels, gems
and crowns through Bronze → Obsidian. Thirty shared material slots bound the
cache. Motion reads the GPU clock, respects reduced motion and ambient-effects
settings, and does not rebuild the UI or upload uniforms on idle frames.
Motion reduction remains adjustable before gateway admission, stored locally
by the existing preferences store. Opaque reading surfaces and scrolling over
both controls and empty space support the minimum desktop and enlarged text.

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

## Accounts and decks

The account screens reuse the table hand HUD's `FrontalMaterial`: leather,
engraved borders and five mineral inlays, which a lobby panel lifts clear of
its tooled line (`dock::INLAY_LIFT`; on the hand they sit on it), with the
panel's last row clear of them in turn. `lobby/dock.rs` maintains seven bounded
material slots, one per differently sized surface. Decorative children ignore
picking. The existing motion preference also controls these materials and the
ambient shader. Native/browser text editing, selection, paste and IME continue
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

A room says what it holds a deck to in its rules rail (`lobby/room/rules.rs`): the
list, or that every deck may sit down; a running game adds the version it froze. The
host's *Ban list…* opens a sheet of the official lists and their own, and a press
picks one (`POST …/list`); those at the table read its cards through *Read the list*.
To anyone not at the table an own list has no name, because a list is its owner's
until a table uses it. A deck a table's list turns away is told why in a sheet over
whatever screen it came from (`confirm::draw_refusal`, `Lobby::refusal_lines`), a
sentence a reason, in the same words the deck builder uses. Offline there are no
lists: they live on a gateway. The rules are `docs/banlists-and-formats.md`.

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
