# Bug reports and feedback

A player reports a bug, an idea or a crash from the client. The client sends
it to the gateway it is signed in at, the gateway adds what only it knows and
passes it to the feedback service, and the service keeps it for whoever reads
reports. A client signed in nowhere sends it to the service itself
(§"Straight from a client"). This document is normative for the gateway's
`POST /reports`, the service's API, and what either keeps.

```
client ── POST /reports ──> gateway ── POST /intake/reports ──> baylee-feedback
          (session)                    (gateway's intake token)     │
client ── POST /client/reports ────────────────────────────────────>│
          (signed in nowhere: no credential, a device id)          │
                                                                    ├── GET/PATCH/DELETE /reports
                                                                    │   (read / admin token)
                                                                    └── /ui/api/… and the web UI
                                                                        (an admin's session, #311)
```

The client picks the road (`client-core::bugreport::route`): the gateway
it is signed in to whenever there is one, else the service it knows, else
none, and then it says so and sends nothing anywhere. Which service it
knows: `feedback_url` in its settings file, else the address the build was
given (`BAYLEE_FEEDBACK_PUBLIC_URL` at compile time); `https://`, or
`http://` to the same machine, and an empty setting turns it off.

## The gateway: `POST /reports` (#307)

Signed-in players only (`Authorization: Bearer <session>`), guests included.
The body, frozen:

```json
{
  "kind": "bug" | "improvement" | "feedback" | "crash" | "other",
  "text": "…",                 // at most 20000 characters
  "game_id": "…" | null,        // the game the report is about, if any; at most 128 characters
  "client": { … },              // anything the client wants to say; at most 2 MiB serialized
  "local_record": {             // optional: the record of a game the client hosted itself
    "complete": true | false,
    "gzip_base64": "…"          // at most 4 MiB of gzip
  }
}
```

`local_record` is a game the client ran in its own process (against the
house), which no gateway keeps a record of: the client writes the same
JSON Lines a hosted game's engine does (`baylee_gamehost::record`) and
attaches it only when the player ticked it for that report. The gateway
cannot vouch for it and does not pretend to: it checks that it is base64
of something that starts as gzip and is at most 4 MiB, and passes it on
marked `record_origin: "client"`. A report carries a `game_id` or a
`local_record`, never both.

| answer | when |
| --- | --- |
| `201 {"report_id": "…"}` | the service took it; the id is the service's |
| `400` | not JSON, an unknown `kind`, no `text`, `client` not an object, `game_id` over 128 characters, a `local_record` beside a `game_id`, or one that is not base64 of gzip |
| `401` | no session, or not a live one |
| `413` | `text` over 20000 characters, `client` over 2 MiB, a `local_record` over 4 MiB, or the body over 7.58 MiB |
| `429` | the account has sent 64 reports in the last hour; one the service did not take (`502`) is not counted |
| `502` | the service did not take it (unreachable, refused, or no `report_id`); also a `local_record` the service found is no game record |
| `503 {"error":"reports are not configured"}` | `BAYLEE_FEEDBACK_URL` is not set |

What the gateway sends on, and nothing else:

- `gateway`: `{name, url, version}`: `BAYLEE_GATEWAY_NAME`,
  `BAYLEE_PUBLIC_URL` (either may be `null`) and the build
  (`baylee_build::short`).
- `reporter`: a pseudonym, HMAC-SHA256 of the account id under
  `BAYLEE_FEEDBACK_KEY`, in hex. The same for every report of one account at
  one gateway, and nothing else about the account can be read from it. With
  no key set it is made under a key derived from `BAYLEE_FEEDBACK_TOKEN`, so
  rotating the token then renames every reporter.
