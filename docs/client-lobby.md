# Account entry, collection and deck history

The account screens reuse the table hand HUD's `FrontalMaterial`: leather,
engraved borders and five mineral inlays. `lobby/dock.rs` maintains seven bounded
material slots, one per differently sized surface. Decorative children ignore
picking. The existing motion preference also controls these materials and the
ambient shader. Native/browser text editing, selection, paste and IME continue
through the existing text-buffer and soft-keyboard paths.

Entry starts with a locally stored gateway list. Adding an HTTP(S) address does
not select it; selecting one enables sign-in and probes its registration policy.
No account request can be submitted before selection, including by keyboard.
Gateway changes discard the previous account state and art-mirror configuration.
HTTP replies carry a session generation so responses from a departed gateway or
account cannot populate the new screen. Addresses are device settings, never
account preferences; tokens and passwords are not stored with them.

The lobby separates **Play** from **Collection**. Play presents the selected deck,
table search and explicit table creation with a seat-count control. Collection
offers personal decks and house decks. The builder gives the working deck the
larger desktop pane, with the searchable card catalog beside it; narrow screens
use tabs. Quantity-weighted mana value, distinct cards, land share and opening
seven land probability complement the mana curve, colored pips and legality
feedback. Opening-hand statistics exclude the sideboard and apply before
mulligans.

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

The blue-hour ambient field is procedural. Shared HUD buttons use the same
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
