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
  "kind": "bug" | "improvement" | "feedback" | "crash" | "other",   // a gateway also sends "chat"
  "text": "…",                 // at most 20000 characters
  "game_id": "…" | null,        // the game the report is about, if any; at most 128 characters
  "client": { … },              // anything the client wants to say; at most 2 MiB serialized
  "local_record": {             // optional: the record of a game the client hosted itself
    "complete": true | false,
    "gzip_base64": "…"          // at most 4 MiB of gzip
  }
}
```

Every attachment a report can carry (system details, the table, the game
log, settings, the picture of the Baylee window, and a local record) starts
selected when the form opens (owner, 10.10.2026); the player can untick any
of them before sending, and only what is still ticked is sent
(`docs/privacy.md` §"Reports and crash reports").

`local_record` is a game the client ran in its own process (against the
house), which no gateway keeps a record of: the client writes the same
JSON Lines a hosted game's engine does (`baylee_gamehost::record`) and
attaches it only when its box is ticked for that report (ticked at each
opening of the form; the player may untick it, and the confirmation names it). The gateway
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

A language-model seat's bridge sends its model's reports of rules or engine
errors here too, one `bug` a game under the seat's own session, with
`client.source: "llm-seat"`, the model, the profile, each report in
`client.reports` (its `log_line` saying after which line of the log it
was made), and the seat's own game log as `client.log`, in the shape the
desktop client sends its log in: the reporter "You", every other seat
`P1…Pn` as the model knew them, the oldest lines left out where `client`
would pass 2,000,000 bytes and `client.log_omitted` saying how many
(`docs/llm-seat.md` §"Reports and lookups"). They are a model's claims:
triage reads them as it reads a player's. The seat's account sat at the
game (a host's own chair handed to its bridge, `--host-chair`, too: the
account's own chair in `game_record_seat`), so the record is attached as
to a player's report. A chair taken on its host's ticket has no session
and sends its report on its seat token instead (below).

### A host's seat bridge: `POST /lobby/games/{id}/chair/report`

A seat bridge sitting in a chair on its host's chair ticket
(`docs/protocol.md` §"A host's chair for a seat bridge") has a seat token
and no session. It sends its language model's report about its game here
(owner, 31.10.2026), `Authorization: Bearer <seat token>`, in the body
above, and the gateway passes it on exactly as it does a `POST /reports`,
with these differences:

- **Whose it is.** The host who handed the chair over answers for it
  (`Delegate::by`, `game_record_seat.delegated_by`), so `reporter` is the
  host's pseudonym and the report counts against the host's 64 an hour
  (owner, 31.10.2026). A hosted model the host ordered for the chair is
  the host's delegate too.
- **About its game only.** The game is the one the token opens: a
  `game_id` is empty or that game's (`400` otherwise), and the gateway
  sends the report under that id either way. No `local_record` (`400`).
- **Its record.** That game's, read for the chair that played it
  (`records::for_delegate`: that game, that chair, that host as
  `delegated_by` with no account), never another table's.
- **Once a chair a game** (`409` after), the chance given back when the
  service did not take it (`502`), and for as long as the room is in the
  lobby: an hour after the game ends.
- **A person's chair is refused** (`403`): a person reports with their
  session. A token that opens nothing, or not this game, is `401`; a body
  over 2.25 MiB is refused before it is read.

Configuration: `BAYLEE_FEEDBACK_URL` (the service's base URL, for example
`https://feedback.baylee.app`), `BAYLEE_FEEDBACK_TOKEN` (this gateway's
intake token there), `BAYLEE_FEEDBACK_KEY` (the pseudonym key; set it, and
never change it, if reports of one player should stay linkable).

## A reported chat message: `POST /social/reports` (beta.8)

A chat message a player reports goes through its own gateway route, never
`POST /reports`, whose body stays frozen. The player's client sends the
message as it was delivered, its message key and the body it decrypted to;
the gateway checks the sender's signature and that the key opens the
ciphertext to exactly that body (`baylee_seal::verify_report`,
`docs/protocol.md` §"Friends and chat (beta.8)"), then forwards to the
intake as any report with `kind: "chat"`:

- `text`: the reporter's note, then the message's text;
- `game_id`: the table's game for a table or team message, else none;
- `client.chat`: `{verified, conversation ("whisper" | "group" | "table" |
  "team"), sender_tag (the sender's public tag in hex, only when
  `verified`; else null), sent_at (the sender's clock, ms), message,
  note}`. No sender pseudonym (since 13.10.2026): it was the same keyed
  hash a reporter is known by, and beside the tag it would have resolved
  every report that account ever filed;
- no record.

`verified: false` means the check failed or could not run (the sender's
device was removed since): say so before acting on it. Such a report names
no sender tag, so nobody can be framed by a forged report.