- `kind`, `text`, `game_id` and `client` as the player sent them.
- `record`: when `game_id` names a game the reporter sat at, that game's
  record (#315, `docs/protocol.md` §"The game record") as
  `{complete, gzip_base64}`; otherwise `null`. A player never gets another
  table's record attached by naming it. It reaches the moment of the report
  (#323): for a game that goes on, the gateway first asks the game's engine
  for what it has not sent yet and waits, at most 3 s, until that is stored
  (`docs/protocol.md` §"The game record"), so played again the record ends
  on the state the game was in when the report arrived. `complete` says
  whether the game's end is in it, and is `false` for every report filed
  while the game goes on; it does not say whether the record reaches the
  report. Only an engine that did not answer in time leaves the record short
  of the report, and the gateway logs that it went without: one of this
  build that was merely slow has sent everything older than 30 s by itself,
  and an older or a stuck one leaves the record where its last piece did.
  With a `local_record`, that record instead, as the client sent it.
- `record_origin`: `"gateway"` for a record the gateway kept, `"client"`
  for a `local_record`. A service from before this field ignores it and
  would file a client's record as the gateway's, so deploy the service
  before the gateway.

Never a name, username, address, session or IP. The client decides what goes
into `client` and says so to the player (`docs/privacy.md`).

Configuration: `BAYLEE_FEEDBACK_URL` (the service's base URL, for example
`https://feedback.baylee.acevik.de`), `BAYLEE_FEEDBACK_TOKEN` (this gateway's
intake token there), `BAYLEE_FEEDBACK_KEY` (the pseudonym key; set it, and
never change it, if reports of one player should stay linkable).

## The service: `baylee-feedback` (#308)

One binary with a Postgres database of its own. It binds to loopback by
default and is meant to sit behind a TLS reverse proxy.

| variable | meaning |
| --- | --- |
| `FEEDBACK_DATABASE_URL` | required; its own database. Its tables are `feedback_report`, `feedback_admin`, `feedback_session`, `feedback_audit` and `feedback_migrations`, so it can share a database with a gateway, but should not |
| `FEEDBACK_GATEWAY_TOKENS` | `name=token,name=token`: one intake token per gateway. A name is letters, digits, `.`, `-`, `_`, at most 64 |
| `FEEDBACK_READ_TOKEN` | reads reports |
| `FEEDBACK_ADMIN_TOKEN` | reads, changes a report's status, deletes reports |
| `FEEDBACK_BIND` | `127.0.0.1:28780` |
| `FEEDBACK_POOL` | database connections, 4 |
| `FEEDBACK_WEB_DIR` | the built web UI (`web/feedback/dist`); unset = no page, `/` is `404` |
| `FEEDBACK_TRUSTED_PROXIES` | comma-separated addresses whose `X-Forwarded-For` the sign-in limiter and the direct route's allowance believe; an entry that is not an address refuses startup |
| `FEEDBACK_DIRECT_KEY` | turns on `POST /client/reports` and keys its reporter pseudonyms; at least 16 characters and none of the tokens. Unset, the route answers `503`. Never change it if a device's reports should stay linkable |

Every token is at least 16 characters and no token may be given twice; the
service refuses to start otherwise. Tokens are kept as SHA-256 and compared in
constant time. `RUST_LOG=info` logs each report taken and deleted, by id.

### Intake: `POST /intake/reports`

A gateway's intake token. The body is what the gateway sends (above).
`201 {"report_id": "<uuid v7>"}`, `400` for a malformed report (as the
gateway's, plus a `reporter` that is not 1–128 letters and digits, or a
control character in a name, URL, version or game id), `401` for anything
but a gateway's token, `413` past the gateway's limits or a record over
64 MiB. The report is filed under the name its token was configured with,
whatever the gateway calls itself.

`record_origin` (`"gateway"` when absent) says who wrote the record. A
client's (`"client"`) is held to a client's bounds and checked for the
shape of a record before it is kept: at most 4 MiB of gzip that unpacks to
at most 32 MiB of JSON Lines (read only that far), a header of
`RECORD_VERSION` 1 first and nothing after it but input, chair and end
lines. That is all the service can check without the engine, which it does
not link; replaying it is the reader's (`zcat`, then
`baylee_gamehost::record::replay`). A shape it refuses is `400` (`413`
past the bounds), so the gateway answers its player `502`.

### Straight from a client: `POST /client/reports`

A client signed in to no gateway reports here itself. A shipped client
holds no secret, so the route asks for none and stands on its limits
instead. Off until `FEEDBACK_DIRECT_KEY` is set. The body, every field
named and no other taken:

```json
{
  "kind": "bug" | "improvement" | "feedback" | "crash" | "other",
  "text": "…",                       // 1–20000 characters
  "build": {"version": "…", "commit": "…" | null},
  "device": "<32 lowercase hex>",    // a random id the client made for reports
  "client": { … },                   // at most 1 MiB serialized: half a gateway's
  "record": {"complete": …, "gzip_base64": "…"}   // optional, as a gateway's local_record
}
```

| answer | when |
| --- | --- |
| `201 {"report_id": "…"}` | taken |
| `400` | not JSON, an unknown field, an unknown `kind`, no `text`, `client` not an object, a `device` that is not 32 lowercase hex digits, a `build` that is empty or over 128 characters, or a `record` that is not base64 or not a record's shape (as the intake checks a client's) |
| `413` | `text` over 20000 characters, `client` over 1 MiB, a `record` over 4 MiB (32 MiB unpacked), or the body over 6.46 MiB |
| `429` | 10 direct reports from one address in the last hour, or 600 from everyone together |
| `503` | `FEEDBACK_DIRECT_KEY` is not set |

