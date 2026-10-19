# Private deck library and replay archive: remaining work

Status: design only, 19 October 2026. These requirements are **not implemented**
by the Central authentication/chat security milestone. The existing gateway
deck and replay features continue to work; a new device does not yet receive a
working private deck library or durable replay archive from Central.

## Current boundary

- `baylee-client/src/lobby/http.rs` sends private deck list/load/save/delete,
  history/copy/revert to the selected gateway. Central login does not intercept
  these requests. `baylee-account::LegacyData.decks` is an untyped backup, not
  the operational library.
- `baylee-db/src/central.rs` exports full owned deck rows and their historical
  versions with preferences. Its 196,608-byte / 2,048-row limit can prevent
  migration of a real account with a large history. Replays are not exported.
- `baylee-gateway/src/routes/rooms.rs` and `tables.rs::own_deck` require a stored
  gateway deck ID to create/join/configure a game. Simply moving CRUD would
  therefore break play.
- `baylee-client-core/src/replay/library.rs::EntrySource` has only local files
  and gateway/game references. `replay/host.rs` caches fetched tapes in memory;
  archiving a hosted replay changes a gateway flag, not a Central object.

Central already offers account-isolated encrypted objects with server-generated
UUIDv7 IDs, exact revisions, conditional writes/deletes and a 512-MiB / 16,384
object quota. `baylee-account` already uploads immutable 256-KiB chunks and only
publishes a chat manifest after every chunk succeeds. Reuse those primitives;
do not add server-readable deck contents, new encryption or gateway access to
the account master key.

## Deck library contract

Add a reusable `baylee-account::decks` module, independent of Bevy and HTTP
gateway implementation details. Suggested public types:

```rust
struct DeckId([u8; 16]);                 // Central-created UUIDv7 anchor
struct DeckRevision(u64);               // monotonic logical save number
struct ChunkRef { id: [u8; 16], revision: u64, bytes: u32, digest: [u8; 32] }
struct BlobRef { chunks: Vec<ChunkRef>, bytes: u64, digest: [u8; 32] }
struct DeckSnapshot {
    id: DeckId,
    revision: DeckRevision,
    name: String,
    format: String,
    cards: Vec<String>,
    sideboard: Vec<String>,
    commanders: Vec<String>,
    // bounded description, cosmetics and source provenance
}
struct DeckHead {
    id: DeckId,
    revision: DeckRevision,
    snapshot: BlobRef,
    history: BlobRef,
}
struct LegacyDeckSource { gateway: String, account: String, deck: String }
```

The encrypted account document stores a small library-root reference. The
encrypted library root holds bounded deck heads; large indexes may themselves
use the generic chunk format. Do not embed all cards/history into the account
document, and do not reuse chat-specific file paths for these objects. Each
decoded snapshot verifies its type/version, deck ID and logical revision
against its authenticated manifest reference. All strings, rows, counts,
histories, chunks and cumulative sizes need explicit checked bounds.

Suggested `Account` methods, returning typed `Error` and owned data:

```rust
fn list_decks(&mut self) -> Result<Vec<DeckSummary>, Error>;
fn load_deck(&mut self, id: DeckId) -> Result<DeckSnapshot, Error>;
fn save_deck(&mut self, id: Option<DeckId>, expected: Option<DeckRevision>,
             draft: DeckDraft) -> Result<DeckHead, Error>;
fn delete_deck(&mut self, id: DeckId, expected: DeckRevision) -> Result<(), Error>;
fn deck_history(&mut self, id: DeckId) -> Result<Vec<DeckSave>, Error>;
fn deck_version(&mut self, id: DeckId, revision: DeckRevision)
    -> Result<DeckSnapshot, Error>;
fn copy_deck(&mut self, source: DeckSnapshot, name: String)
    -> Result<DeckHead, Error>;
fn revert_deck(&mut self, id: DeckId, expected: DeckRevision, source: DeckRevision)
    -> Result<DeckHead, Error>;
```

`copy_deck` allocates a new ID and records source provenance; a public house deck
is first fetched from the gateway and converted to the same validated draft.
`revert_deck` makes a new save, retaining the prior head and history. Deletion
publishes a tombstone before deleting unreferenced chunks. Version conflicts
are reported, never resolved by silently overwriting. Upload failure, revocation
or quota exhaustion leaves the previous root reachable. Publish the library
root through the same refreshed account-document CAS used by preferences/chat
so unrelated sections cannot be overwritten. Retry ambiguous publication by
reading its exact generation before deleting or re-uploading anything.

### Legacy conversion and cutover

