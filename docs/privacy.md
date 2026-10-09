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
| Account | Postgres `account` | until its player deletes it | `DELETE /account` |
| Guest account | Postgres `account` (`guest`) | until its last session lapses, 29–30 days after its last request | the sweep, or signing out |
| Session | Postgres `session_token` (hash only) | 12 h (account) / 30 days (guest), sliding | the sweep, use after expiry, signing out |
| Closed beta key (#317) | Postgres `invite` (hash only), and which key admitted an account (`account.invite_id`) | until the operator removes the row | SQL by the operator; revoking only closes it |
| Admin console numbers (counts only) | nowhere: counted per request | — | — |
| Confirmation link | Postgres `confirmation` (hash only) | 24 h valid | use, the next resend for that account, or the sweep once expired |
| Deck and its history | Postgres `deck`, `deck_version` | indefinitely | `DELETE /decks/{id}`, or the account's deletion |
| Settings | Postgres `client_settings` | indefinitely | the account's deletion |
| Which terms of use an account accepted, and when | Postgres `account` (`terms_version`, `terms_accepted_at`) | until the next acceptance replaces it | the account's deletion |
| Uploaded sleeve or mat | disk, `BAYLEE_DECK_IMAGE_PATH`; its owners in Postgres `upload` | while an account claims it or a deck shows it | the deletion of the last account that claims it, or the sweep at the gateway's next start |
| Lobby tables | gateway memory | ≤ 2 h waiting, 1 h after a game ends | the lobby sweep, a restart |
| Who has a lobby socket open (account ids, for a count) | gateway memory | while the socket is open | the socket closing, a restart |
| Socket ticket (hash only) | gateway memory | ≤ 45 s (`BAYLEE_WS_TICKET_SECS`), or until used | use, the ticket sweep, a restart |
| Chair ticket (hash only), with the room, chair and host account it is for | gateway memory | ≤ 120 s (`BAYLEE_CHAIR_TICKET_SECS`), or until used | use, its host leaving the room or being deleted, the ticket sweep, a restart |
| Rate-limit keys (IP, typed login name) | gateway memory | a window (300 s), then until the next check | the limiter itself |
| Game state | engine process memory | the game | the process exits |
| Server logs | stdout | the host's choice | the host |
| Legacy import file | disk, `STORE_PATH` + `.imported` | indefinitely | nothing |
| Client settings | the player's device | until the player removes them | the player; a guest's token at sign-out |
| A report the player sends | leaves the device for the gateway (`POST /reports`), or, signed in nowhere, for the feedback service (`POST /client/reports`) | see [Reports](#reports-and-crash-reports-309-310-314) | — |
| Record of a game the client hosted (against the house) | the player's device, `records/` beside the settings (native only) | the last 20 games, at most 64 MiB together | the client, oldest first; the player |
| Device id for direct reports (random) | the player's device, `client-settings.json` (`report_device`) | until the player removes it | the player |
| Which terms version this device accepted at each gateway (a copy) | the player's device, `client-settings.json` (`terms`, by gateway address) | until the next acceptance there replaces it | the player |
| Game record | Postgres, written by the gateway (#315) | without a time limit | nothing |
| Crash file | the player's device, `crash-report.json` | until the next start sends or discards it | the client |
| Update check (#326) | leaves a desktop client for `api.github.com` and GitHub's download hosts | GitHub's own terms | switching "Check for updates automatically" off |
| Update choice and payloads | the player's device: `update.json`; per-user updater state (paths below) | choices until changed; active and previous payload retained; older payloads removed on a safe next launch | the player; the launcher when no runtime is using them |
| Where the player was, across "Restart now" (no secret) | the player's device, `resume.json` beside the settings (desktop only) | until the next start reads it; ignored after ten minutes | the client, at the next start |

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
  - `invite_id`: on a closed beta (#317), which key let the account in
    (below); empty otherwise.
  - `terms_version` and `terms_accepted_at`: on a gateway with terms of use
    (`BAYLEE_TERMS_PATH`, WG-1), which version the account last accepted
    and when (`POST /account/terms`); empty until it does, and on a gateway
    without terms. Only the last acceptance is kept, and not the language
    the terms were read in (the version is one for all of them). It goes
    with the account.
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
- **Kept:** until its player deletes it.
- **Removed:** `DELETE /account` (`account::delete_account`), with the
  password asked again (a guest's session is enough). In the client it is
  "Delete account" on the settings screen (#292). One statement,
  `baylee_db::accounts::delete`, deletes the row, and the foreign keys
  cascade to its decks and their history, sessions, confirmations, settings
  and picture claims
  (`crates/baylee-db/src/migration/m20260915_000001_account_side.rs`,
  `…m20260916_000002_deck_kinds_and_history.rs`,
  `…m20260925_000008_upload_owners.rs`). Then the gateway
  removes the pictures nobody claims any more (see below) and takes the
  account out of every lobby table and socket (`account::forget_accounts`).

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
  - `DELETE /account`, with the guest's session.
  - Every way, the cascade takes its decks, history and settings, and the
    gateway lets go of it as of a deleted account (above).
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

## Closed beta keys (#317)

`crates/baylee-db/src/invites.rs`, the migration
`m20260927_000010_invites`, and the command `baylee-gateway invite`
(`crates/baylee-gateway/src/invite.rs`).

- **Stored:** `invite(id, key_hash, created_at, note, uses_left,
  expires_at, revoked_at)`. `key_hash` is SHA-256 of the key's canonical
  form; the key itself is printed once, by the command that makes it, and
  never stored or logged. `note` is the operator's own word on whom a key
  was for, at most 100 characters: it may name a person, so write what you
  need and no more.
- **Linked:** an account made with a key keeps its id (`account.invite_id`),
  so `invite list` can say how many accounts each key admitted. Nothing
  about the player is written to the key's row.
- **Seen by:** the operator, through `invite list`, SQL, or the admin
  console (`GET /admin/invites` on its loopback listener, shown to the
  feedback service's signed-in admins, below). No public route shows a
  key's row, and no player sees another's `invite_id`. The console shows a
  key only in the answer that made it, as the command prints it once.
- **Kept:** until the operator removes the row. `invite revoke` only closes
  a key; there is no delete command, so removal is SQL
  (`DELETE FROM invite WHERE id = …`). Removing a key leaves its accounts
  (`ON DELETE SET NULL`), and deleting an account leaves its key's row.
- **Key tries** count in `AppState.sign_in_limiter` under the caller's
  address (below).

## The admin console's numbers

`GET /admin/stats` (`docs/protocol.md` §"The admin console"), on a loopback
listener of the gateway's own and shown to the feedback service's signed-in
admins on its Overview.

- **What it reveals:** counts and sums only: accounts (all, with an
  address, confirmed, admitted by a key, made today and in the last 7 and
  30 days), guests, live sessions and the accounts they belong to, players
  with a lobby socket open or at a table, games running, waiting, started
  and finished, agents and their capacity, and keys by state. No list of
  users, no name, username, address, account id, game id or IP; the
  gateway's e2e test holds the answer free of the names it registered.
- **What a count can still tell:** on a small gateway a count is close to
  a person. "1 player online" or "1 account made today" says that somebody
  is; the admins who see it are the operator's own, signed in at the
  feedback service.
- **Kept:** nowhere. The numbers are counted from the tables and memory
  above at each request; neither the gateway nor the feedback service
  stores them. The service's audit keeps who made or revoked which key id
  through the console, and when (`docs/feedback.md` §"What is kept").
- **Logs:** the gateway logs each change made through the console on
  `baylee_gateway::audit`: the admin's name (an operator's, not a
  player's), the action and key ids; never a key, the token, or a key's
  note.

## Confirmation mail

- **Stored:** `confirmation(token_hash, account_id, expires_at)`, the hash
  only. Valid `CONFIRM_TTL_SECS`, 24 h.
- **Sent:** only to an address on an unconfirmed account, and only when
  `POST /auth/confirm/resend` names it. New accounts have none, so in
  practice only imported accounts get mail. The mail carries the display name
  and the link (`mail::Mailer`). Without `BAYLEE_SMTP_URL` nothing is sent.
- **Removed:**
  - `store::take_confirmation` when the link is used;
  - `store::clear_confirmations` removes all of an account's links when it
    asks for a new one;
  - `baylee_db::confirmations::sweep` removes every expired link, every
    600 s, beside the session sweep (#293);
  - the account's deletion, by cascade.

## Decks

`crates/baylee-db/src/entity/deck.rs`, `deck_version.rs`.

- **Stored:**
  - name, format, `description` (free text), card rows (a row may carry a
    note of up to 500 characters, `baylee_core::deckrow`), sideboard,
    commanders;
  - the ids of the sleeve and mat images, only ones the player uploaded
    (#292);
  - `copied_from`/`copied_version`, version and `updated_at`.
  - Each change to the lists keeps the previous lists in `deck_version`,
    with a `summary`. The migration's words: "History only ever grows, and
    no row is ever rewritten".
- **Seen by others:** the deck's name, while seated at a lobby table. Its
  cards go to the engine as card indices and printings, with no account id,
  deck name or note (`baylee_core::preset`).
- **Kept:** indefinitely.
- **Removed:** `DELETE /decks/{id}` (`store::delete_deck`), and the history
  with it by cascade. The sleeve and mat files stay while an account claims
  them (see below).

## Settings

- **Stored:** `client_settings(account_id, doc, updated_at)`. The gateway
  treats `doc` as opaque JSON of at most 16 KiB (`MAX_SETTINGS_BYTES`). The
  client stores its `Preferences` there: keymap, phase stops, automation,
  standing answers to abilities, and display and sound choices
  (`baylee_client_core::prefs`).
- **Kept:** indefinitely. Each save merges into it per top-level key (a
  key the save names is replaced, `null` removes one, the rest stay), so a
  key is gone only when a client removes it or with the account.
- **Removed:** only with the account.

## Uploaded sleeves and mats

`crates/baylee-gateway/src/cosmetics.rs`.

- **Stored:** a JPEG on disk under `BAYLEE_DECK_IMAGE_PATH` (default
  `deck-images`), named by the SHA-256 of its bytes. It is re-encoded from
  pixels after a crop and resize, so no metadata of the upload is copied.
  Two players uploading the same picture share one file. Who uploaded it is
  recorded, one row per player: `upload(image_id, account_id, kind,
  created_at)` (#292). The pictures uploaded before that were given to every
  account with a deck that showed them. A picture no deck showed had no
  owner, and the sweep below removed it.
- **Who may upload:** a registered session only, since guests are refused.
  Uploads are capped at 8 MiB.
- **Who may read:** anyone who has the id. `GET /images/{id}` asks for no
  session and is cached `public` for a year. A deck's images reach the other
  seats at its table through `GET /games/{id}/cosmetics`.
- **Removed:**
  - when the last account claiming it is deleted
    (`account::forget_pictures`): the file, and any deck's mention of it.
    Deleting a deck leaves the file to the account that uploaded it;
  - as the gateway starts, every picture with no owner that no deck shows
    (`account::sweep_pictures`, #301), under the same lock as an upload and
    a deletion. That takes the ones uploaded before #292 that no deck
    showed, and the files of a deletion the gateway stopped in the middle
    of. It logs how many went, not which.

## Lobby tables (memory)

`crates/baylee-gateway/src/lobby.rs`.

- **Held:**
  - per seat: the account id, a hash of the seat token, the deck's name
    and a full copy of the deck, ready and team; for a host's seat bridge
    (`docs/protocol.md` §"A host's chair for a seat bridge") instead of an
    account id the host's account id and the name the bridge sat under,
    and its deck only here, never as a deck row;
  - per table: the host's account id, the room name, a hash of the room
    password, and the game's preset.
- **Seen by others:** every signed-in session sees an open or running
  table's name, host, seat handles, deck names and whether it is locked;
  for a seat bridge's chair its name and the handle of the host it sits
  for (`delegated_by`).
- **Kept:** in `spawn_cleanup`:
  - a waiting table goes 2 h after it opened (`WAITING_TIMEOUT_SECS`);
  - a finished game goes 1 h after it ended (`OVER_GRACE_SECS`), unless a
    rematch room still points at it;
  - a restart loses all of it.
- **Removed early:** a deleted account's chairs are emptied at every table,
  waiting, running or over (`Lobby::forget_account`), and so are the
  chairs its seat bridges sat in; its seat and lobby sockets close. A
  running game plays on with the house in that chair. A host who leaves a
  waiting room takes its bridges' chairs with it.
- **Counted, not shown:** `GET /lobby/stats` (WG-0) answers a signed-in
  session three numbers — players online, tables waiting, games running —
  and no ids. "Online" is the accounts with a lobby socket open (held in
  memory per account while the socket is, `presence.rs`) and those in a
  chair of a running game; it is never stored or logged.

## Rate limits (memory)

- `AppState.limiter` (10 per 300 s) is keyed by the client's IP address,
  for registration, guest sign-in and resend. `X-Forwarded-For` is read only
  from a peer listed in `BAYLEE_TRUSTED_PROXIES` (`rate_limit_ip`).
- `AppState.sign_in_limiter` (8 per 300 s) is keyed by the account, or by
  what was typed when no account matches, which can be an e-mail address.
  A successful sign-in forgets the key, and so does an account's deletion.
  On a closed beta it also counts key tries, keyed `invite:{ip}`, forgotten
  when a key admits somebody.
- The art mirror's limiter is keyed by account id (`art::OUTBOUND_PER_ACCOUNT`).
- `auth::RateLimiter` prunes a key's old hits when the key is used, and drops
  idle keys in a sweep that runs inside the next check made after a window
  has passed.
- No IP address is written to the database or to a log line.

## Games (agent and engine)

- The **agent** receives a game id, a per-game engine token and the gateway
  URL (`StartEngine`), and nothing about players. It writes no files.
- The **engine** receives `GameSetup`:
  - the seats' handles (`Name#tag`, or "House AI"), for the whole game,
    also a player's who deletes their account during it;
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
  label, counts (for example "idle guests deleted"), and the id of a picture
  whose file could not be removed. No log line names
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
- **Tokens in URLs (#294):** a socket is opened with a ticket
  (`?ticket=`), bought over `POST /ws-ticket` with the session or seat token
  in `Authorization`, and the cosmetics route takes the seat token in
  `Authorization` too. A ticket in a proxy's access log is spent or expired
  within seconds. Until 2026-10-31 (UTC) the gateway still accepts the old
  `?token=` on `/lobby/ws`, `/games/{id}/ws` and `/games/{id}/cosmetics` from
  clients that have not updated (`BAYLEE_WS_LEGACY_TOKENS=off` ends that
  earlier), and logs the route, never the token, each time. What still
  carries a secret in its address: `/auth/confirm?token=` (a mailed link) and
  the browser client's handover page, `?game=…&token=…` (a page address the
  gateway never sees, `docs/protocol.md` §"Opening a socket: tickets"). The
  one proxy configuration the repository ships, `scripts/server/play.caddy`
  (the browser client at `/play/`, #327), writes no access log, and a test
  keeps it that way, because the handover page's address can carry a seat
  token.
- **Socket tickets, in memory only:** the gateway keeps each ticket as a
  SHA-256 hash with what bought it (an account id and its session's hash, or a
  game id, seat and the seat token's hash) and when, until it is used or
  `BAYLEE_WS_TICKET_SECS` (45 s by default) have passed; a sweep every minute
  removes expired ones, and a restart all of them.
- **Chair tickets, in memory only** (`chair.rs`): a SHA-256 hash, the room,
  the chair and the host's account id, and when, until redeemed, its host
  leaves the room or is deleted, or `BAYLEE_CHAIR_TICKET_SECS` (120 s by
  default) have passed. The host's client hands the ticket to its bridge on
  the bridge's stdin, never in its arguments or environment, and neither
  side logs it. Tries are counted per host account and per address
  (`AppState.chair_limiter`, 30 a minute each).

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
  directory (`$XDG_CONFIG_HOME/baylee` when set; else `%APPDATA%\Baylee` on
  Windows and `~/.config/baylee` elsewhere, `client-core::userdirs`), the
  client keeps:
  - `client-settings.json`: the language, the last username (for an older
    file, possibly an e-mail), the gateway list and how often each was used,
    per gateway a kept guest's token and handle, and what reports may carry
    (#309);
  - `crash-report.json`, after a crash, until the next start deals with it
    (#310);
  - `records/game-<start>-<seed>.jsonl.gz`: the record of each game this
    client hosted itself (against the house): every input from the shuffle
    on, so every seat's cards, hidden ones included, and no name. The last
    20, at most 64 MiB together, oldest deleted first
    (`bugreport::retention`). Written step by step as the game is played,
    and compacted when it ends or is left, so a crash leaves the game up
    to the step it happened in. They
    never leave the machine on their own (below);
  - in `client-settings.json`, `report_device`: a random id (32 hex digits)
    made for the first report sent straight to the feedback service, and
    `feedback_url`, that service's address when the player set one;
  - in `client-settings.json`, `terms`: per gateway address, the version of
    its terms of use this device last accepted there (WG-1). A copy with one
    job: a guest coming back with its kept session signs in to nothing, so
    the gateway says nothing about its terms; the client holds `/info`'s
    version against this one and asks again only when they differ. The
    account's own record (version and time) is the gateway's, above; the
    terms sheet's note says both;
  - `resume.json`, only between "Restart now" (a ready update, desktop
    only) and the next start, which reads and deletes it whatever it
    holds; ignored when older than ten minutes or not handed over by that
    restart (`client-core::resume`, `docs/client.md` §"Restarting into an
    update"). It says where the player was: the chosen gateway's address,
    the username (or that it was a guest), the lobby's screen, the hub tab,
    the open settings section, the id of the deck open in the builder, a
    waiting room's game id, a hosted game's id and seat number, or a house
    game's record file name, seat names and engine hash, and how the table
    was being looked at (arrangement, visited seat, drawer, fold, zone
    browser). **No secret**: no session, seat token or password. The
    session crosses the restart only through pipes (relaunch helper,
    launcher, new client's stdin), never a file, an argument, the
    environment or a log, and a hosted chair's new ticket is asked of the
    gateway by that session;
  - `preferences.json`;
  - `offline-decks.json`;
  - a card-text cache per language.

  Each is written `0600` on unix (`write_at`, tested). A registered
  account's session is held in memory only. A kept guest's entry goes at
  sign-out and when the gateway says the guest has ended. The art cache
  (`artreader`) holds only images, with default permissions.
- **Browser:** `localStorage` keys `baylee:client-settings` (a kept guest's
  token included), `baylee:gateway` and one per stored document
  (`baylee:preferences.json`, `baylee:offline-decks.json`,
  `baylee:card-text-<lang>.json`), for the origin the page came from, until the player clears the
  site's data; a registered account's session stays in memory, as natively.
  The browser's HTTP cache also keeps the client's files and the card images
  it fetched. A seat token may arrive in the page's URL. The hosted page
  (`https://baylee.acevik.de/play/`, #327) sets no cookie and its policy
  lets it talk only to its own gateway and the three Scryfall hosts below.
- **Third parties:**
  - the client fetches card images from `cards.scryfall.io` and
    `backs.scryfall.io`;
  - it asks `api.scryfall.com` for card text and printings, naming a card
    and a language;
  - Scryfall sees the device's IP address, as any host does; in a browser
    every card image comes from Scryfall and never from the gateway's
    mirror (`docs/protocol.md` §"Card art"), so playing at `/play/` shows
    Scryfall the player's IP address straight from their browser; the page
    sends `Referrer-Policy: no-referrer`, so not which page asked;
  - a session is sent to the gateway it belongs to and to no other host
    (`artreader::authorization`, `cardtext::text_request`).
  - **Updates (#326, desktop only):** at start and every six hours the
    client asks `api.github.com` for the list of releases, without an
    account or any identifier of the player: the request carries
    `User-Agent: Baylee/<version>` and the `ETag` of GitHub's previous
    answer, nothing else. When a newer release exists it downloads the
    archive, `.sig` and `.sha256` from GitHub (`github.com`, which redirects
    to its download hosts). GitHub sees the device's IP address, the
    version and when it asks. The switch "Check for updates automatically"
    (`update.json` beside the settings, per device, default on) turns every
    such request off; "Check for updates" then asks once. "Update
    automatically" (default on) decides only whether a download is
    downloaded and installed. Updater state is under
    `$XDG_STATE_HOME/baylee` (or `$HOME/.local/state/baylee`) on Unix and
    `$LOCALAPPDATA/baylee` on Windows, in a directory keyed by a hash of the
    canonical installed launch path. It holds the staged download, complete
    versioned payloads, activation records, and local process-coordination
    files. The selected payload and its predecessor remain; older payloads
    are removed at a subsequent launch only when no runtime can use them.
    Interrupted recovery keeps its records and required files until it can
    finish. The original package stays untouched. None of this local state
    is uploaded; a browser or phone build makes no updater request.
- **No tracking:** no cookie is set by the gateway, no analytics or
  telemetry library is linked, and the CORS policy allows no credentials.

## Game records and reports (#315, #307, #308)

Kept apart from the table above so the two strands' rows merge cleanly.

| What | Where | Kept | Removed by |
| --- | --- | --- | --- |
| Game record (every input, all hands and libraries) | gateway Postgres `game_record`, `game_record_chunk` | indefinitely (the owner's decision) | nothing yet |
| Who sat in which seat of a recorded game | gateway Postgres `game_record_seat` (account id) | until the account is deleted | the account's deletion sets it to `NULL`; the record stays |
| Whose seat bridge played a seat of a recorded game | gateway Postgres `game_record_seat.delegated_by` (the host's account id; `account_id` stays `NULL`) | until that account is deleted | the account's deletion sets it to `NULL`; the record stays |
| Report count per account | gateway memory | an hour | the limiter itself; a report the service did not take is not counted |
| Direct reports per address and in all | the feedback service's memory | an hour, a restart forgets them | the allowance itself; never written down |
| Direct report (kind, text, build, the `client` object, a client's record), `channel = 'direct'`, gateway `(direct)` | the feedback service's `feedback_report` | until an admin deletes it | `DELETE /reports/{id}` on the service |
| A client's record of a game it hosted (`record_origin = 'client'`, unverified) | `feedback_report.record` | with the report | with the report |
| Report (kind, text, game id, the client's `client` object, the record of a game its reporter sat at) | the feedback service's own Postgres, `feedback_report` | until an admin deletes it | `DELETE /reports/{id}` on the service |
| Reporter pseudonym | `feedback_report.reporter` | with the report | with the report |
| Feedback admin (name, Argon2id password hash, since when) | the feedback service's `feedback_admin` | until removed | `baylee-feedback admin remove`, on the server only |
| Feedback admin session (SHA-256 of the cookie token, sign-in and last-use times) | `feedback_session` | 12 hours unused, at most 7 days | sign-out, lapse (deleted when next shown and at every sign-in), or the admin's removal |
| Triage audit (time, admin name or `token`, report id, action, its detail) | `feedback_audit` | indefinitely, also after the report is deleted (#311) | nothing yet |
| Failed feedback sign-ins per address and per name | the service's memory | 15 minutes, a restart forgets them | the limiter itself; never written down |

- **The record names nobody.** Seats are numbers; the seat names the engine
  was told never enter it. The only link to a person is `game_record_seat`
  (who played a seat, or whose seat bridge did), which goes with the
  account. What stays after that is a game between
  numbered seats, with their decks and every card they held.
- **No seat or lobby route reads a record.** The one reader a route
  reaches is `POST /reports`, which attaches a game's record only when the
  reporter sat at that game (`docs/protocol.md` §"The game record"). The
  other is the operator's export below, which no route reaches.
- **Training and balancing use only the anonymised export.** The house AI's
  training and the balancing of decks read game records only as
  `baylee-gateway records export --out <dir>` writes them
  (`crates/baylee-gateway/src/recordexport.rs`), never the database. The
  export leaves out the game's id (the join to `game_record_seat` and to a
  feedback report, and a time, being a UUIDv7) and `game_record_seat`
  altogether; sets every line's host time (`at`) to 0; sets a `Human`
  seat's account number to 0; names each file by a count
  (`000001.jsonl.gz`), with no index of ids or times beside it; refuses a
  record with a line of a kind it does not know; and refuses a record in
  which the id, email, username, display name or invite key of an account
  seated at the game (or whose seat bridge played there) occurs, counting
  refusals without naming them. What stays is the game: the build, the
  preset (format, seed, seats by number with their teams, decks and
  printings, the house's profiles), every input with its seat and who
  answered it (a player, the clock, the house, a stand-in), chair changes,
  each seat's declared mind (the model a seat said it was) and the end.
  Names shorter than three characters are not searched for. A deck list or
  a rare printing can still be recognised by someone who knows the deck;
  the export does not hide which cards were played. Opting a player out of
  training is not built (`tools/trainer/HANDOVER.md`).
- **What reaches the feedback service** is `docs/feedback.md`'s list: the
  gateway's name, public URL and build, the pseudonym (HMAC-SHA256 of the
  account id under `BAYLEE_FEEDBACK_KEY`, which the service does not hold),
  and what the player sent. Never a name, username, address, session or IP.
  The service has no column for an address or a name and logs no request's
  address. A report sent straight from a client carries no account at all:
  its reporter is HMAC-SHA256 of the client's random device id under the
  service's `FEEDBACK_DIRECT_KEY`, and the address it came from is counted
  in memory for an hour and kept nowhere.
- **The feedback web UI (#311)** is for the service's admins, who are
  developers, not players. Its one cookie (`__Host-baylee-feedback`,
  `HttpOnly; Secure; SameSite=Strict`) holds the session token and is sent
  only to the service's own origin; the page loads nothing from anywhere
  else (no CDN, font or telemetry) and opens GitHub only when an admin
  presses "Open a new issue on GitHub", and then with nothing of the report
  (owner, 08.10.2026): only a neutral technical summary the admin wrote
  themselves (the field asks for no personal data and no quotes from the
  report), a neutral category and the build. Not the player's text, not
  the pseudonym, not the client's details, and no link back to the report
  or the service; the admin reads it over on GitHub before submitting. The
  audit names the admin who changed a report; it names no player. Its
  Overview shows a gateway's numbers (counts only, above) and its keys.
- **Open:** records have no deletion path yet, not even for a player who
  deletes their account; their account link is cut, the game stays.

## Reports and crash reports (#309, #310, #314)

The client's half; what the gateway and the feedback service do with a
report once it has it is theirs to describe.

- **When:** only when the player presses Send in the report form (F8, the
  table's game menu, the lobby's gear menu or settings), and, for a crash,
  on the next start once the player has said yes to crash reports.
- **Where to:** `POST {gateway}/reports` on the gateway the lobby is signed
  in to, with that session's bearer. Signed in nowhere, `POST
  {service}/client/reports` on the feedback service the settings
  (`feedback_url`) or the build (`BAYLEE_FEEDBACK_PUBLIC_URL`) name, with
  no credential and the device's random id; knowing none, the form says so
  and nothing is sent, to the gateway, Scryfall or anyone else. Before a
  direct report goes, and before any report carrying a game's record, the
  form shows a page listing where it goes and every part it carries, and
  only its "Send now" sends.
  A crash report goes to the gateway the client was signed in to when it
  crashed, or, if none, to the live gateway every build knows
  (`gateway_list::PINNED`), and only once signed in there.
- **What, always:** the kind, the text the player wrote, this client's
  version and commit, and the game's id at the gateway when the report is
  written at a networked table. The gateway adds its own record of that
  game (#315).
- **What, if ticked** (one box each, `bugreport::Category`, all off until
  the player ticks them):
  - *System and hardware:* platform (OS/architecture), logical CPU count,
    graphics adapter and backend, window size and scale, interface
    language.
  - *The table as you see it:* the seat's `PlayerView` (which holds nothing
    hidden from that seat and names no player), the open question, the
    half-built answer, the turn and step.
  - *Your game log:* the seat's own log, written in English with the
    reporter as "You" and every other seat as "Player A", "Player B", …
    in seat order (`bugreport::seat_log`); no display name or handle is
    in it.
  - *Settings:* language, preview size, text view, zone view, music level,
    the number (not the addresses) of saved gateways, and the account's
    preferences (key bindings, standing answers, automation). Never the
    username, a gateway address or a token.
  - *Screenshot:* a PNG of the window as it was when the form opened, at
    most 1280 pixels wide. It shows whatever was on screen, which can
    include other players' names; the box says so. Native builds only.
- **The record of a game hosted here**, only when the player ticks
  "Attach the whole record of this game" for this one report: the box
  starts unticked at every opening of the form and the yes is kept
  nowhere. Its sentence says what it is: every move from the shuffle on,
  so every seat's cards, hidden ones too (hands, libraries, face-down
  cards), and no name. "Never offer to attach a game's record" is the one
  standing answer (`RecordConsent::Never`, per device). Sent gzipped, at
  most 4 MiB (a larger one stays home and the report goes without it); the
  gateway and the service keep it marked as the client's, unverified. A
  report at a networked table never carries one: that game's record is the
  gateway's.
- **The device id**, on a direct report only: 32 random hex digits made
  once, never derived from the machine, an account or an address, never
  sent to a gateway.
- **What, never:** a session, seat or guest token. `bugreport::seal` looks
  for every token the client holds in the serialised body, and in an
  attached record's own lines, and refuses to send a body that contains
  one, whichever field it got into.
- **Crash reports:** the panic hook writes `crash-report.json` next to the
  settings (the panic message and location, the thread's name, the time,
  the build, the backtrace, and on disk only the gateway it was signed in
  to), with the home directory written `~`. No network in the hook. The
  backtrace is the list of functions the program was in when it stopped,
  each with the source file and line it was compiled from (a dependency's
  under `~/.cargo`); it names code, not the player or the game, and is cut
  to its first 16 000 characters (`bugreport::BACKTRACE_CHARS`, the frames
  nearest the crash kept). The next start asks
  once (`CrashConsent::Unasked`); "Send crash reports" sends each crash
  from then on, "Don't send" deletes the file and asks no more. A crash
  report carries the error, its location, the thread, the time, the
  backtrace and the build, and the system details only if that box is
  ticked. The file is deleted once the gateway has it (or has
  refused it for good); unanswered, it waits for the next start.
- **Consent is per device** (`client-settings.json`, `reports`), shown and
  changed in the form itself: un-ticking a box, or the crash box, is the
  revocation, and holds from the next report on.
- **Game records (#315):** the gateway keeps each game's full record in
  its database without a time limit; the record numbers its seats and
  names nobody (see "Game records and reports" above).

## Open points

Found while writing this inventory. They are facts for the owner and the PM
to weigh, not conclusions.

1. **Uploaded images:**
   - they are readable by anyone with the id, without a session, and cached
     `public` for a year, so a copy can outlive the file's removal;
   - a gateway that stops between an account's deletion and the removal of
     its pictures leaves those files behind with no owner until it starts
     again: the deletion commits first, and the start's sweep takes them.
2. **The legacy import file** stays on disk after import, with e-mails and
   password hashes in it.
3. **Tokens in query strings** (listed under Logs): until 2026-10-31 an old
   client still sends its session or seat token in a socket's address, and
   `/auth/confirm?token=` always does; whatever access log sits in front of
   the gateway sees those.
4. **Retention of stdout logs and of backups** is not set anywhere in the
   repository.
5. **Game records** (#315) are kept without a time limit. The record names
   no account; who sat in a seat is `game_record_seat.account_id` (and
   `delegated_by` for a host's seat bridge), both `ON DELETE SET NULL`
   (migrations `m20260927_000009`, `m20261006_000013`; held by
   `baylee-db/tests/schema.rs`): an account's deletion cuts its link to
   every record, and the game, between numbered seats, stays. Training and
   balancing read only the anonymised export (above).
6. **Reports:** a screenshot can show other players' names, and the text
   the player writes can contain anything; neither is filtered.