The allowances count every request, a refused one too, so probing costs as
much as sending; they are memory only, forgotten on restart, and the
address they count under (`X-Forwarded-For` from a trusted proxy, else the
peer) is never written down or logged. The reporter is
`HMAC-SHA256(key, "device\0" + device)` under a key derived from
`FEEDBACK_DIRECT_KEY`: one device's reports line up, and neither the device
id nor anything about an account or an address is kept. Such a report is
stored apart: `channel = 'direct'`, gateway `(direct)` (a name no intake
token can be configured under), `game_id` empty, and its record, if any,
`record_origin = 'client'`. The route answers a browser's preflight and
names any origin (`Access-Control-Allow-Origin: *`, never with
credentials), because a browser build of the client is served from its own
origin; no other route of the service does.

The client confirms first: a direct report, and any report carrying a
record, goes only after a page listing where it goes and every part it
carries (`client-core::bugreport::ReportForm::parts`).

### References in the text

`text` stays plain printable text and reads on its own: a card the player
named is written `[Lightning Bolt]` (the brackets the game log uses), a
player `[@steady 1]`, the name as the player saw it. What the text cannot
say rides in the free-form `client` object, so neither body changes and the
service checks nothing new:

```json
"refs": {
  "cards":   [{"text": "Lightning Bolt", "at": [5, 21], "card": 1234,
               "print": {"scryfall_id": "…", "lang": "en", "finish": "normal"},
               "face": 0, "object": 17, "zone": "stack", "owner": 1}],
  "players": [{"text": "steady 1", "at": [25, 36], "seat": 1}]
}
```

