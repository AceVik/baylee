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
