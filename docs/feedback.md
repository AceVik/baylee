# Bug reports and feedback

A player reports a bug, an idea or a crash from the client. The client sends
it to the gateway it is signed in at, the gateway adds what only it knows and
passes it to the feedback service, and the service keeps it for whoever reads
reports. This document is normative for the gateway's `POST /reports`, the
service's API, and what either keeps.

```
client ── POST /reports ──> gateway ── POST /intake/reports ──> baylee-feedback
          (session)                    (gateway's intake token)     │
                                                                    ├── GET/PATCH/DELETE /reports
                                                                    │   (read / admin token)
                                                                    └── /ui/api/… and the web UI
                                                                        (an admin's session, #311)
```

## The gateway: `POST /reports` (#307)

Signed-in players only (`Authorization: Bearer <session>`), guests included.
The body, frozen:

```json
{
  "kind": "bug" | "improvement" | "feedback" | "crash" | "other",
  "text": "…",                 // at most 20000 characters
  "game_id": "…" | null,        // the game the report is about, if any; at most 128 characters
  "client": { … }               // anything the client wants to say; at most 2 MB serialized
}
```

| answer | when |
| --- | --- |
| `201 {"report_id": "…"}` | the service took it; the id is the service's |
| `400` | not JSON, an unknown `kind`, no `text`, `client` not an object, or `game_id` over 128 characters |
| `401` | no session, or not a live one |
| `413` | `text` over 20000 characters, `client` over 2 MB, or the body over 2.25 MB |
| `429` | the account has sent 64 reports in the last hour; one the service did not take (`502`) is not counted |
| `502` | the service did not take it (unreachable, refused, or no `report_id`) |
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
  of the report, at most 30 s of play short while that engine lives (it
  sends what has waited that long by itself); the gateway logs that it went
  without.

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
| `FEEDBACK_TRUSTED_PROXIES` | comma-separated addresses whose `X-Forwarded-For` the sign-in limiter believes; an entry that is not an address refuses startup |

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

### Reading: `GET /reports`, `GET /reports/{id}`, `GET /reports/{id}/record`

The read token or the admin token.

`GET /reports?status=&kind=&gateway=&reporter=&game_id=&q=&from=&to=&has_record=&limit=&offset=`
lists newest first (`limit` 50, at most 200). `q` is words the text holds,
case aside (`%` and `_` are literal); `from` and `to` are days,
`YYYY-MM-DD` in UTC, both inclusive; `has_record` is `true` or `false`. A
malformed filter is `400`.

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
    "issue_number": 311,
    "issue_url": "https://github.com/AceVik/baylee/issues/311"
  }]
}
```

`issue_url` is made from `issue_number` (an issue of `AceVik/baylee`), never
stored as typed. Both are `null` for a report linked to no issue.

`GET /reports/{id}` is one report with its `client` object as well.
`GET /reports/{id}/record` is the game record as stored: `application/gzip`,
JSON Lines inside (`zcat` reads it); `404` when the report has none. It
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
| `GET /ui/api/me` | `{"name"}` |
| `GET /ui/api/reports?…` | the list, with the filters above |
| `GET /ui/api/reports/{id}` | one report with `client` |
| `GET /ui/api/reports/{id}/record` | the record, as a download |
| `GET /ui/api/reports/{id}/audit` | `[{"at", "actor", "action", "detail"}]`, oldest first; kept after the report is deleted |
| `PATCH /ui/api/reports/{id}` | `{"status"?, "issue"?}`: `issue` a positive number links, `null` unlinks, absent leaves it; answers the report |
| `DELETE /ui/api/reports/{id}` | `204` |
| `GET /ui/api/facets` | `{gateways, statuses, kinds, reporters}`, each `[{"value","count"}]`; the 50 busiest reporters |
| `POST /ui/api/login`, `POST /ui/api/logout` | above |

The UI links an issue by number and opens GitHub's own "new issue" page
prefilled with the report's text, kind, build, gateway, arrival time, record
state and a link back to the report (not its pseudonym or client details); the admin submits it there. The service holds no GitHub
token and never calls GitHub.

### Headers

Every answer of the service, page, file or JSON, carries
`Content-Security-Policy: default-src 'self'; script-src 'self'; style-src
'self'; img-src 'self' data:; font-src 'self'; connect-src 'self';
object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors
'none'` (`data:` images for a report's screenshot, which is a PNG in the
report), `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`,
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
  address is logged.
- **For the web UI** it keeps each admin's name and Argon2id hash until
  `admin remove`, each session's token hash with its sign-in and last-use
  times until it lapses or is ended (at most 7 days), and an audit row per
  change or deletion (time, admin name or `token`, report id, what) without
  a limit. Failed sign-ins are counted per address and per name in memory
  for 15 minutes and never written down; the log says that a sign-in
  failed, not by whom or from where, and names the admin who signed in or
  out.

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
RUST_LOG=info
```

`baylee-deploy stage` builds the web UI (`npm ci && npm run build` in
`web/feedback`) where npm is installed and puts `dist/` at
`/opt/baylee/web/feedback` before restarting the service; without npm, or
when the build fails, it says so and the UI installed last stays. Then make
the first admin with `baylee-feedback admin add <name>`.

and the gateway's `/etc/baylee/gateway.env` gets `BAYLEE_FEEDBACK_URL`,
`BAYLEE_FEEDBACK_TOKEN` (the `eu` token above) and `BAYLEE_FEEDBACK_KEY`.
