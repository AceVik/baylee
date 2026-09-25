# What baylee keeps about people

A data inventory for #270 (item 11, DSGVO): what is stored, why, for how
long, and how it goes. It states what the code does, with the file and the
symbol that does it, and draws no legal conclusion; the owner draws those
from it. Read against the code on 2026-09-25. Symbols, not line numbers, are
the pointers, because line numbers move.

"Kept until deleted by hand" means no code path removes it.

## At a glance

| What | Where | Kept | Removed by |
| --- | --- | --- | --- |
| Account | Postgres `account` | indefinitely | nothing (no route; see [Open points](#open-points)) |
| Guest account | Postgres `account` (`guest`) | until its last session lapses, 29–30 days after its last request | the sweep, or signing out |
| Session | Postgres `session_token` (hash only) | 12 h (account) / 30 days (guest), sliding | the sweep, use after expiry, signing out |
| Confirmation link | Postgres `confirmation` (hash only) | 24 h valid | use, or the next resend for that account |
| Deck and its history | Postgres `deck`, `deck_version` | indefinitely | `DELETE /decks/{id}`, or the account's deletion |
| Settings | Postgres `client_settings` | indefinitely | the account's deletion |
| Uploaded sleeve or mat | disk, `BAYLEE_DECK_IMAGE_PATH`; its owners in Postgres `upload` | indefinitely | nothing (an owner row: the account's deletion) |
| Lobby tables | gateway memory | ≤ 2 h waiting, 1 h after a game ends | the lobby sweep, a restart |
| Rate-limit keys (IP, typed login name) | gateway memory | a window (300 s), then until the next check | the limiter itself |
| Game state | engine process memory | the game | the process exits |
| Server logs | stdout | the host's choice | the host |
| Legacy import file | disk, `STORE_PATH` + `.imported` | indefinitely | nothing |
| Client settings | the player's device | until the player removes them | the player; a guest's token at sign-out |

## Accounts

`crates/baylee-db/src/entity/account.rs`, written by `store::create_account`
(`crates/baylee-gateway/src/store.rs`).

- **Stored:**
  - `id`: a UUIDv7.
  - `username` and `username_key`, its lower case: the login. The entity says
    "Private: never shown to another player".
  - `display_name` and `tag`, which others see as `Name#tag`. `tag` is an
    identity column, and the entity notes it is "also a registration counter".
  - `password_hash`: an Argon2id PHC string from `argon2::Argon2::default()`
    with a random salt (`auth::hash_password`).
  - `email`: optional. Registration writes none since #269. Only accounts
    imported from the legacy file carry one.
  - `confirmed_at`, `created_at`, `lang` (the language a mail is sent in),
    `guest`.
  - No IP address, user agent or last-login time is stored anywhere.
- **Why:** the username signs in. The display name and tag are how other
  players see and find the account (`GET /players/{handle}`). The e-mail
  exists only to reach the player; while #280 is open, sign-in by address
  is also accepted.
- **Seen by others:**
  - `GET /players/{handle}` answers `id`, `display_name`, `tag` and `handle`
    to any signed-in session, guests included.
  - The lobby shows handles, never ids.
  - `GET /me` shows the owner everything but the hash.
- **Kept:** indefinitely.
- **Removed:** no route and no store function deletes a registered account.
  If a row is deleted by hand, the foreign keys cascade to its decks and
  their history, sessions, confirmations and settings
  (`crates/baylee-db/src/migration/m20260915_000001_account_side.rs`,
  `…m20260916_000002_deck_kinds_and_history.rs`). A copy another player took
  of one of its decks survives, with `copied_from` set to null.

## Guests (#269)

- **Stored:** an `account` row with `guest = true`, a display name ("Guest"
  unless one was given), a tag, `lang`, `created_at`, and one session. A
  `CHECK` keeps a guest without username, e-mail or password
  (`m20260925_000007`). A guest may keep decks and settings, and may not
  upload images (`cosmetics::upload`).
- **Kept:** `auth::GUEST_TOKEN_TTL`, 30 days, renewed at most once a day
  (`auth::Lifetime::GUEST`): 29 to 30 days after the guest's last request.
- **Removed:**
  - `store::purge_guests` (`baylee_db::guests::purge`) deletes every guest
    with no live session. It runs every 600 s in `spawn_cleanup`, after the
    session sweep.
  - `POST /auth/logout` deletes the guest at once.
  - Either way the cascade takes its decks, history and settings.
- **Limits:** `BAYLEE_GUEST_CAP` caps live guests (default 1000), and
  `BAYLEE_GUESTS=off` turns guests off.

## Sessions

`crates/baylee-db/src/entity/session_token.rs`, `auth.rs`, `store.rs`.

- **Stored:** `token_hash` (SHA-256), `account_id` and `expires_at`. The token
  itself is never stored: 256 bits from the OS generator, handed to the
  client once (`auth::new_token`). Every sign-in adds a row.
- **Kept:**
  - an account's session: `auth::TOKEN_TTL`, 12 h, renewed at most every
    6 h (`Lifetime::ACCOUNT`);
  - a guest's session: as above.
- **Removed:**
  - `store::sweep_tokens` every 600 s;
  - `store::resolve_token` when an expired one is presented;
  - `store::drop_token` at sign-out.
- **Other secrets, in memory only:** seat and engine tokens as SHA-256 hashes
  (`lobby::LobbySeat`, `lobby::LobbyGame`), and a room password as an
  unsalted SHA-256. They go with the lobby table.

## Confirmation mail

- **Stored:** `confirmation(token_hash, account_id, expires_at)`, the hash
  only. Valid `CONFIRM_TTL_SECS`, 24 h.
- **Sent:** only to an address on an unconfirmed account, and only when
  `POST /auth/confirm/resend` names it. New accounts have none, so in
  practice only imported accounts get mail. The mail carries the display name
  and the link (`mail::Mailer`). Without `BAYLEE_SMTP_URL` nothing is sent.
- **Removed:**
  - `store::take_confirmation` when the link is used;
  - `store::clear_confirmations` removes an account's expired rows, and
    runs only inside the next resend for that account. There is no periodic
    sweep, although the entity's doc says the sweep is what removes them.

## Decks

`crates/baylee-db/src/entity/deck.rs`, `deck_version.rs`.

- **Stored:**
  - name, format, `description` (free text), card rows (a row may carry a
    note of up to 500 characters, `baylee_core::deckrow`), sideboard,
    commanders;
  - the ids of the sleeve and mat images;
  - `copied_from`/`copied_version`, version and `updated_at`.
  - Each change to the lists keeps the previous lists in `deck_version`,
    with a `summary`. The migration's words: "History only ever grows, and
    no row is ever rewritten".
- **Seen by others:** the deck's name, while seated at a lobby table. Its
  cards go to the engine as card indices and printings, with no account id,
  deck name or note (`baylee_core::preset`).
- **Kept:** indefinitely.
- **Removed:** `DELETE /decks/{id}` (`store::delete_deck`), and the history
  with it by cascade. The sleeve and mat files stay (see below).

## Settings

- **Stored:** `client_settings(account_id, doc, updated_at)`. The gateway
  treats `doc` as opaque JSON of at most 16 KiB (`MAX_SETTINGS_BYTES`). The
  client stores its `Preferences` there: keymap, phase stops, automation,
  standing answers to abilities, and display and sound choices
  (`baylee_client_core::prefs`).
- **Kept:** indefinitely, overwritten on each save.
- **Removed:** only with the account.

## Uploaded sleeves and mats

`crates/baylee-gateway/src/cosmetics.rs`.

- **Stored:** a JPEG on disk under `BAYLEE_DECK_IMAGE_PATH` (default
  `deck-images`), named by the SHA-256 of its bytes. It is re-encoded from
  pixels after a crop and resize, so no metadata of the upload is copied.
  Two players uploading the same picture share one file. Who uploaded it is
  recorded, one row per player: `upload(image_id, account_id, kind,
  created_at)` (#292). The pictures uploaded before that were given to every
  account with a deck that showed them. A picture no deck showed has no
  owner.
- **Who may upload:** a registered session only, since guests are refused.
  Uploads are capped at 8 MiB.
- **Who may read:** anyone who has the id. `GET /images/{id}` asks for no
  session and is cached `public` for a year. A deck's images reach the other
  seats at its table through `GET /games/{id}/cosmetics`.
- **Removed:** never. Deleting a deck, an account or a guest leaves the file.
  An owner row goes with its account.

## Lobby tables (memory)

`crates/baylee-gateway/src/lobby.rs`.

- **Held:**
  - per seat: the account id, a hash of the seat token, the deck's name
    and a full copy of the deck, ready and team;
  - per table: the host's account id, the room name, a hash of the room
    password, and the game's preset.
- **Seen by others:** every signed-in session sees an open or running
  table's name, host, seat handles, deck names and whether it is locked.
- **Kept:** in `spawn_cleanup`:
  - a waiting table goes 2 h after it opened (`WAITING_TIMEOUT_SECS`);
  - a finished game goes 1 h after it ended (`OVER_GRACE_SECS`), unless a
    rematch room still points at it;
  - a restart loses all of it.

## Rate limits (memory)

- `AppState.limiter` (10 per 300 s) is keyed by the client's IP address,
  for registration, guest sign-in and resend. `X-Forwarded-For` is read only
  from a peer listed in `BAYLEE_TRUSTED_PROXIES` (`rate_limit_ip`).
- `AppState.sign_in_limiter` (8 per 300 s) is keyed by the account, or by
  what was typed when no account matches, which can be an e-mail address.
  A successful sign-in forgets the key.
- The art mirror's limiter is keyed by account id (`art::OUTBOUND_PER_ACCOUNT`).
- `auth::RateLimiter` prunes a key's old hits when the key is used, and drops
  idle keys in a sweep that runs inside the next check made after a window
  has passed.
- No IP address is written to the database or to a log line.

## Games (agent and engine)

- The **agent** receives a game id, a per-game engine token and the gateway
  URL (`StartEngine`), and nothing about players. It writes no files.
- The **engine** receives `GameSetup`:
  - the seats' handles (`Name#tag`, or "House AI");
  - the preset: seed, house rules, printings, and each seat's cards, team and
    controller;
  - then each seat's actions and standing answers.

  It keeps the game's journal in memory and writes nothing to disk except, in
  its development mode only, the port file. The process exits when its game
  ends, and the journal with it.
- The engine token reaches the engine on its command line
  (`baylee-agent/src/lib.rs`), which the agent's comment notes is visible in
  that machine's process list.

## Logs

- **Where:** every binary logs through `tracing_subscriber::fmt` to stdout,
  at the level `RUST_LOG` sets. There is no log file, no access log (no
  `tower-http` trace layer) and no JSON output. How long stdout is kept is up
  to whatever runs the process.
- **What is personal in them:** game ids, seat numbers, an agent's operator
  label, and counts (for example "idle guests deleted"). No log line names
  a username, e-mail, display name, account id, IP address or token.
  - The engine's development harness (`baylee-engine-server` without
    `--attach`) logs a connecting peer's address. It binds loopback by
    default and is not the hosted path.
  - `db_down` and a few sweeps log a database error's text whole (`{e:#}`).
    Whether a Postgres error can carry a stored value there was not checked.
- **Tokens:** no log line prints one. Some types would print one if they
  were ever logged with `{:?}`: `auth::IssuedToken`, `store::NewAccount` and
  `store::Account` (hash and e-mail), `baylee-agent`'s config, the engine's
  `Attach`, and the generated protobuf messages that carry a token. The
  client's `KeptGuest` and `cardtext::SignedIn` print `<token>` instead.
- **Tokens in URLs:** some secrets travel in a query string, where a reverse
  proxy's access log would see them: `/lobby/ws?token=` (the account's
  session), `/games/{id}/ws?token=` and `/games/{id}/cosmetics?token=` (a seat
  token), and `/auth/confirm?token=`. The repository ships no proxy
  configuration.

## Card data

The card catalog (`crates/baylee-catalog`) and the art cache
(`BAYLEE_ART_PATH`) hold Scryfall's card data and images, including artists'
names, and nothing about players. Requests to Scryfall send the User-Agent
`baylee/<version>` and a card or printing id, and nothing about who asked.

## The legacy import file

On first start against an empty database the gateway imports
`gateway-store.json` (`STORE_PATH`) and renames it `<path>.imported`
(`baylee_db::import::imported_name`); nothing deletes it. It holds the
imported accounts' e-mails, display names, password hashes, token hashes,
decks and settings as JSON.

## The client

- **Native** (`crates/baylee-client/src/settings.rs`): under the config
  directory (`$XDG_CONFIG_HOME/baylee` or `~/.config/baylee`), the client
  keeps:
  - `client-settings.json`: the language, the last username (for an older
    file, possibly an e-mail), the gateway list and how often each was used,
    and per gateway a kept guest's token and handle;
  - `preferences.json`;
  - `offline-decks.json`;
  - a card-text cache per language.

  Each is written `0600` on unix (`write_at`, tested). A registered
  account's session is held in memory only. A kept guest's entry goes at
  sign-out and when the gateway says the guest has ended. The art cache
  (`artreader`) holds only images, with default permissions.
- **Browser:** `localStorage` keys `baylee:client-settings` (a kept guest's
  token included), `baylee:gateway` and one per stored document. A seat
  token may arrive in the page's URL.
- **Third parties:**
  - the client fetches card images from `cards.scryfall.io` and
    `backs.scryfall.io`;
  - it asks `api.scryfall.com` for card text and printings, naming a card
    and a language;
  - Scryfall sees the device's IP address, as any host does;
  - a session is sent to the gateway it belongs to and to no other host
    (`artreader::authorization`, `cardtext::text_request`).
- **No tracking:** no cookie is set by the gateway, no analytics or
  telemetry library is linked, and the CORS policy allows no credentials.

## Open points

Found while writing this inventory. They are facts for the owner and the PM
to weigh, not conclusions.

1. **An account cannot be deleted.** No route or store function deletes a
   registered account. `docs/legal.md` §4 says "account deletion endpoint".
2. **Uploaded images:**
   - they are never deleted;
   - they are readable by anyone with the id, without a session;
   - the ones uploaded before #292 that no deck showed have no owner, so
     no account's deletion can take them.
3. **Confirmation rows:**
   - there is no periodic sweep, contrary to the entity's doc;
   - expired rows go only when the same account asks for a resend.
4. **The legacy import file** stays on disk after import, with e-mails and
   password hashes in it.
5. **Tokens in query strings** (listed under Logs) are exposed to whatever
   access log sits in front of the gateway.
6. **Retention of stdout logs and of backups** is not set anywhere in the
   repository.
