# Environment variables

The optional workstation dashboard `baylee-monitor` has no service-specific
environment variables; its CLI flags and user unit are described in
[`../workstation-monitor.md`](../workstation-monitor.md).

Every variable a Baylee process reads, by process. A value outside its range
refuses startup where this says so; the normative behaviour behind each is in
the document it names.

## Neural player and offline evaluation (`baylee-train`)

- `BAYLEE_EP=cuda` selects the v4 ONNX CUDA provider; unset or `cpu` uses CPU.
  Build with `--features onnx-cuda` and provide the CUDA 13 / cuDNN 9 libraries
  on `LD_LIBRARY_PATH`. A v4 binary without that feature refuses `cuda`, and
  provider initialization failures are errors. TF32 is disabled to preserve
  agreement with the FP32 rollout probability audit. This selects inference;
  Python training chooses its PyTorch device separately.
  With the locked trainer environment, the Linux library directories are
  `tools/trainer/.venv/lib/python3.13/site-packages/nvidia/cu13/lib` and
  `tools/trainer/.venv/lib/python3.13/site-packages/nvidia/cudnn/lib` (resolve
  them to absolute paths for a launch; use the actual Python minor version).
  PyTorch 2.15 no longer installs cuRAND itself; the trainer explicitly depends
  on `cuda-toolkit[curand]` because the ONNX provider requires `libcurand.so.10`.
- `train_ops pipeline --manifest <path>` specifies every stage as a literal
  argument array, including its evaluation binary and `eval4 --threads` limit.
  Each ONNX session uses one CPU
  intra-op thread. Measure whole games before increasing worker counts.
- `BAYLEE_TRAIN_OPS` selects the native `train_ops` executable used by PyTorch
  checkpoint I/O, receipts and policy admission. Otherwise the adapter checks
  `PATH` and this checkout's `target/release`, then `target/debug`. It has no
  Python fallback for hashing, orchestration or admission.
- `llm_collector4 collect --admin-url http://127.0.0.1:<port>` requires
  `BAYLEE_COLLECTOR_ADMIN_TOKEN`, read once into memory and never passed to
  children. The URL must be a literal loopback IP and port; redirects and
  proxies are disabled. Authentication is checked before collecting games.
  This is the dedicated collection gateway's admin credential, used only to
  abort the exact owned game after failed attempts. A separately verified
  closure does not admit the failed record or manufacture a game outcome.
  `llm_collector4 serve --admin-bind 127.0.0.1:<port>` reuses an explicitly
  supplied `BAYLEE_COLLECTOR_ADMIN_TOKEN`, or generates a random token in memory
  when unset, and passes it only to its gateway child. No token is
  written to status files, logs or command arguments; the admin listener is
  off unless this bind is supplied.

## Gateway (`baylee-gateway`)