`at` is `[start, end)` in characters of `text`, brackets included; `card`
the registry index (the append-only ledger's, stable across builds);
`print` the printing; `object`, `zone` and `owner` only for a report
written at a table, `owner` and `seat` seat numbers as the view counts
them, never an account. A bracket naming nothing the client offered is not
a reference and has no entry; a report whose text names nothing has no
`refs` key, and its body is the bytes it was before references existed.
The client builds it (`client-core::bugreport::refs::refs_for`) from the
candidates the form offered: at a table only the seat's own view
(`refs::candidates`, never a library, another hand or a face-down card),
elsewhere the compiled pool. The admin UI draws each resolved reference as
a chip in its place in the text (a card linking to its Scryfall page, with
its picture on hover; a player as a seat), lists them under the text, and
leaves a bracket the client resolved to nothing, or whose range does not
say what the text says there, as the text the player wrote. A card's
picture is loaded by the admin's browser straight from Scryfall's image
host, never through this service (`docs/legal.md` §3; §"Headers" below).

`scripts/server/feedback-direct.caddy` is the proxy's part: the route
alone on the public site, its body bounded before the service reads it, no
access log.

### Reading: `GET /reports`, `GET /reports/{id}`, `GET /reports/{id}/record`

The read token or the admin token.

`GET /reports?status=&kind=&gateway=&reporter=&game_id=&channel=&q=&from=&to=&has_record=&limit=&offset=`
lists newest first (`limit` 50, at most 200). `q` is words the text holds,
case aside (`%` and `_` are literal); `from` and `to` are days,
`YYYY-MM-DD` in UTC, both inclusive; `has_record` is `true` or `false`;
`channel` is `gateway` or `direct`. A malformed filter is `400`.

`channel` says how a report came (`gateway`, or `direct`: unauthenticated,
§"Straight from a client"); `record_origin` who wrote its record
(`gateway`, or `client`: unverified), `null` without one. The web UI says
both beside the report.

```json
{
  "total": 12,
  "reports": [{
    "id": "…", "created_at": "2026-09-27T12:00:00Z", "updated_at": "…",
    "gateway": "eu", "gateway_name": "Baylee EU", "gateway_url": "…",
    "gateway_version": "0.1.0-beta.1+build.42 (3f9a1c7e21)",
    "reporter": "5bdc…", "kind": "bug", "status": "new",
    "text": "…", "game_id": "…",
    "has_record": true, "record_complete": true, "record_bytes": 21606,
    "channel": "gateway", "record_origin": "gateway",
    "issue_number": 311,
    "issue_url": "https://github.com/AceVik/baylee/issues/311"
  }]
}
```

`issue_url` is made from `issue_number` (an issue of `AceVik/baylee`), never
stored as typed. Both are `null` for a report linked to no issue.

`GET /reports/{id}` is one report with its `client` object as well.
`GET /reports/{id}/record` is the game record as stored: `application/gzip`,
JSON Lines inside (`zcat` reads it), its writer in `X-Record-Origin`
(`gateway` or `client`); `404` when the report has none. It
replays with `baylee_gamehost::record::replay` on the build its header names.

### Changing and deleting: `PATCH /reports/{id}`, `DELETE /reports/{id}`

The admin token. `PATCH` takes `{"status": "new" | "triaged" |
"in_progress" | "resolved" | "wont_fix" | "duplicate"}` and answers the
report; `DELETE` answers `204`, and `404` for a report that is not there.
Deleting is how a report goes: the service keeps reports until they are
deleted. Each change and each deletion is written to the audit
(`feedback_audit`) under the actor `token`.

## The web UI (#311)

`https://feedback.baylee.acevik.de/` is a page for admins to triage reports
on: a filtered, paged list, one report with everything it carries, its
status, its GitHub issue, and deletion. The page is built from `web/feedback`
(React, TypeScript; `web/feedback/README.md`) into static files the service
serves itself from `FEEDBACK_WEB_DIR`. Unset, the service serves no page and
`/` stays `404`; a directory without an `index.html` refuses to start.

What the list does (beta.7; the README has the keys): views by status,
search and filters in the URL, a drawer that opens a report beside the
list, a selection that sets a status on several reports (one `PATCH` and
one audit row each; there is no bulk route), a quiet refresh every 30
seconds that announces new reports instead of moving the rows, an export
of what is shown as CSV made in the browser, and an unread marker kept in
the browser's `localStorage` only (the service keeps no per-admin state).
**A reporter is never resolved to an account.** The pseudonym is an HMAC
the gateway makes so that this service cannot know who wrote a report
(§"The gateway" above, `docs/privacy.md`); asking the gateway's console to
turn it back into a name would undo that promise in one request, so the UI
does not, and the gateway's console has no such route. The UI shows an
alias derived in the browser from the pseudonym itself (two words, a hue,
its first four characters; `web/feedback/src/reports/alias.ts`), which is
stable per reporter and per gateway and says nothing more than the hex
does.

### Admins: `baylee-feedback admin …`

Admins are kept in the service's database and are made only on the server,
by the binary:

```sh
baylee-feedback admin add <name>      # the password is one line of stdin;
                                      # on a terminal it is asked twice, unechoed
baylee-feedback admin remove <name>   # and every session they hold
baylee-feedback admin list            # name, since when, sessions
```

with `FEEDBACK_DATABASE_URL` set (on the server: `sudo -u … env $(cat
/etc/baylee/feedback.env) /opt/baylee/bin/baylee-feedback admin add viktor`).
A name is 1–64 letters, digits, `.`, `-`, `_`; a password at least 12
characters. Passwords are kept as Argon2id. No route makes, lists or
removes an admin.

### Signing in and sessions

`POST /ui/api/login` `{"name", "password"}` answers `200 {"name"}` and sets
the session cookie; a wrong password and an unknown name get the same
`401 {"error":"wrong name or password"}` after the same Argon2 run.

- **The session** is a random 256-bit token in the cookie
  `__Host-baylee-feedback` (`HttpOnly; Secure; SameSite=Strict; Path=/`),
  kept in `feedback_session` only as its SHA-256. It ends after **12 hours**
  unused or **7 days** after sign-in, whichever comes first; each request
  moves the first clock. `POST /ui/api/logout` deletes it in the database,
  and removing the admin deletes all of theirs. Lapsed sessions are deleted
  when they are next shown and at every sign-in. This cookie is the
  service's own UI talking to its own origin; the gateway's no-cookie rule
  is the gateway's.
- **Limits:** 10 failed sign-ins per address and 5 per name in 15 minutes;
  past either, `429` with `Retry-After`, the right password included. The
  counts are in memory only and a restart forgets them. Behind a proxy set
  `FEEDBACK_TRUSTED_PROXIES` to its address(es), or every sign-in counts
  under the proxy's: the limiter then reads `X-Forwarded-For` from the
  right, as the gateway does.
- **CSRF:** every changing `/ui/api` request (sign-in and sign-out included)
  must carry `X-Baylee-CSRF: 1` and an `Origin` whose host is the request's
  `Host`, and must not be marked `Sec-Fetch-Site` other than `same-origin`;
  otherwise `403`. A form on another site can send neither header, and a
  cross-site `fetch` that tries is preflighted and never allowed (there is
  no CORS).

### The UI's routes: `/ui/api/…`

A live session, and nothing else: the read and admin tokens open none of
these, and a session opens none of the token routes. Without one, every
route below is `401`.

| route | what |
| --- | --- |
| `GET /ui/api/me` | `{"name", "gateway_admin"}`: the second says whether the admin console is configured (below) |
| `GET /ui/api/reports?…` | the list, with the filters above |
| `GET /ui/api/reports/{id}` | one report with `client` |
| `GET /ui/api/reports/{id}/record` | the record, as a download |
| `GET /ui/api/reports/{id}/audit` | `[{"at", "actor", "action", "detail"}]`, oldest first; kept after the report is deleted |
| `PATCH /ui/api/reports/{id}` | `{"status"?, "issue"?}`: `issue` a positive number links, `null` unlinks, absent leaves it; answers the report |
| `DELETE /ui/api/reports/{id}` | `204` |
| `GET /ui/api/facets` | `{gateways, statuses, kinds, reporters}`, each `[{"value","count"}]`; the 50 busiest reporters |
| `POST /ui/api/login`, `POST /ui/api/logout` | above |
| `/ui/api/admin/…` | the admin console, below |

The UI links an issue by number and opens GitHub's own "new issue" page;
the admin submits it there. The service holds no GitHub token and never
calls GitHub. **What goes into that public issue** (owner, 08.10.2026) is
only a neutral technical summary the admin writes themselves, in a field
that is empty until they do and says "no personal data, no quotes from the
report", a neutral category (bug, crash, improvement, feedback, other; the
report's kind until changed) and the build. Never the player's text, their
pseudonym, the client's details, the gateway, the arrival time, the report's
id or a link back to it: the service's address and a report id (a UUIDv7,
which carries when the report came) tell a stranger nothing they need, and
the way back runs inside the service, where the report keeps the issue's
number. Until a summary is written there is no link to open
(`web/feedback/src/github.ts`; `e2e/triage.spec.ts` holds the issue URL to
no report text).