**Who can be found (13.10.2026).** The sender of a verified message can:
a moderator must be able to act on reported content (ban the account that
wrote it), and the tag is public in every handle. The report detail shows
the tag and *Find sender's account*, which opens **Accounts** filtered to
that one tag (`?tag=`, the gateway's `GET /admin/accounts?tag=`, exact,
registered or guest). The reporter cannot (below, §"A reporter is never
resolved to an account"). 10 reports an hour
per account; `503` without `BAYLEE_FEEDBACK_URL`. Deploy the service (which
knows the `chat` kind since its migration `m0006_chat_kind`) before the
gateway. `POST /client/reports` refuses `chat`: only a gateway can check one.

## The service: `baylee-feedback` (#308)

One binary with a Postgres database of its own. It binds to loopback by
default and is meant to sit behind a TLS reverse proxy.

| variable | meaning |
| --- | --- |
| `FEEDBACK_DATABASE_URL` | required; its own database. Its tables are `feedback_report`, `feedback_contact`, `feedback_admin`, `feedback_session`, `feedback_audit` and `feedback_migrations`, so it can share a database with a gateway, but should not |
| `FEEDBACK_GATEWAY_TOKENS` | `name=token,name=token`: one intake token per gateway. A name is letters, digits, `.`, `-`, `_`, at most 64 |
| `FEEDBACK_READ_TOKEN` | reads reports |
| `FEEDBACK_ADMIN_TOKEN` | reads, changes a report's status, deletes reports |
| `FEEDBACK_BIND` | `127.0.0.1:28780` |
| `FEEDBACK_POOL` | database connections, 4 |
| `FEEDBACK_WEB_DIR` | the built web UI (`web/feedback/dist`); unset = no page, `/` is `404` |
| `FEEDBACK_TRUSTED_PROXIES` | comma-separated addresses whose `X-Forwarded-For` the sign-in limiter and the direct route's allowance believe; an entry that is not an address refuses startup |
| `FEEDBACK_DIRECT_KEY` | turns on `POST /client/reports` and keys its reporter pseudonyms; at least 16 characters and none of the tokens. Unset, the route answers `503`. Never change it if a device's reports should stay linkable |
| `FEEDBACK_CONTACT_ORIGINS` | turns on `POST /site/contact` for the pages served at these origins, comma-separated: each `https://host[:port]`, or `http://` for `localhost`, `127.0.0.1` or `[::1]` only (a page tried on this machine); a path, a trailing slash, credentials or plain `http://` to anywhere else refuses startup. Unset, the route answers `503` |

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

### Offline now: `POST /client/alive`

How many people are playing offline right now, anonymously. While a game a
client hosts itself (`LocalHost`) runs, and its player has left "Send
anonymous usage count" on (Settings › Privacy & Data, default on), the
client sends once a minute (`client-core::usage::BEAT_SECS`):

```json
{ "game": "<32 lowercase hex digits>" }
```

and nothing else: the value is random, made for that one game, held in the
client's memory only and forgotten when the game ends. It is not the
direct reports' device id, not an account, nothing about the game; it is
there only so a game is counted once, not once per beat. The body is
`deny_unknown_fields` and at most 256 bytes; anything else is `400` and
not counted. The answer is `204`.

The service keeps each value with the time it was last heard, in memory
(`alive::Presence`, at most 20 000), and drops it 150 s after its last beat
(`alive::TTL`: two beats and a margin). Nothing is written to the database
or logged per beat; a restart forgets it all. The route stands on the
direct reports' switch (`FEEDBACK_DIRECT_KEY`, `503` without), answers any
origin as they do, and has an allowance of its own, in memory: 300 beats
per address and 120 000 in all per hour. With the setting off, or with no
feedback service known (`feedback_url` `""`, no `BAYLEE_FEEDBACK_PUBLIC_URL`),
the client sends nothing.

The admin overview shows the count ("Offline now"), from
`GET /ui/api/admin/offline` (`{"offline_now": n, "window_secs": 150}`), for a
signed-in admin; it needs no gateway console.

### The landing page's contact form: `POST /site/contact`

The landing page (`web/site`, `#contact`) has a form for a message to the
people behind Baylee. The page is served from the gateway's host, the
service from its own, so the form posts across origins with `fetch`; the
page's CSP names the service in `connect-src` (`scripts/server/site.caddy`).
Off until `FEEDBACK_CONTACT_ORIGINS` is set. The body, every field named
and no other taken:

```json
{
  "name": "…" | null,      // optional, at most 100 characters, no control character
  "email": "…" | null,     // optional, at most 254, one @ and a dot after it, no whitespace
  "message": "…",          // 1–5000 characters after trimming; line breaks and tabs only
  "language": "en" | "de", // the page's language when it was sent
  "website": ""            // the honeypot: the page hides it from people
}
```

| answer | when |
| --- | --- |
| `201 {"ok": true}` | taken; also when `website` is filled, and then nothing is stored |
| `400` | not JSON, an unknown field, an empty message, a malformed name or address, another language |
| `403` | the request's `Origin` is missing or not one of `FEEDBACK_CONTACT_ORIGINS` |
| `413` | a field over its bound, or the body over 32 KiB |
| `429` | 5 messages from one address in the last hour, or 100 from everyone together |
| `503` | `FEEDBACK_CONTACT_ORIGINS` is not set |

Every answer says `Vary: Origin` and names the asking origin in
`Access-Control-Allow-Origin` only when it is a configured one, never with
credentials; the preflight (`OPTIONS`) answers `204` to those and `403` to
any other. A browser always sends an `Origin` with such a `fetch`, so a
request without one is refused too. The allowance counts every request
past the origin check, a refused one too, in memory only, under the address
the direct reports use (`X-Forwarded-For` from a trusted proxy, else the
peer), never written down or logged. There is no CAPTCHA and no third
party: the honeypot, the allowances and the bounds are the defence.

What is stored (`feedback_contact`): the message, the name and the address
if given (blanks are none), the language, when it came, whether an admin
has dealt with it (`new` or `done`, and since when). Nothing about the
request. A message is deleted 90 days after it was marked done and a year
after it came in any case (`contact::sweep`, run before a message is taken
and before the list is read), or earlier by an admin. The log says that a
message was taken, by id only.

The admins read them in the web UI's *Contact* page (`/contact`, every
admin, no gateway console needed):