- `PORT` (`0` binds any free port; `BAYLEE_PORT_FILE`, a path, receives the bound port just before serving, which is how e2e helpers find it, #279).
- `DATABASE_URL` (required).
- `BAYLEE_DB_POOL` (8).
- `STORE_PATH` (import-only, not gitignored).
- `BAYLEE_ART_PATH`/`BAYLEE_DECK_IMAGE_PATH` (`off` disables; empty also refuses uploads; `/art` answers signed-in players only and the wasm client takes art straight from Scryfall, `docs/protocol.md` §"Card art").
- `BAYLEE_REGISTRATION` (`off`: nobody new; `invite`: a closed beta, where a new guest needs a key, a returning guest none; `docs/protocol.md` §"A closed beta: keys (#317)"; who a Central account new here may be, `docs/central-login.md`).
- `BAYLEE_AUTH_MODE` (whom the gateway lets in: `guests`, `central` or `mixed`; set, it wins over the console's choice and locks it, unset the console's choice or the default `central` holds; any other value stops it; a mode without guests wins over a key; replaces `BAYLEE_GUESTS`, which is read no more; `docs/central-login.md` §"Who a gateway lets in").
- `BAYLEE_GUEST_CAP` (live guests, default 1000, `0` = no cap; guests sign in with a display name only, are purged when their last session lapses, 30-day sliding, and may not upload images).
- `BAYLEE_TRUSTED_PROXIES`.
- `BAYLEE_AGENT_TOKEN` (none = no agent).
- `BAYLEE_PUBLIC_URL` (what the gateway calls its own address in a bug report). The gateway sends no mail since the cutover; `BAYLEE_SMTP_URL` and `BAYLEE_MAIL_FROM` are not read.
- `BAYLEE_ENGINE_URL` (set for a remote agent).
- `BAYLEE_UNIX_SOCKET` (a path: also serve every route on a unix socket, `0660`, a stale file removed; an agent on it is local, and so are its games, `/health` `games.local_running`; `docs/protocol.md` §"On the same machine: the unix socket").
- `BAYLEE_GATEWAY_NAME` (what `GET /info` names the gateway; unset = the client shows the address; over 64 characters or a control/bidi character refuses startup).
- `BAYLEE_SOURCE_URL` (what `/info` and `/source` name as this build's source, AGPL §13; unset = the repository; not `http(s)://`, over 200 characters, or a whitespace/control/bidi character refuses startup).
- `BAYLEE_IMPRESSUM_URL` (the operator's legal notice, `/info` `impressum`, `null` when unset; the client links it with its host on the front door's About, the terms sheet and Settings → Network; not `https://` with a host, over 200 characters, or a whitespace/control/bidi character refuses startup; no privacy URL, the terms carry the privacy notice).
- `BAYLEE_FEEDBACK_URL`/`BAYLEE_FEEDBACK_TOKEN`/`BAYLEE_FEEDBACK_KEY` (`POST /reports` forwards to that service; unset URL = 503; the key makes the reporter pseudonym, unset = derived from the token; `docs/feedback.md`).
- `BAYLEE_DIRECT_REPORT_KEY`: turns on `POST /client/reports` on the official gateway (with `BAYLEE_REPORTS_TO=central` and the bridge) and keys its device pseudonyms; at least 16 characters, else startup refuses; the feedback service's `FEEDBACK_DIRECT_KEY` value keeps a device's reports linkable (`docs/feedback.md`).
- `BAYLEE_REPORTS_TO`: `feedback` (default) or `central`, which files `POST /reports` with Baylee Central under the gateway's Central link instead (a reported chat message still goes to the feedback service); `central` without `BAYLEE_FEEDBACK_KEY` or `BAYLEE_FEEDBACK_TOKEN` refuses startup; any other value refuses startup (`docs/feedback.md` §"To Baylee Central instead").
- `BAYLEE_RECONNECT_SECS` (the reconnect window a room gets when it names none, default 180, `10..=3600`).
- `BAYLEE_REPLAY_JOBS` (replay jobs at once, default 4, `0..=64`, `0` = no replays: `/replays` routes `503`, `/info.replays` `null`; `docs/protocol.md` §"Replays").
- `BAYLEE_REPLAY_JOB_SECS` (how long a replay job may take, default 60, `1..=600`, past it `504`).
- `BAYLEE_ADMISSION_HOLD` (an absolute path; while that file exists no new game starts on any agent, `503`, running games go on, `/health` `admission`; a deploy with deploy hooks places it, `docs/deploy-hooks.md`, `docs/protocol.md` §"Holding admission during a deploy").
- `BAYLEE_ROOM_GRACE_SECS` (how long a waiting room keeps the chair of a player whose sockets all closed, default 60, `1..=600`).
- `BAYLEE_WS_TICKET_SECS` (a socket ticket's life, default 45, `1..=600`, anything else refuses startup).
- `BAYLEE_CHAIR_TICKETS=off` (no host may hand a chair to a seat bridge: ticket and redemption `403`).
- `BAYLEE_CHAIR_TICKET_SECS` (a host's chair ticket for its seat bridge, default 120, `1..=600`; the delegate must be named `LLM-…`; the bridge sits on it with no account, so guests off or a closed beta still seat it; `docs/protocol.md` §"A host's chair for a seat bridge").
- `BAYLEE_WS_LEGACY_TOKENS=off` (refuse the old `?token=` on sockets before its end, 2026-10-31; `docs/protocol.md` §"Opening a socket: tickets").
- `BAYLEE_TERMS_PATH` (a UTF-8 Markdown file ≤ 64 KiB for every language, or a directory of at most four `terms.<lang>.md` such files, `terms.en.md` required as the fallback, all naming one `<!-- version: … -->`; unset = the default terms every gateway ships, `docs/gateway-legal/`; `off` = none and nothing changes; a version an admin publishes in the console, `POST /admin/terms`, wins over both; `GET /terms?lang=` carries the version's `digest`; a Central account's acceptance lives on Central and a Central-bound session that has not accepted gets `403` `terms_required` on all but `GET /terms`, sign-out and `DELETE /account`; e2e helpers set `off`; unreadable, oversize, empty, no `en`, two versions, `draft` or a control character refuses startup; `docs/protocol.md` §"Terms of use (WG-1)"; `docs/terms-placeholder.md` and `docs/terms-placeholder/` are not legal text).
- `BAYLEE_ADMIN_TOKEN` (≥ 32 characters, none of the gateway's other secrets; unset = no admin console: `GET /admin/stats` (counts only), `GET/POST /admin/invites`, `DELETE /admin/invites/{id}`, as `invite create|list|revoke`).
- `BAYLEE_ADMIN_BIND` (`127.0.0.1:28767`; loopback only, else startup refuses; never on the public port or the unix socket; `BAYLEE_ADMIN_PORT_FILE` as `BAYLEE_PORT_FILE`). The console refuses `Origin`/`Sec-Fetch-*` and a non-loopback `Host` before the token, compares it in constant time, shuts for 5 min after ten wrong ones, wants `X-Baylee-Admin` on a change and logs each change on `baylee_gateway::audit` without a key; `docs/protocol.md` §"The admin console".
- `BAYLEE_CENTRAL_CONFIG_FILE` (the operator's file linking the gateway to Baylee Central: the official gateway's `bridge`, or a registered gateway's `origin`, `credential`, `heartbeat` and `metadata`; read once at start, never logged; it takes precedence over a pasted token; `docs/central-login.md`).
- `BAYLEE_CENTRAL_ENDPOINT` (where a gateway token pasted into the console talks to: `live` by default, or an explicit numeric-loopback development origin such as `http://127.0.0.1:28790`; anything else refuses startup; `docs/central-login.md` §"Linking a gateway with a token").


A `dev-table` gateway reads `BAYLEE_DEV_SEAT_BOARD` (`0:Card;1:Card`, `baylee_cards::decks::deal_named`); a bad spec refuses startup. Keep board seeding behind the feature.

CORS is `Access-Control-Allow-Origin: *` without `Allow-Credentials`: keep bearer-header auth, never set cookies.

## Gateway: friends and chat

`docs/protocol.md` §"Friends and chat (beta.8)", `baylee_protocol::social`, gateway `social/`.

- `BAYLEE_SOCIAL` (`off` = every `/social/…` route and the social ticket `403`, `/info.social` `null`; unset or `on` = on; anything else refuses startup).
- `BAYLEE_SOCIAL_KEEP_DAYS` (days an undelivered message is held, 1–30, default 14, a guest's device always 1; anything else refuses startup).

The gateway carries sealed bodies it cannot open, checks a device statement and a reported message with `baylee-seal`, and logs counts and refusal kinds only. A table's chat members are the accounts in its chairs: AI, seat-bridge and spectator seats never receive chat, and the engine never sees a social frame.

## Gateway and seat agent: hosted model seats

`docs/protocol.md` §"Hosted language-model seats", `docs/llm-seat.md` §"A hosted seat".

Gateway:

- `BAYLEE_SEATHOST_TOKEN` (≥ 32, none of its other secrets; unset = no hosted seat).
- `BAYLEE_SEATHOST_BRIDGE_URL` (where hosted bridges dial, default the loopback port).

Seat agent `baylee-seathost` (`scripts/server/baylee-seathost@.service`, one OS user per instance):

- `BAYLEE_SEATHOST_TOKEN`.
- `BAYLEE_SEATHOST_NAME`.
- `BAYLEE_SEATHOST_STATE` (its `hosted.json`, `spend/<id>.json`).
- `BAYLEE_SEATHOST_CAPACITY`.
- `BAYLEE_SEATHOST_BRIDGE_GATEWAY` (required on `unix:`).
- `BAYLEE_SEAT_BIN`.
- `BAYLEE_KEY_STORE=file:<dir>`.

Registered hosts only; keys write-only over the unix socket; tests use in-process launchers or the fake CLI, never a real model.

## Agent (`baylee-agent`)

- `BAYLEE_GATEWAY` (`unix:<path>` dials the gateway's unix socket and hands engines the same; `baylee_protocol::unix_socket` reads it).
- `BAYLEE_AGENT_TOKEN`.
- `BAYLEE_AGENT_NAME`.
- `BAYLEE_AGENT_CAPACITY` (0 = unlimited).
- `BAYLEE_ENGINE_BIN` (default beside the agent).

## Feedback service (`baylee-feedback`)

`docs/feedback.md`.

- `FEEDBACK_DATABASE_URL` (required, its own database).
- `FEEDBACK_GATEWAY_TOKENS` (`name=token,…`).
- `FEEDBACK_READ_TOKEN`.
- `FEEDBACK_ADMIN_TOKEN` (each ≥ 16 characters, none twice).
- `FEEDBACK_BIND` (`127.0.0.1:28780`).
- `FEEDBACK_POOL` (4).
- `FEEDBACK_WEB_DIR` (the built `web/feedback` UI; unset = `/` is 404).
- `FEEDBACK_TRUSTED_PROXIES` (for the sign-in limiter and the direct allowance).
- `FEEDBACK_DIRECT_KEY` (≥ 16, none of the tokens; unset = `POST /client/reports` is 503).
- `FEEDBACK_CONTACT_ORIGINS` (the landing page's origins, `https://…` or loopback `http://`; unset = `POST /site/contact` is 503; CORS names only these, a filled honeypot is answered 201 and dropped, messages in `feedback_contact` are swept 90 days after done and a year after arrival; `docs/feedback.md` §"The landing page's contact form").
- `FEEDBACK_IMPRESSUM_KEY`/`FEEDBACK_IMPRESSUM_BLOB` (both or neither; the key ≥ 32, none of the tokens, signs proof-of-work challenges; the blob is the file `crates/baylee-tools/src/site/impressum.rs` wrote, server-only, never in the repo; unset = `/site/impressum…` is 503; suspicion only raises the puzzle's difficulty or, beyond doubt or after the honeypot, answers a decoy, never refuses a person; `docs/feedback.md` §"The Impressum").
- `FEEDBACK_GATEWAY_ADMIN_URL`/`FEEDBACK_GATEWAY_ADMIN_TOKEN` (both or neither; the gateway's console and its `BAYLEE_ADMIN_TOKEN`, ≥ 32, none of the service's tokens; `baylee-deploy stage` makes one and writes both env files `0600`): `/ui/api/admin/…` passes a signed-in admin (session, CSRF on writes) on to it, builds each request itself, and answers a refused token `502`, never `401` (`docs/feedback.md` §"The admin console").

The UI's GitHub issue carries only the admin's own summary, a category and the build, never report text or a link back (owner, 08.10.2026). The direct route takes a report from a client signed in nowhere: no credential, a random device id it keeps only as an HMAC, per-address and service-wide hourly allowances in memory, stored `channel = 'direct'`, gateway `(direct)`, its record `record_origin = 'client'`; the only route with `Access-Control-Allow-Origin: *` (`scripts/server/feedback-direct.caddy`). A client's record (either road) is checked for shape only: the service links no engine. Deploy the service before the gateway. The UI (#311) signs in admins made only by `baylee-feedback admin add|remove|list` and talks to `/ui/api/…` with a session cookie (the service's own origin; the gateway's no-cookie rule stays); the token routes are unchanged. `cargo` never needs node: CI's `web-feedback` job and `baylee-deploy` build the UI.

## Engine server (`baylee-engine-server`)

- `--attach/--game/--token` or `BAYLEE_ATTACH_URL`/`BAYLEE_GAME`/`BAYLEE_ENGINE_TOKEN`; none = listening dev harness (`PORT`, `0` and `BAYLEE_PORT_FILE` as on the gateway; `BAYLEE_BIND`). Never bind it publicly: unauthenticated, every hand leaks.
- `BAYLEE_ENGINE_PUMP_STEPS` (attached transport only, default `128`, integer `1..=128`; invalid values refuse launch). `1` publishes each house-pump slice for local neural-seat experiments. A socket action followed by house work can still share one output batch; this does not expose internal automatic actions or guarantee training-equivalent observations for mixed house/socket tables. Two connected socket seats receive their own filtered view after every explicit answer.

## Client (`baylee-client`)

- `BAYLEE_GATEWAY` + `BAYLEE_GAME`, `BAYLEE_SEAT_TOKEN`, optional `BAYLEE_SEAT`; in a browser `?game=…&token=…`.
- At build time `BAYLEE_FEEDBACK_PUBLIC_URL` (`option_env!`) names the feedback service a client signed in nowhere reports to; `feedback_url` in the settings file overrides it (`""` = off); neither = such a client sends nothing. A `LocalHost` game is recorded and the last 20 kept in `records/` beside the settings (native); a report carries one only when ticked for that report, never remembered.

## Seat bridge (`baylee-seat`)

`docs/llm-seat.md`.

- `BAYLEE_SEAT_CONFIG` (the settings file: model profiles and daily/monthly caps; `--config` wins; unset = `llm-seat.json` in the client's config directory; a named file must exist, none at all = no caps, as before).

Keys only from the variable a profile names (`key_env`) or the OS credential store (`baylee-seat key set|status|delete`, service `baylee-seat`, account `{key_env}@{host}`; `BAYLEE_KEY_STORE=off` disables), never in the file (refused); tests use `MemoryKeys` or `BAYLEE_KEY_STORE=off`, never the real store. Under a file each game reserves in `llm-spend.json` beside it (`--ledger`) before it sits down and settles after; a killed bridge's reservation counts in full. Tests pass temp paths, never the real config directory. A host's client starts it with `--tethered --chair-ticket`, the chair ticket the first stdin line (never argv, env or a log); it says ready only after `Mind::check` (the cheapest provider call, a CLI's login check). Lines on paths that can outlive their reader go through `client_core::say!`/`say_err!`, never `println!`. `--mind cli:<tool>[:<model>]` (`provider: cli`) plays an agent CLI on its own login, no key (`cli.rs`, `docs/llm-seat.md` §"A CLI as the model"): the resolved program run with an argv array, never a shell; an empty `0700` temp working directory; `env_clear()` plus an allowlist, a key-shaped value refusing the start. Tests drive only `examples/fake-agent-cli.rs`, never a real CLI or model.

## macOS CI runner (`baylee-macos-runner`)

`docs/releasing.md` §"The macOS runner". Read by `serve`, `batch` and
`prepare`; all four are optional and exist for testing the runner elsewhere.

- `BAYLEE_RUNNER_REPO`: the repository it serves (default `AceVik/baylee`).
- `BAYLEE_RUNNER_IMAGE`: the local Tart image each job clones (default `baylee-runner`).
- `BAYLEE_RUNNER_BASE`: the upstream image `prepare` clones (default: `Settings::base`, pinned by digest).
- `BAYLEE_CARGO_LOCK`: the lock command it holds while jobs are queued (default `~/.baylee-locks/with-cargo-lock.sh`).

The fine-grained token is never a variable: it is read from the login
keychain (service `baylee-macos-runner`) and handed to each `gh` child alone.