### The admin console (`/ui/api/admin/…`)

The UI's admin area shows one gateway: its **Overview** (`/admin`:
headline numbers, a month of days as charts, the open tables and every
count), **Live** (`/admin/live`: who is online and where, every waiting and
running table with its chairs, the agents; every 5 seconds), **Accounts**
(`/admin/accounts`: searched, filtered and sorted in the query string;
`/admin/accounts/{id}`: one account, its decks and latest games) and
**Beta keys** (`/admin/keys`), so the owner needs a shell for none of it.
A sidebar on a wide screen, a tab bar at the bottom of a phone. The
service passes each request on to the gateway's admin console
(`docs/protocol.md` §"The admin console") with the gateway's token, which
the browser never sees:

| route | what |
| --- | --- |
| `GET /ui/api/admin/stats` | the gateway's `GET /admin/stats`, as it answers |
| `GET /ui/api/admin/live` | the gateway's `GET /admin/live` |
| `GET /ui/api/admin/accounts` | the gateway's `GET /admin/accounts`; the query is read into `q`, `kind`, `sort`, `online`, `offset`, `limit` and written out again (anything else `400`) |
| `GET /ui/api/admin/accounts/{id}` | the gateway's `GET /admin/accounts/{id}`; `400` for an id that is not a UUID |
| `GET /ui/api/admin/invites` | its keys, newest first; never a key |
| `POST /ui/api/admin/invites` | `{"count"?, "uses"?, "expires"?, "note"?}` as `invite create` takes them (`"30d"`, `"12h"`); `201 {"keys": [{"id", "key"}], "uses", "expires_at", "note"}`, the only time a key is shown |
| `DELETE /ui/api/admin/invites/{id}` | revokes one: `204`, or `404` for no key not already revoked |
| `GET /ui/api/admin/audit` | the 50 latest changes made here, `[{"at", "actor", "action", "detail"}]`, newest first |