| route | what |
| --- | --- |
| `GET /ui/api/contact?status=new\|done` | `{"open", "total", "messages": […]}`: the newest 500 the filter matches, and how many are open |
| `GET /ui/api/contact/{id}` | one message with its `audit` |
| `PATCH /ui/api/contact/{id}` | `{"status": "new" \| "done"}`; answers the message |
| `DELETE /ui/api/contact/{id}` | `204` |

A change and a deletion are audit rows (`feedback_audit.contact_id`, the
admin, the time, `status` with the new status or `delete`), never with the
text or the address; they outlive the message. The console's audit list
leaves them out. An answer goes by the admin's own mail (the page's
*Reply by e-mail* is a `mailto:` link); the service sends no mail.

### The Impressum: `/site/impressum`

The landing page's Impressum (`web/site/impressum.html`) names the operator,
a private person; the details are in no file of the repository and in
nothing a crawler reads as text (`web/site/README.md` §"The Impressum",
`docs/legal.md` §4). The service hands them out for a solved proof of work
(`crates/baylee-feedback/src/impressum.rs`). The site's Caddy passes
`/site/impressum` and everything under it on to the service
(`scripts/server/site.caddy`), so the page asks its own origin. Off (`503`)
until `FEEDBACK_IMPRESSUM_KEY` (at least 32 characters, none of the
service's tokens) and `FEEDBACK_IMPRESSUM_BLOB` (the file
`crates/baylee-tools/src/site/impressum.rs` wrote, on the server only, read on each
request) are set, both or neither; a file that is not such a blob refuses
startup.

| route | what |
| --- | --- |
| `GET /site/impressum/challenge?s=<bits>` | `{"nonce", "parts", "difficulty", "expires", "sig"}`: 8 parts, 17 zero bits each for an ordinary browser, valid 5 minutes, signed with HMAC-SHA256 under the key |
| `POST /site/impressum` | the challenge's fields and `"counters"` (one per part); `200` the blob as `text/plain` |
| `GET /site/impressum/kontakt` | the honeypot: an invented Impressum as HTML |

The puzzle, the same in `web/site/impressum-worker.js`: for each part, the
smallest counter `c` with `SHA-256(nonce + ":" + part + ":" + c)` starting
with `difficulty` zero bits. About a second of a laptop's time at 17 bits
(the worker measured 2.4 million hashes a second in Chromium on an Apple
laptop, so 8 × 2¹⁷ ≈ 0.4 s there); it starts only when the visitor presses
*Impressum anzeigen*.

**No person is ever locked out** (owner, 13.10.2026): no challenge is
refused, a suspicious request only gets up to 4 more bits (each doubles
the work; at 21 bits about 7 s on that laptop). Points: more than 10
challenges from one address in an hour (+1, +2 past 30, +3 past 100), no
`Accept-Language` (+1), no `Sec-Fetch-Site` (+1), an empty user agent or one
that names a tool, a crawler or a headless browser (+2), and what the page
reports in `s`: `navigator.webdriver` (1, +2), no `navigator.languages` (2,
+1), a software WebGL renderer such as SwiftShader or llvmpipe (4, +1). Only a
robot beyond doubt (6 points with both a headless/tool user agent and
`webdriver`) or an address that asked the honeypot in the last 24 hours gets
a **decoy**: a blob of the same shape holding invented details (common
names, `example.com`/`.net`/`.org` addresses, telephone numbers whose
subscriber part starts with 0, which no German number does), the same for
one address. The honeypot itself is a link on the Impressum page no person
sees (hidden by CSS, `aria-hidden`, not focusable, disallowed in
robots.txt).

