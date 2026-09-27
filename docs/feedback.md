# Bug reports and feedback

A player reports a bug, an idea or a crash from the client. The client sends
it to the gateway it is signed in at, the gateway adds what only it knows and
passes it to the feedback service, and the service keeps it for whoever reads
reports. This document is normative for the gateway's `POST /reports`, the
service's API, and what either keeps.

```
client ── POST /reports ──> gateway ── POST /intake/reports ──> baylee-feedback
          (session)                    (gateway's intake token)     │
                                                                    └── GET/PATCH/DELETE /reports
                                                                        (read / admin token)
```

## The gateway: `POST /reports` (#307)

Signed-in players only (`Authorization: Bearer <session>`), guests included.
The body, frozen:

```json
{
  "kind": "bug" | "improvement" | "feedback" | "crash" | "other",
  "text": "…",                 // at most 20000 characters
  "game_id": "…" | null,        // the game the report is about, if any
  "client": { … }               // anything the client wants to say; at most 2 MB serialized
}
```

| answer | when |
| --- | --- |
| `201 {"report_id": "…"}` | the service took it; the id is the service's |
| `400` | not JSON, an unknown `kind`, no `text`, or `client` not an object |
| `401` | no session, or not a live one |
| `413` | `text` over 20000 characters, `client` over 2 MB, or the body over 2.25 MB |
| `429` | the account has sent 20 reports in the last hour |
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
  table's record attached by naming it.

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
| `FEEDBACK_DATABASE_URL` | required; its own database. Its tables are `feedback_report` and `feedback_migrations`, so it can share a database with a gateway, but should not |
| `FEEDBACK_GATEWAY_TOKENS` | `name=token,name=token`: one intake token per gateway. A name is letters, digits, `.`, `-`, `_`, at most 64 |
| `FEEDBACK_READ_TOKEN` | reads reports |
| `FEEDBACK_ADMIN_TOKEN` | reads, changes a report's status, deletes reports |
| `FEEDBACK_BIND` | `127.0.0.1:28780` |
| `FEEDBACK_POOL` | database connections, 4 |

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

`GET /reports?status=&kind=&gateway=&reporter=&game_id=&limit=&offset=`
lists newest first (`limit` 50, at most 200):

```json
{
  "total": 12,
  "reports": [{
    "id": "…", "created_at": "2026-09-27T12:00:00Z", "updated_at": "…",
    "gateway": "eu", "gateway_name": "Baylee EU", "gateway_url": "…",
    "gateway_version": "0.1.0-beta.1+build.42 (3f9a1c7e21)",
    "reporter": "5bdc…", "kind": "bug", "status": "new",
    "text": "…", "game_id": "…",
    "has_record": true, "record_complete": true, "record_bytes": 21606
  }]
}
```

`GET /reports/{id}` is one report with its `client` object as well.
`GET /reports/{id}/record` is the game record as stored: `application/gzip`,
JSON Lines inside (`zcat` reads it); `404` when the report has none. It
replays with `baylee_gamehost::record::replay` on the build its header names.

### Changing and deleting: `PATCH /reports/{id}`, `DELETE /reports/{id}`

The admin token. `PATCH` takes `{"status": "new" | "triaged" |
"in_progress" | "resolved" | "wont_fix" | "duplicate"}` and answers the
report; `DELETE` answers `204`, and `404` for a report that is not there.
Deleting is how a report goes: the service keeps reports until they are
deleted.

## What is kept, and where

- **The gateway** keeps every game's record (`game_record`,
  `game_record_chunk`, `game_record_seat`) without a limit, and which account
  sat where until the account is deleted. It keeps nothing of a report: it
  counts reports per account in memory for the rate limit, and forgets.
- **The service** keeps each report as it came, the gateway's name, the
  pseudonym, and the record if one was attached, until an admin deletes it.
  It has no column for an address or a name, and no request's address is
  logged.

## Running it

`scripts/server/baylee-feedback.service` is the unit, built and installed by
`scripts/server/baylee-deploy` beside the gateway. It reads
`/etc/baylee/feedback.env`:

```sh
FEEDBACK_DATABASE_URL=postgres://baylee_feedback:…@127.0.0.1:5432/baylee_feedback
FEEDBACK_GATEWAY_TOKENS=eu=<openssl rand -hex 32>
FEEDBACK_READ_TOKEN=<openssl rand -hex 32>
FEEDBACK_ADMIN_TOKEN=<openssl rand -hex 32>
RUST_LOG=info
```

and the gateway's `/etc/baylee/gateway.env` gets `BAYLEE_FEEDBACK_URL`,
`BAYLEE_FEEDBACK_TOKEN` (the `eu` token above) and `BAYLEE_FEEDBACK_KEY`.