- **Who.** A live session on every route (`401` without), and on the two
  that change something the CSRF rule above too (`403`). The read and admin
  tokens open none of them.
- **Configured by** `FEEDBACK_GATEWAY_ADMIN_URL` (`http://127.0.0.1:28767`)
  and `FEEDBACK_GATEWAY_ADMIN_TOKEN` (the gateway's `BAYLEE_ADMIN_TOKEN`),
  both or neither, else the service refuses to start; the token has 32
  characters at least and is none of the service's own tokens or its direct
  key. Unset, the routes answer `404` and the Overview tab is not drawn.
- **What reaches the gateway.** The service builds every request itself: a
  body is read into the four fields and written out again (anything else is
  `400` here), an id must be a UUID, and only `Authorization` (the
  service's token) and `X-Baylee-Admin` (the signed-in admin's name) are
  sent, never the cookie or `Origin`. Proxy settings in the environment and
  redirects are ignored, so the token goes to that address and nowhere else.
- **What comes back.** The gateway's answer and status as they are, with
  three exceptions: a gateway refusing the token (`401`, `403`) is `502`
  here, never a `401` that the UI would read as "signed out"; its limit
  (`429`) is `503`; a gateway that does not answer is `502`.
- **Audit.** Every change that succeeded is a row in `feedback_audit`
  without a report (action `gateway.invite.create` or
  `gateway.invite.revoke`, the admin, the time, and the ids, uses and
  expiry; never a key or a note) and an `info` line in the log; the gateway
  logs its own line too. A key's note may name a person, so it is kept only
  at the gateway, as the command keeps it.
- **The page.** Mobile first (one column on a phone, 44 px targets, safe-area
  insets, a sticky bar that folds as the page scrolls), light and dark as
  the system says, English or German as the browser asks. The numbers are
  asked again every 15 seconds while the page is visible and not while it
  is hidden. New keys are shown once with Copy, Copy all and, where the
  browser has it, Share; there is no invite link, because the client takes
  a key only typed into its sign-up form. Revoking asks once more.

Several gateways are not configured yet: the routes would take a
`?gateway=<name>` and the two settings a list of `name=url` pairs, as
`FEEDBACK_GATEWAY_TOKENS` does.

### Headers

Every answer of the service, page, file or JSON, carries
`Content-Security-Policy: default-src 'self'; script-src 'self'; style-src
'self'; img-src 'self' data: https://cards.scryfall.io; font-src 'self';
connect-src 'self'; object-src 'none'; base-uri 'none'; form-action
'self'; frame-ancestors 'none'` (`data:` images for a report's screenshot,
which is a PNG in the report; `cards.scryfall.io` for the picture of a card
a report names or a set lists, which the admin's browser fetches from
Scryfall itself and this service never touches, `docs/legal.md` §3 — an
image source only, as `connect-src` stays `'self'`), `X-Frame-Options:
DENY`, `Referrer-Policy: no-referrer` (so Scryfall sees a card id and
nothing of a report),
`X-Content-Type-Options: nosniff`, and same-origin opener and resource
policies. JSON is `Cache-Control: no-store`. Files under `assets/` (named by
their hash) are `public, max-age=31536000, immutable`, the rest `no-cache`.
A path with no extension that names no file is the UI's own route and gets
`index.html`; `/ui/api`, `/reports`, `/intake` and `/health` never do. A
path with `..`, `.`, a backslash or a NUL, raw or percent-encoded, is
refused, and a file reached through a link out of the directory is not
served.

## What is kept, and where

- **The gateway** keeps every game's record (`game_record`,
  `game_record_chunk`, `game_record_seat`) without a limit, and which account
  sat where until the account is deleted. It keeps nothing of a report: it
  counts reports per account in memory for the rate limit, and forgets.