| answer | when |
| --- | --- |
| `403` | an `Origin` that is not one of `FEEDBACK_CONTACT_ORIGINS` from a request that is not same-origin (another site's page; never the Impressum's); on `POST`, a challenge this service did not sign or one changed since |
| `400` | not a solution: malformed, the wrong number of counters, or a counter that does not solve its part |
| `409` | that challenge was answered already (each is taken once) |
| `410` | the challenge expired (the page then asks for a new one, once) |
| `503` | the Impressum is not configured, or the blob file is gone |

Every answer says `Cache-Control: no-store`, `X-Robots-Tag: noindex,
noarchive, nosnippet` and `Vary: Origin`, and names a configured origin in
`Access-Control-Allow-Origin` (a page tried locally, as for the contact
form); the preflight is the contact form's. Memory only, never written down
or logged: challenges per address for an hour (an IPv6 address by its /64),
the honeypot's marks for 24 hours, answered challenges until they expire. The
log says that the honeypot was asked and how often since the start, never
by whom. Nothing bans an address at the firewall: the owner decided
against fail2ban (13.10.2026), because a ban is a hard block of a possibly
shared address, and the site keeps no access log for one to read.

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

`GET /reports?…` lists newest first (`limit` 50, at most 200) under the
filters of §"Triage" (`crate::triage::Filter`): `status`, `kind` and
`priority` take several values, comma-separated; `q` finds the words of the
text or of a crash's message, a piece of the text (`%` and `_` literal) or
a piece of a report's id; `from` and `to` are days, `YYYY-MM-DD` in UTC,
both inclusive; `has_record` is `true` or `false`; `channel` is `gateway`
or `direct`. A report in the trash is listed only under `trash=true`. A
malformed filter is `400`.

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
    "issue_url": "https://github.com/AceVik/baylee/issues/311",
    "assignee": "viktor", "priority": "high", "labels": ["lands", "ui"],
    "duplicate_of": null, "signature": null, "revision": 4,
    "done_at": null, "deleted_at": null, "seen": false, "notes": 1
  }]
}
```

The triage fields are §"Triage"'s; over the token routes `seen` is always
`false`, and no note is shown.

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
The token's `DELETE` is final, wherever the report is: the UI's moves it
into the trash (§"Triage"). Each change and each deletion is written to the
audit (`feedback_audit`) under the actor `token`.

## Triage

What an admin decides about a report, beside its status
(`crates/baylee-feedback/src/triage.rs`, migration `m0007_triage`):

- **Assignee:** the admin who looks after it, by name, or nobody.
- **Priority:** `critical`, `high`, `normal`, `low`, or none.
- **Labels:** at most 16, each lowercase letters, digits and dashes, 1–32
  characters; kept sorted, once each.
- **Duplicate of:** the report it duplicates. Marking one sets the status
  `duplicate` and unmarking it `triaged` (unless the change names
  another); leaving that status clears it. The original is never a duplicate itself: one level only, and
  a report that becomes a duplicate hands its own duplicates on to its
  original, so no chain or cycle forms.
- **Revision:** every change moves it on. A change may name the revision
  it was made against (`expected_revision`); another one is `409`, so two
  admins never overwrite each other unseen. A change that changes nothing
  writes nothing: no revision, no audit line.
- **Done:** `done_at` is set when a report enters `resolved`, `wont_fix` or
  `duplicate` and cleared when it leaves them.
- **Signature:** a crash's message with its numbers taken out and its file
  without line numbers, hashed (MD5, a grouping key, nothing secret), so the
  same crash is one group (`GET /ui/api/crashes`).
- **The trash:** deleting in the UI sets `deleted_at`; the report leaves
  every list but the trash, can be restored, and is purged from there by an
  admin or by itself after **30 days** (`triage::TRASH_DAYS`, swept before
  the list is read; the audit names the actor `sweep`).
- **Notes:** internal, admins only, 1–4000 characters, each with its author
  and time; only its author removes one. They are rows of their own, not
  audit lines, and go with the report when it is purged.
- **Opened:** which reports each admin has opened (`feedback_seen`), for the
  unread marker and `unseen=true`.
- **Saved views:** an admin's named lists (at most 50), a name and the list's
  query string, checked as the list would read it.

The list's filters beyond §"Reading": `assignee` (a name, or `none`),
`label`, `signature`, `duplicate_of`, `trash`, `unseen` (the UI only),
`sort` (`received`, `changed`, `priority`, `status`, `kind`, `reporter`,
`gateway`) and `dir` (`desc`, `asc`).

## The web UI (#311)

`https://feedback.baylee.app/` is a page for admins to triage reports
on: a filtered, paged list, one report with everything it carries, and
§"Triage": status, priority, assignee, labels, duplicates, notes, its
GitHub issue, and the trash. The page is built from `web/feedback`
(React, TypeScript; `web/feedback/README.md`) into static files the service
serves itself from `FEEDBACK_WEB_DIR`. Unset, the service serves no page and
`/` stays `404`; a directory without an `index.html` refuses to start.