Parse the existing backup into a strict, versioned legacy DTO. Preserve all
current fields, printing/choice/notes strings, historical card lists, version
numbers, descriptions and provenance. Keep `(canonical gateway origin, legacy
account ID, legacy deck ID)` as the idempotency key; never assume two gateways'
deck IDs share ownership. Gateway image/upload IDs are origin-scoped, not
portable Central assets.

Convert into new immutable objects, verify their reads, and atomically publish
the complete root plus migration marker. Until that succeeds, preserve the
legacy backup/source and report migration as incomplete. Repeating the process
must not duplicate decks or lose history. Large source accounts require a
bounded paginated/chunked export bound to one source generation/digest; the
current whole-document limit must not be worked around by dropping old saves.
Final linking still checks source credential generation and the complete source
digest under the existing mutation barrier. A changed source requires refresh.

### Gameplay handoff

For Central accounts the client resolves the selected private deck and sends
only that deck snapshot with create/join/set-seat. Add an explicit versioned
request choice: a private submitted snapshot **or** an existing gateway deck
reference, never both. Existing reference requests remain for compatible legacy
clients and gateway-owned shared decks. Do not persist a second private library
copy on the gateway merely to obtain an ID.

The gateway validates submitted snapshots with its existing row/count/commander
checks in `routes/deck_lines.rs`, catalog resolution, blocked-card policy and
origin-local cosmetics ownership. Recheck policy when the room starts, as
`room_preset` already does. Store the chosen snapshot only in room/game state;
the engine must receive the same validated preset as today. Client-side checks
are helpful feedback but cannot authorize gameplay or another account's assets.

Intercept private library operations in native `lobby/http.rs::dispatch`, submit
them to the existing bounded Central account worker, and translate typed results
into current `LobbyEvent`/library replies. Keep shared catalog, card legality,
available printings and gateway block lists on the gateway. Keep plaintext and
keys out of diagnostics, account snapshots and dev-control output.

## Replay archive: separate implementation

Add a separately typed account replay index and immutable blob content. An entry
binds format/version, source origin/game, authorized perspective/seat, content
digest/length and private metadata such as archive status. Never persist
`HostedAccess.token`, an active gateway lease, or a credential-bearing URL.

Use the existing **authorized tape** response for hosted games. Its route first
verifies that the account sat at that game and projects only permitted views.
The unrestricted raw hosted record contains every player's hidden information;
putting that into every participant's archive would defeat this boundary.
Local solo records can retain their existing full-record format. Training data
is a distinct permission/processing pipeline, not an exception to replay access.

After a game completes, fetch and validate the allowed tape, upload immutable
chunks, then publish the manifest. Import pre-existing hosted games through a
bounded, resumable discovery cursor. Deduplicate on canonical origin, game,
perspective/seat and content version. Private archive/favourite/label state
belongs to Central; gateway sharing consent remains gateway authority. An
already downloaded authorized tape cannot be recalled by later changing that
consent, which must be reflected in the product's sharing semantics.

Add `EntrySource::Central` and open the stored record/tape after authenticated
chunk validation even when the original gateway is offline. The current
32-MiB network-response and 256-MiB decoded replay limits must apply to restored
data too, with separate cumulative account quota and bounded worker queues.
Malformed/missing/over-limit blobs fail closed without deleting the source or
publishing partial manifests. An upload failure is a visible pending archive,
not an apparently successful sync.

## Required acceptance tests and proposed split

1. **Account module:** typed CRUD/history/copy/revert, old-state preservation on
   failed chunk/CAS/quota operations, tombstones, cross-account object rejection,
   corrupt/wrong-ID/wrong-revision references and non-duplicating legacy retries.
   Cover an account whose source exceeds the existing 192-KiB migration limit.
2. **Gateway:** create/join/set-seat with a submitted deck works without a private
   deck DB row; conflicting forms, malformed counts, invalid commanders,
   blocked cards and unowned cosmetics fail. A policy change before room start
   must still reject the deck. Existing reference clients continue to work.
3. **Client:** no private deck CRUD HTTP call reaches a gateway after Central
   cutover; copy from the shared catalog persists centrally; switching gateway
   preserves the library. Logout/login on a fresh device restores names, cards,
   printing choices, history and the selected deck without a stale snapshot.
4. **Replays:** a fresh device opens its Central archive while the gateway is
   unavailable; the archive contains only the permitted perspective; revoked
   sessions cannot finish publication; missing chunks, gzip bombs, truncated
   manifests and repeated discovery cannot yield partial or duplicate entries.

Implement deck Account API/legacy DTO, gateway gameplay-snapshot admission and
client routing in parallel only after the typed wire contract is agreed. Replay
archive migration is a following slice, using the same generic immutable blob
helper. A milestone must not be described as full account-data centralization
until these end-to-end tests pass.