- **The service** keeps each report as it came, the gateway's name, the
  pseudonym, and the record if one was attached, until an admin deletes it.
  It has no column for an address or a player's name, and no request's
  address is logged. A direct report's pseudonym is the device's, keyed;
  the addresses its allowance counts are in memory for an hour.
- **The client** keeps the records of the last 20 games it hosted itself
  (at most 64 MiB together, gzipped) in `records/` beside its settings, and
  the random device id it sends direct reports under in its settings file.
  Neither leaves the machine except in a report the player confirmed
  (`docs/privacy.md`).
- **For the web UI** it keeps each admin's name and Argon2id hash until
  `admin remove`, each session's token hash with its sign-in and last-use
  times until it lapses or is ended (at most 7 days), and an audit row per
  change or deletion (time, admin name or `token`, report id, what) without
  a limit. Failed sign-ins are counted per address and per name in memory
  for 15 minutes and never written down; the log says that a sign-in
  failed, not by whom or from where, and names the admin who signed in or
  out. A change made through the admin console is an audit row without a
  report id (the admin, the time, key ids, uses, expiry), also without a
  limit; the gateway's numbers it shows are not stored here at all.

## Running it

`scripts/server/baylee-feedback.service` is the unit, built and installed by
`scripts/server/baylee-deploy` beside the gateway. It reads
`/etc/baylee/feedback.env`:

```sh
FEEDBACK_DATABASE_URL=postgres://baylee_feedback:…@127.0.0.1:5432/baylee_feedback
FEEDBACK_GATEWAY_TOKENS=eu=<openssl rand -hex 32>
FEEDBACK_READ_TOKEN=<openssl rand -hex 32>
FEEDBACK_ADMIN_TOKEN=<openssl rand -hex 32>
FEEDBACK_WEB_DIR=/opt/baylee/web/feedback
FEEDBACK_TRUSTED_PROXIES=127.0.0.1
FEEDBACK_DIRECT_KEY=<openssl rand -hex 32>   # only to take reports straight from clients
FEEDBACK_GATEWAY_ADMIN_URL=http://127.0.0.1:28767        # the admin console
FEEDBACK_GATEWAY_ADMIN_TOKEN=<the gateway's BAYLEE_ADMIN_TOKEN>
RUST_LOG=info
```

The last two need nobody to type them: `baylee-deploy stage`, where the
feedback unit is installed, makes one token with `openssl rand -hex 32`
when neither `/etc/baylee/gateway.env` nor `/etc/baylee/feedback.env` has
it, copies it across when one of them does, never replaces one, writes
`BAYLEE_ADMIN_TOKEN` to the first and the two `FEEDBACK_GATEWAY_ADMIN_…`
lines to the second, and leaves both files `0600`; the token is never
printed or put on a command line. The service restarts with it at once,
the gateway when `finish` restarts it (until then the Overview says the
gateway did not answer).

Direct reports also need the route on the public site
(`scripts/server/feedback-direct.caddy`, whose header says how to install
it) and clients built with the service's address:
`BAYLEE_FEEDBACK_PUBLIC_URL=https://feedback.example cargo build -p
baylee-client` (and the same for `trunk build`). A client built without
it, or told `""` in its settings, says it cannot send unless signed in.

`baylee-deploy stage` builds the web UI (`npm ci && npm run build` in
`web/feedback`) where npm is installed and puts `dist/` at
`/opt/baylee/web/feedback` before restarting the service; without npm, or
when the build fails, it says so and the UI installed last stays. Then make
the first admin with `baylee-feedback admin add <name>`.

and the gateway's `/etc/baylee/gateway.env` gets `BAYLEE_FEEDBACK_URL`,
`BAYLEE_FEEDBACK_TOKEN` (the `eu` token above) and `BAYLEE_FEEDBACK_KEY`.