What the list does (the README and the `?` key have the keys): views
(inbox, open, mine, unassigned, done, all, trash) with their counts;
search, several statuses, kinds and priorities, assignee, label and the
other filters in the URL, each counted under the rest and shown as a pill;
sortable columns, page numbers and sizes; saved views kept on the service;
crash groups; a drawer that opens a report beside the list; a selection
changed in one request (`POST /ui/api/reports/bulk`: all of it or none of
it, an audit row per report and part); the trash with an undo; a quiet
refresh every 30 seconds that announces new reports instead of moving the
rows; an export of the selection made in the browser, or of all that
matches (at most 5000) by the service; and an unread marker per admin,
kept by the service. One report's page steps to the previous and next of
the list it was opened from (`j`, `k`).
**A reporter is never resolved to an account** (the sender of a reported
chat message is, above). The pseudonym is an HMAC
the gateway makes so that this service cannot know who wrote a report
(§"The gateway" above, `docs/privacy.md`); asking the gateway's console to
turn it back into a name would undo that promise in one request, so the UI
does not, and the gateway's console has no such route. The UI shows an
alias derived in the browser from the pseudonym itself (two words, a hue,
its first four characters; `web/feedback/src/reports/alias.ts`), which is
stable per reporter and per gateway and says nothing more than the hex
does.

**Its words** are Project Fluent files, one per language,
`web/feedback/locales/<lang>/feedback.ftl` (English and German today). The
page speaks the browser's language when it has a file for it, else English;
the sign-in form is English only. English decides which messages exist and
which arguments each takes, and the UI's tests refuse a language that lacks
a message or reads other arguments; `docs/translations.md` has the rules and
the German glossary, `web/feedback/README.md` §"Its words" how the UI reads
the files.

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
| `GET /ui/api/reports.csv?…` | the list as CSV, at most 5000 rows, the `client` part left out but its build; a cell that would start a formula is prefixed with `'` |
| `GET /ui/api/reports/{id}` | one report with `client` and `note_list` |
| `GET /ui/api/reports/{id}/record` | the record, as a download |
| `GET /ui/api/reports/{id}/audit` | `[{"at", "actor", "action", "detail"}]`, oldest first; kept after the report is purged |
| `GET /ui/api/reports/{id}/neighbours?…` | `{"previous", "next", "position", "total"}` of the report in the list the filter makes; `position` `null` when it is not in it |
| `PATCH /ui/api/reports/{id}` | `{"status"?, "issue"?, "assignee"?, "priority"?, "labels"?, "duplicate_of"?, "expected_revision"?}`: a part left out stays, `null` clears it, `labels` is the whole set; `409` on another revision; answers the report |
| `DELETE /ui/api/reports/{id}` | into the trash, `204` |
| `POST /ui/api/reports/{id}/restore`, `…/purge` | out of the trash, or deleted for good from it; `404` where it is not |
| `POST /ui/api/reports/{id}/notes` `{"text"}`, `DELETE …/notes/{note}` | a note added, or removed by its author; both answer the notes |
| `POST /ui/api/reports/bulk` | `{"ids": [≤ 200], "change": {…}}` (as `PATCH`, with `labels_add` and `labels_remove` for `labels`, no revision) or `{"ids", "action": "trash" \| "restore" \| "purge"}`; one transaction; `{"changed"}` |
| `POST /ui/api/seen` | `{"ids": [≤ 500], "seen"}`: opened by this admin, or not; `204` |
| `GET /ui/api/facets?…` | `{statuses, kinds, gateways, reporters, assignees, priorities, labels, channels, total}`, each `[{"value","count"}]` under the filter but its own dimension (`value` `null` for nobody assigned or no priority); the 50 busiest reporters |
| `GET /ui/api/crashes?…` | the crashes the filter matches by signature: `[{"signature", "count", "open", "first", "last", "latest", "message", "location"}]`, at most 100 |
| `GET /ui/api/views`, `POST` `{"name", "query"}`, `DELETE /ui/api/views/{id}` | this admin's saved views; a name saved again replaces its query |
| `GET /ui/api/admins` | every admin's name, for assigning |
| `GET /ui/api/stats` | `{"days": 30, "rows": [{"day", "kind", "count"}]}`: reports per UTC day and kind over the last 30 days, by day then kind, for the overview's chart of bugs and crashes; counts only |
| `POST /ui/api/login`, `POST /ui/api/logout` | above |
| `/ui/api/admin/…` | the admin console, below |
| `/ui/api/contact…` | the landing page's contact messages, above |

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
headline numbers, a month of days as charts, the reports' bugs and crashes
among them, the open tables, every count, and whether the gateway and this
service answer, with their builds), **Live** (`/admin/live`: who is online
and where, every waiting and running table with its chairs, the agents,
and the server's last hour: CPU, memory, disk, network, requests, sockets,
uptime and each engine process on that machine, as `GET /admin/metrics`
samples it, `docs/protocol.md` §"The admin console"; every 5 seconds),
**Sets** (`/admin/sets`: how far the pool is through each set in release
order, from the gateway's ledger and compiled pool; `/admin/sets/{code}`:
one set's cards, each a chip to its Scryfall page with its picture on
hover from Scryfall's host, as a report's card chips are, `docs/legal.md`
§3), **Accounts** (`/admin/accounts`: searched, filtered and sorted in the
query string; `/admin/accounts/{id}`: one account, its decks and latest
games), **Blocked cards** (`/admin/cards`: a card of the gateway's pool
found by name and blocked without an end or until a moment, with a reason
the players are told; the blocks in force, each lifted; the earlier ones;
`docs/protocol.md` §"Blocked cards"), **Beta keys** (`/admin/keys`) and
**Baylee Central** (`/admin/central`: the gateway's link to Central, a token
made on baylee.app pasted and removed, `docs/central-login.md` §"Linking a
gateway with a token"), so the owner needs a shell for none of it.
A sidebar on a wide screen, a tab bar at the bottom of a phone. The
service passes each request on to the gateway's admin console
(`docs/protocol.md` §"The admin console") with the gateway's token, which
the browser never sees:

| route | what |
| --- | --- |
| `GET /ui/api/admin/stats` | the gateway's `GET /admin/stats`, as it answers |
| `GET /ui/api/admin/central` | the gateway's `GET /admin/central`: its link to Baylee Central, never more of a token than its last four characters |
| `PUT /ui/api/admin/central/token` | `{"token"}`, read into that field and written out again: the gateway checks the token with Central, keeps it and uses it; an audit row without the token |
| `DELETE /ui/api/admin/central/token` | the gateway forgets its pasted token and is unlinked |
| `GET /ui/api/admin/live` | the gateway's `GET /admin/live` |
| `GET /ui/api/admin/metrics` | the gateway's `GET /admin/metrics`: the last hour of server samples (`docs/protocol.md` §"The admin console") |
| `GET /ui/api/admin/sets` | the gateway's `GET /admin/sets`: how far the pool is through each set |
| `GET /ui/api/admin/sets/{code}` | the gateway's `GET /admin/sets/{code}`, the code lower-cased; `400` unless it is one to eight letters and digits |
| `GET /ui/api/admin/accounts` | the gateway's `GET /admin/accounts`; the query is read into `q`, `tag`, `kind`, `sort`, `online`, `offset`, `limit` and written out again (anything else `400`); `tag` is one exact tag, a chat report's sender |
| `GET /ui/api/admin/accounts/{id}` | the gateway's `GET /admin/accounts/{id}`; `400` for an id that is not a UUID |
| `DELETE /ui/api/admin/accounts/{id}` | the gateway's `DELETE /admin/accounts/{id}`: the account deleted as its player's own deletion does (beta.8) |
| `PUT /ui/api/admin/accounts/{id}/ban` | `{"reason", "until"?}`, read into those two fields and written out again; `DELETE` the same path lifts the ban (beta.8) |
| `POST /ui/api/admin/games/{id}/close` | `{"reason"}`: the gateway ends the running game through its engine; `202` (beta.8) |
| `GET /ui/api/admin/invites` | its keys, newest first; never a key |
| `POST /ui/api/admin/invites` | `{"count"?, "uses"?, "expires"?, "note"?}` as `invite create` takes them (`"30d"`, `"12h"`); `201 {"keys": [{"id", "key"}], "uses", "expires_at", "note"}`, the only time a key is shown |
| `DELETE /ui/api/admin/invites/{id}` | revokes one: `204`, or `404` for no key not already revoked |
| `GET /ui/api/admin/cards/search?q=` | the gateway's `GET /admin/cards/search`; `q` is read and written out again (anything else `400`) |
| `GET /ui/api/admin/cards/blocks` | every card block, newest first, as the gateway lists them |
| `POST /ui/api/admin/cards/blocks` | `{"index", "reason", "until"?}` (`until` RFC 3339; absent for no end); `201` with the block; `409` when the card is blocked already |
| `DELETE /ui/api/admin/cards/blocks/{id}` | lifts one: `204`, or `404` for no block still in force |
| `GET /ui/api/admin/audit` | the 50 latest changes made here, `[{"at", "actor", "action", "detail"}]`, newest first |
| `GET /ui/api/admin/auth-mode` | the gateway's `GET /admin/auth-mode`: who it lets in, where that comes from, whether it is connected to Baylee Central |
| `PUT /ui/api/admin/auth-mode` | `{"mode"}`, one of `guests`, `central`, `mixed`, read and written out again (anything else `400`); the gateway's answer as it is, `409` while its `BAYLEE_AUTH_MODE` sets the mode; audited as `gateway.auth_mode.set` with the mode |

- **Who.** A live session on every route (`401` without), and on those
  that change something the CSRF rule above too (`403`). The read and admin
  tokens open none of them.
- **Configured by** `FEEDBACK_GATEWAY_ADMIN_URL` (`http://127.0.0.1:28767`)
  and `FEEDBACK_GATEWAY_ADMIN_TOKEN` (the gateway's `BAYLEE_ADMIN_TOKEN`),
  both or neither, else the service refuses to start; the token has 32
  characters at least and is none of the service's own tokens or its direct
  key. Unset, the routes answer `404` and the Overview tab is not drawn.
- **What reaches the gateway.** The service builds every request itself: a
  body is read into the fields the gateway takes and written out again
  (anything else is `400` here), an id must be a UUID, and only `Authorization` (the
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
  expiry; never a key or a note; `gateway.card.block` with the card's name
  and index, the end and the block's id, or `gateway.card.unblock` with the
  id; never the reason) and an `info` line in the log; the gateway logs its
  own line too. A key's note and a block's reason may name a person, so
  they are kept only at the gateway. The acts on an account or a game
  (beta.8, `crates/baylee-feedback/src/moderation.rs`) are rows too:
  `gateway.account.password.reset`, `gateway.account.ban` (with the ban's
  end), `gateway.account.unban`, `gateway.account.delete` and
  `gateway.game.close`, each with the account's or game's id; never the
  password, nor a ban's or game's reason, which may name a person and is
  kept only on the account row (the ban's) or nowhere (the game's).
- **The page.** Mobile first (one column on a phone, 44 px targets, safe-area
  insets, a sticky bar that lifts off the page as it scrolls), light and dark
  as the system says, English or German as the browser asks, WCAG 2.1 AA
  (its design tokens and how they are held to it: `web/feedback/README.md`
  §"Its look"). The numbers are
  asked again every 15 seconds while the page is visible and not while it
  is hidden. New keys are shown once with Copy, Copy all and, where the
  browser has it, Share; there is no invite link, because the client takes
  a key only typed into its sign-up form. Revoking asks once more, and so
  do blocking and unblocking a card.
- **Acting on an account or a game** (beta.8, `src/admin/Moderation.tsx`).
  An account's page has *Act on this account*: **Reset password**
  (registered accounts only; asks first, then shows the new password once
  in a dialog with Copy and says it is not shown again), **Ban** (a reason,
  shown to the player, and an end: a moment in the browser's time zone,
  sent in UTC, or permanently; asks in its form) or **Lift ban** while one
  holds (asks first), and **Delete account** (its handle typed out before
  the button wakes; then back to the list). The page shows a ban in force
  with its end, its admin and its reason, and a reset still to be finished.
  A running table on Live has **End game** (a reason, prefilled, shown to
  the players; asks in its dialog). Every dialog is the browser's own
  `<dialog>`: focus held inside, Escape cancels.

#### Hosted models (`/ui/api/admin/llm/…`)

The **Models** pages: which seat agents are connected, which profiles each
runs, their state, games, spend against caps, key kept or not; and the
changes, passed on to the gateway's `/admin/llm/…` (`docs/protocol.md`
§"Hosted language-model seats"), each as the gateway answers it:

| route | gateway route |
| --- | --- |
| `GET /ui/api/admin/llm/seathosts` | `GET /admin/llm/seathosts` |
| `GET /ui/api/admin/llm/profiles` | `GET /admin/llm/profiles` |
| `PUT /ui/api/admin/llm/seathosts/{host}/profiles/{id}` | `PUT /admin/llm/seathosts/{host}/profiles/{id}`; the body is read into a definition (`label`, `vendor`, `enabled`, `max_games`, `caps`, `canary`, `fallback`, `profile`) and written out again; a `fallback` that is not 1–64 of `A-Za-z0-9._-` is `400` here, every other refusal is the seat agent's sentence, passed back |
| `DELETE /ui/api/admin/llm/seathosts/{host}/profiles/{id}` | `DELETE …` |
| `POST /ui/api/admin/llm/seathosts/{host}/profiles/{id}/enabled` | `{"enabled"}` |
| `POST /ui/api/admin/llm/seathosts/{host}/profiles/{id}/probe` | — |
| `POST /ui/api/admin/llm/seathosts/{host}/profiles/{id}/key` | `{"key": "…" \| null}`; write-only |

`{host}` and `{id}` must be 1–64 of `A-Za-z0-9._-` (`400` otherwise). The
key is held in this service's memory for the request only: it is in no
row, no log line, and an error naming it is never passed back; the page
shows only `key: kept/absent` from the listing. Every change that
succeeded is a `feedback_audit` row, action `gateway.llm.profile.write`,
`.delete`, `.enable`, `.disable`, `.probe`, `gateway.llm.key.set` or
`.delete`, detail `<host>/<id>` and nothing else.

The page (`/admin/models`) asks again every 5 seconds while visible: the
seat agents (connected, local, load against capacity), each profile once
with a row per seat agent holding it (state with "until", games, spend
against caps, key, last error, the fallback it names and, while new games
start there, "new games on …"), its actions (switch, probe, edit, key,
delete), an assignment grid (ticking copies a definition to a seat agent
with `PUT`, never a key; unticking deletes there), and the `gateway.llm.*`
audit rows. Deleting, unassigning and forgetting a key ask once more. The
key field is a password field, cleared as it is sent.

The editor (add or edit a profile) has a box with a one-line hint for
every value an admin sets, in four sections: the profile (label, vendor,
enabled, canary); games and fallback (`max_games`, from which new games
start on the fallback, and the fallback profile: none, or an API profile
of the same seat agent and vendor; an agent CLI takes none,
`docs/protocol.md` §"A fallback profile"); the budget per game
(`game_usd`, `game_tokens`, `game_calls`, the price in and out, which are
fields of the profile and are written into it); and the four caps of the
profile's spend book. The rest of the profile (provider, model, address,
answer mode, effort, …) stays a JSON box in the settings file's format,
shown without the fields the boxes own. Save asks once more ("games already
running keep what they started with"), and a refusal of the seat agent
(`definition_fault`, `definitions_fault`) stands in the editor in its own
words. Keys stay write-only, through *Key…*.

**Who may seat each model** (owner, 13.10.2026; `docs/protocol.md` §"Who
may seat which model"). Per profile id a policy, `registered`, `listed` or
`nobody`, the default mark, and the list of accounts a `listed` profile
admits:

| route | gateway route |
| --- | --- |
| `GET /ui/api/admin/llm/access` | `GET /admin/llm/access` |
| `GET /ui/api/admin/llm/access/{id}` | `GET /admin/llm/access/{id}` |
| `PUT /ui/api/admin/llm/access/{id}` | `{"policy", "default_for_registered"?}`, read into those two fields and written out again |
| `POST /ui/api/admin/llm/access/{id}/allow` | `{"accounts": [<uuid>…]}`, 1–200 UUIDs, read and written out again |
| `POST /ui/api/admin/llm/access/{id}/disallow` | the same |
| `GET /ui/api/admin/accounts/lookup?q=` | `GET /admin/accounts/lookup?q=`: the list editor's type-ahead |

Each change wants the session and the CSRF proof, as every change here
does. A change that succeeded is a `feedback_audit` row,
`gateway.llm.access.policy` with the detail `<id>: <policy>` (and whether
it became or stopped being the default), or `gateway.llm.access.allow` /
`.disallow` with `<id>: <account ids>`; never a username, display name or
handle. The page's panel lists every profile a seat agent reports or a
policy names, with its policy (`not set: nobody` for a profile no admin has
opened), how many accounts are on its list, the default mark, and whether a
seat agent reports it now. *Manage access* opens a dialog: the policy as
three choices, the default mark (which selects `registered`), and the
list: a type-ahead (two characters at least, asked 250 ms after the last
key, ↑/↓ and Enter, at most ten registered accounts, those already on the
list shown and not pickable), the accounts about to be added and taken off
as chips, the list itself filtered, ordered by name or by when they were
added, and taken off one by one or all selected at once. *Save…* shows
what will change and asks before anything is sent; a change stops no
game and no chair already ordered.

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
- **A contact message** from the landing page is kept as typed (the name
  and the address only if given), with its language and when it came,
  until 90 days after an admin marked it done, a year after it came at the
  latest, or until an admin deletes it. Its audit rows (who changed or
  deleted it, when) hold neither its text nor its address and are kept.
  The addresses its allowance counts are in memory for an hour.
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
- **For triage** it keeps on each report the assignee's name, a priority,
  labels, the report it duplicates and when it was done or put into the
  trash; internal notes (author, time, text) with the report until it is
  purged; each admin's saved views and which reports they opened until the
  admin is removed. A report in the trash is purged 30 days after it went
  in. Notes are about the report: no personal data, no quotes from it, as
  the UI says above the note field.

## Running it

`scripts/server/baylee-feedback.service` is the unit, built and installed by
`baylee-deploy` (crate baylee-ops) beside the gateway. It reads
`/etc/baylee/feedback.env`:

```sh
FEEDBACK_DATABASE_URL=postgres://baylee_feedback:…@127.0.0.1:5432/baylee_feedback
FEEDBACK_GATEWAY_TOKENS=eu=<openssl rand -hex 32>
FEEDBACK_READ_TOKEN=<openssl rand -hex 32>
FEEDBACK_ADMIN_TOKEN=<openssl rand -hex 32>
FEEDBACK_WEB_DIR=/opt/baylee/web/feedback
FEEDBACK_TRUSTED_PROXIES=127.0.0.1
FEEDBACK_DIRECT_KEY=<openssl rand -hex 32>   # only to take reports straight from clients
FEEDBACK_CONTACT_ORIGINS=https://gateway.baylee.app   # only to take the landing page's contact form; list a host the page is still served at as an alias too
FEEDBACK_IMPRESSUM_KEY=<openssl rand -hex 32>       # only to hand out the Impressum (with the next)
FEEDBACK_IMPRESSUM_BLOB=/etc/baylee/site-private/impressum.bin
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

The contact form needs `POST /site/contact` on the public site too (the
same Caddy file) and the landing page's CSP naming the service
(`scripts/server/site.caddy`, `connect-src`).

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
