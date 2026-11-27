# The language-model seat: settings and spend

The seat bridge (`baylee-seat join`, `crates/baylee-seat`) sits at a table
as an ordinary socket player and hands its questions to a mind: the house,
a script, or a language model, behind an API or behind an agent CLI that a
subscription is signed in to. This page is about the language model: the
settings file that says which model plays and what it may spend, and the
spend book that holds a player's caps across games. The types are
`baylee_client_core::llmseat` (pure, every target), the files are
`llmseat::store` and `llmseat::ledger::Book` (native only), and the bridge
applies them in `baylee_seat::config` and `baylee_seat::spend`. What a
decision tells the model and what its answer may say (plans, `until`,
`react`, the deck as the provider reads it, the cache) is
`docs/llm-protocol.md`.

Both hosted and local language-model seats can accept or reject another
player's draw offer. The offer is a fresh decision with explicit yes/no
options, interrupting queued card choices. It never counts as conceding.

**No player is named to a model** (owner, 13.10.2026). Every bridge, a
hosted one on the operator's machine and a player's own `baylee-seat` on
their device, with every provider (an API or an agent CLI), names the
chairs `P1…Pn` (seat order) and its own seat "you" in everything it sends:
the prompt's prefix says `P2 is an opponent.`, never a display name,
handle, tag or account id; neither the game's id nor the deck's own name
is in a prompt, a CLI's directory name or a managed session's label.
There is no option that names them: `GameContext` carries no names, and
the narrator's roster for the log writes `P2`
(`baylee_seat::narrator`). The people at the table did not choose the
provider, and the terms say so. What the requests carry besides the game
text: the API's own fields (`model`, `messages`/`system`, `max_tokens`
or, for `OpenAI`'s own API, `max_completion_tokens`,
`tools`/`tool_choice`, `response_format`, `reasoning_effort`,
`output_config`, `cache_control`), never an OpenAI `user` or an
Anthropic `metadata`; the headers `content-type`, `anthropic-version`,
the key (`x-api-key` or `authorization`) and, for a managed session, the
`x-session-id` its endpoint issued; the HTTP library's own `user-agent`
(`ureq/<version>`). A CLI gets the argv and environment listed under
[A CLI as the model](#a-cli-as-the-model).

An example file, with no key in it:

```json
{
  "default": "sonnet",
  "caps": {
    "day_usd": 10,
    "month_usd": 60,
    "day_tokens": 20000000,
    "month_tokens": 200000000
  },
  "profiles": {
    "sonnet": {
      "provider": "anthropic",
      "model": "claude-sonnet-5-5",
      "effort": "medium",
      "game_usd": 3,
      "think_secs": 60
    },
    "opus": {
      "provider": "anthropic",
      "model": "claude-opus-5-5",
      "effort": "high",
      "max_tokens": 24000,
      "game_usd": 6
    },
    "deepseek": {
      "provider": "openai",
      "model": "deepseek-chat",
      "base_url": "https://api.deepseek.com/v1",
      "key_env": "DEEPSEEK_API_KEY",
      "answer": "json",
      "game_tokens": 3000000
    },
    "cc-opus": {
      "provider": "cli",
      "model": "claude:opus",
      "effort": "low",
      "game_calls": 400,
      "think_secs": 90
    }
  }
}
```

## The file

`llm-seat.json` in the client's config directory (`client-core::userdirs`,
the directory of `client-settings.json`: `$XDG_CONFIG_HOME/baylee`, else
`~/.config/baylee`; `%APPDATA%\Baylee` on Windows). `--config <file>` names
another, else `BAYLEE_SEAT_CONFIG`; a file named either way must be there,
so a mistyped path never drops the caps. The bridge reads it; the client's
settings panel writes it (`llmseat::store::save`, whole, through a
temporary file, mode `0600` on unix). The client finds it the same way
(`BAYLEE_SEAT_CONFIG`, else the config directory's), and names it to every
bridge it starts (`--config`), so a bridge plays the file the player saw.

Unknown fields are refused, and so is every fault, in one sentence naming
the profile: a file is played as written or not at all.

| Field | Meaning |
|---|---|
| `default` | The profile played when `--profile` names none. |
| `caps.day_usd`, `caps.month_usd` | The most the games of models with a price may spend together per calendar day and per month, in US dollars. |
| `caps.day_tokens`, `caps.month_tokens` | The same in tokens, for the games of models without a price. |
| `profiles.<name>` | Up to 32 letters, digits, `-` and `_`, starting with a letter or digit. |
| `provider` | `anthropic`, `openai` (any OpenAI-compatible endpoint) or `cli` (an agent CLI, [below](#a-cli-as-the-model)). Required. It names a wire protocol, not a vendor: the client calls the first two *Anthropic Messages* and *OpenAI-compatible*, and `base_url` says whose server speaks it (DeepSeek answers both). |
| `model` | The provider's model id; for `cli` the tool, then its own model if any: `claude`, `claude:opus`. Required. |
| `effort` | A word such as `low`, `medium`, `high` (default medium on Anthropic, the endpoint's or the CLI's own elsewhere). |
| `answer` | `tools`, `json` or `json_schema` (the last two for OpenAI-compatible endpoints and CLIs; a CLI answers only these, `json_schema` by default). `json` asks the endpoint for a JSON object (`response_format` `json_object`, which `DeepSeek` takes); `json_schema` for one held to the answer's schema (`json_schema`, for an endpoint that refuses a bare object, such as LM Studio). Either way the model is told the answer's fields in its instructions, and when an endpoint turns the one mode down, the error says to try the other. |
| `session_api` | Optional boolean, disabled by default. OpenAI-compatible servers implementing the managed-session extension only; see below. The same profile works in a local bridge and a hosted seat agent. |
| `max_tokens` | The most one reply may take (default 16000 Anthropic, 8000 OpenAI-compatible). Sent as `max_tokens`, except to `OpenAI`'s own API (`base_url` on `api.openai.com`), whose current models refuse it and take `max_completion_tokens` (`llm::openai::reply_limit`); DeepSeek, Google's OpenAI-compatible endpoint and local servers take `max_tokens`. A CLI takes no such limit: for one it is only what a call is held at for its reply (default 16000). |
| `price` | `{"input": …, "output": …}`, US dollars per million tokens: the price of a model this build has none for, or a better one. It applies to the profile's own model only. A cache write is billed at 1.25× the input price for five minutes' entry and 2× for an hour's (the API path caches the game's constant head for an hour, `docs/llm-protocol.md` §"The cache"), a cache read at the input price unless the build knows better. |
| `game_usd` | The most one game may spend in dollars (default $5); only for a model with a price. |
| `game_tokens` | The most one game may spend in tokens, in and out (default 5,000,000; 20,000,000 for `cli`); the limit of a model without a price. |
| `game_calls` | The most calls one game may make; past it the house finishes the game (default 500 for `cli`, no limit for an API). |
| `think_secs` | The longest one answer may take (default 60). |
| `key_env` | The environment variable the key is read from (default `ANTHROPIC_API_KEY`, or `BAYLEE_LLM_API_KEY` for `openai`), and, with the address's host, the name a key is kept under in this machine's credential store ([below](#where-a-key-is-kept)). Not for `cli`. |
| `base_url` | Where the API is: `https://`, or `http://` on loopback only. Not for `cli`. |
| `command` | `cli` only: the tool's program, a whole path; default: the tool's name on the bridge's `PATH`. Name a version's own file to play that version (`/Users/<you>/.local/share/claude/versions/<version>`, [below](#a-cli-as-the-model)). |
| `lookup` | Optional, `scryfall` or absent (the default). Every seat may look a card up in this build's card text and its gateway's catalog; `scryfall` lets its lookups also bring a card's Scryfall rulings and the text of a card neither knows ([below](#reports-and-lookups)). The settings panel shows no box for it and keeps it as the file has it. |
| `fallback` | Optional: the profiles of this file that take a game over when this model is rate limited, out of its quota or failing, first to last: one name (`"backup"`) or a list (`["opus-api", "deepseek"]`), each behind an API ([below](#a-fallback-profile)). A bridge plays this chain unless `--fallback-profile` names another or `--no-fallback` none; a name a command line cannot carry is refused, and one that is no profile of the file, plays an agent CLI or is the profile itself is left out as the chair sits down. A hosted profile names its fallbacks in its definition instead. The settings panel shows no box for it and keeps it as the file has it. |

A model without a price in this build plays only with a `price` or a
`game_tokens`, as `--spend-tokens` on the command line (`join --help`);
where the caps count dollars a day or a month, it also needs the token cap
of that period, or its spend would count nowhere.

## Managed API sessions (optional)

Set `"session_api": true` only for an OpenAI-compatible endpoint that implements
this extension. In the desktop settings panel it is **Advanced → Managed game
sessions**. Ordinary API profiles remain unchanged; no capability is guessed
from a vendor, model name, port or address.

- Before its first model call, each game/seat creates `POST {base}/sessions`
  with `{"label":"game <random UUID> / seat <number>"}`. The label contains
  no player names, profile names, actual game identifier or clock (a v4
  UUID, not a v7 that carries the time it was made). The response names
  `id` (1–128 ASCII letters/digits, `_`, `-`) and optionally `idle_timeout_secs`.
- The bridge adds `X-Session-Id` to Chat Completions. It still sends the complete
  ordered transcript, including the exact assistant content; the endpoint can
  recognize an extension and reuse context. Unlike ordinary API conversations,
  these continue across game turns until the existing context-size limit rotates
  them. A fresh conversation always carries a full board and prefix.
- While the seat owns the game it sends `POST {base}/sessions/{id}/touch` every
  third of the advertised inactivity interval, at most five minutes apart (five
  minutes when no interval is returned). Heartbeats have a two-second deadline
  and do not count as model calls. A 404/410 on a heartbeat forgets the expired
  id; the next decision creates a new session and sends its complete context.
- At game end, a permanently closed room, model swap or bridge cancellation,
  `DELETE {base}/sessions/{id}` releases that seat's session. Temporary socket
  reconnects keep it. The deletion worker has a two-second deadline; server-side
  inactivity expiry remains necessary for SIGKILL, machine failure and lost
  network responses. An id returned by a creation already in flight at game end
  is still tracked and deleted.

All lifecycle calls use the same address, authentication and no-redirect policy
as the model call. A completion returning 410 is not replayed automatically:
that decision follows the usual failure policy; the next call starts a session.
This also applies when a proxy returns a non-JSON error page. A late expiry
response cannot discard a newer session. If an aborted decision still has a
blocking HTTP call in flight, another decision for that session fails locally
until the old call finishes; different seats continue independently.
Servers may return 202 while their own cleanup continues. Labels and session ids
are never put in the table's public mind declaration, prompts or transcripts.

## Where a key is kept

In the environment or in this machine's credential store, never in the
file. A profile names the variable its key is read from (`key_env`); the
bridge reads that variable first, and where it is unset, the key kept in
the operating system's credential store under the same name for the
profile's address: the macOS Keychain, the Windows Credential Manager, the
Secret Service on Linux. The shared `baylee-keyring` adapter retains the native
Windows/macOS providers and uses MIT-licensed libsecret Rust bindings with the
dynamically linked LGPL-2.1-or-later system library on Linux; entry names remain
compatible. See [native credential storage](native-credential-store.md) for the
dependency inventory, session checks and isolated Linux tests.

- **The entry.** Service `baylee-seat`, account `{key_env}@{host}` (with
  the port where the address names one, `llmseat::keys::KeyEntry`): the
  host of the profile's `base_url`, else of the environment's address,
  else the provider's. A key is kept for one host, so a profile pointed
  somewhere else finds none, and a key is never sent to an address it was
  not kept for. Two profiles of one vendor on one host share a key; two
  protocols of one vendor at two addresses (`api.deepseek.com/v1` and
  `/anthropic` share a host) do too.
- **Keeping one.** The client's settings panel has a key box under each
  profile's address ([below](#the-settings-panel)), and a terminal has
  `baylee-seat key set|status|delete --profile <name>` (or `--key-env
  <variable> --host <host>`). `set` reads the key as one line on stdin,
  never from the command line; `status` answers `set`, `absent` or
  `unavailable: <why>`, and asks the store whether there is a key without
  reading it out. Nothing prints a key, and an error that might quote one
  has every key-shaped run blanked.
- **Who reads it.** The bridge, itself, at the moment it sits down (or, in
  a debug build, takes an order for another profile). The client never
  reads a key back: its box sends a key to `baylee-seat key set` on that
  child's stdin and forgets it, and a bridge it starts is handed no key.
  This is the smaller exposure of the two ways a bridge could get the key:
  a key handed down in the bridge's environment can be read by every
  process of the same user (`/proc/<pid>/environ`, `ps eww`) for the whole
  game and is inherited by every child (a CLI mind's tool included),
  where the store answers only the program that asks, under its own
  access rules, and the key then sits only in the process that sends it.
  It also means one program touches the store, so macOS asks once, for
  `baylee-seat`, rather than for each program that would read the entry.
  A rebuilt (unsigned) bridge may be asked again.
- **Where there is none.** A browser, a phone, a machine without a
  running Secret Service, or `BAYLEE_KEY_STORE=off` (every test that
  starts a bridge sets it: a test never reaches the player's store): the
  key box says why it is not there, and the variable works as before.
  Every test inside a process uses `llmseat::keys::MemoryKeys`.
- **Never the file.** A field named like a key (`api_key`, `key`,
  `token`, `secret`, `password`, `authorization`, …), a profile's name
  included, or a name or value shaped like one refuses the file
  (`llmseat::shaped_like_a_key`: the generic `sk-`, `Bearer ` and
  `x-api-key` where a word starts, so `risk-free` is none; the long
  provider markers `sk-ant-`, `sk-proj-`, `ghp_`/`gho_`/`ghu_`/`ghs_`/`ghr_`
  and `github_pat_` with twenty key characters after them wherever they
  stand, so a key glued to a model id is one) with a sentence saying
  where keys go, and so does a command line that carries the key of any
  variable the bridge would read. The settings panel refuses a key typed
  into any box but the key box, beside that box, and writes nothing.
- **Its address.** A profile's key goes only to the address written
  beside it: a profile's `base_url` beats `ANTHROPIC_BASE_URL` and
  `BAYLEE_LLM_BASE_URL`, and without one the environment's address, then
  the provider's, is used.

## Which model plays, with what

A flag beats the profile, and the profile beats the build.

- The profile is the one `--profile <name>` names, else the file's
  `default`. `--profile` with no file, or a name the file lacks, is
  refused, and so is `--profile` beside `--mind house` or `scripted`.
- `--mind anthropic[:<model>]`, `openai:<model>` or `cli:<tool>[:<model>]`
  puts its model in the profile's place and keeps the profile's other
  settings; a named profile of another provider is refused, the default
  one is left out with a note, and the model plays with the build's
  defaults.
- Nothing named and no default: the house plays.
- `--effort`, `--answer`, `--max-tokens`, `--price-in`/`--price-out`,
  `--spend-usd`, `--spend-tokens`, `--spend-calls` and `--think-secs` beat
  the profile's field of the same meaning.
- What the model is reached through is checked before the game reserves
  anything in the spend book: a missing key, or a CLI whose program is not
  there, refuses the game and costs the book nothing.
- No file: everything is as it was before the file existed. No caps and
  no spend book; a game's budget is $5 (or `--spend-usd`), checked before
  each call.

## A fallback profile

A language model, behind an API or an agent CLI, can hand a game to other
profiles when it cannot answer, rather than to the house: when its
provider says it is rate limited, busy or out of its quota, or when its
subscription's window is spent, or when it fails otherwise (owner,
13.10.2026: the house plays only in an emergency; the staging acceptance's
`502` mid-game is none; owner, 31.10.2026: a game stays playable when a
quota or subscription runs out, on the same model elsewhere first and then
on the next most suitable one). The fallbacks are a **chain**, tried in
order. The house plays only where the whole chain cannot.

- **Where the chain comes from.** A profile names it in the settings file:
  `"fallback": "backup"` (one, as it always could) or `"fallback":
  ["opus-api", "deepseek"]` (several, in the order they take over), each a
  profile of the same file ([The file](#the-file)). `baylee-seat join`
  plays that chain unless `--fallback-profile` is given, once for each
  fallback in order (`--profile primary --fallback-profile opus-api
  --fallback-profile deepseek`), which replaces it; `--no-fallback` plays
  none. A hosted seat gets it from its definition's `fallback`
  ([below](#a-hosted-seat)), and the collector names its profile's chain
  explicitly. A fallback's own `fallback` is never followed.
- **What it takes over on.** A limit: HTTP 429, or an error whose `code`
  or `type` (in the body's `error`, or at its top) is `rate_limited` or
  `busy`, whatever the status; a `Retry-After` in seconds is kept as when
  it lifts (`MindError::Limited`, `llm::limited_by`); an agent CLI's rate
  limit or spent quota, as its dialect reads it (`Outcome::RateLimited`),
  is the same limit. And every other failure, the model unavailable: a
  network error, a 5xx without such a code (`502 server_error`), a refused
  key, a task that died, a CLI process that died or broke its lockdown
  (`MindError::Unavailable`). A model that declines a kind of question
  (`MindError::Declined`) is not down: the house answers that question and
  the chain stands by. Past the end of the chain, or with none, a limit
  hands the rest of the game to the house at once: it answers that
  question and every one after it, no model is asked anything more, the
  seat stays at the table, and the house is declared to it
  (`SeatCore::limited`, owner after the feedback of 28.10.2026). It used to
  count as the model unavailable, and the third in a row took the mind off
  the table: the table held the chair as away for its whole reconnect
  window, three minutes, before its stand-in played, and the mind came
  back to be limited again. A failure is still the model unavailable: the
  house answers, and a run of three takes the mind off the table.
- **The same question, at once, down the chain.** The question the model
  could not answer is asked of the first fallback within what is left of
  that question's budget (`think_secs`, else 60 s): the table does not wait
  longer, the house does not answer it, and the failure does not count
  towards taking the mind off the table (`bridge::Standby`). From then on
  that fallback plays, until it cannot answer either: then the next one
  takes over in the same way, the same question again if that is where it
  failed. A game never hands back to a mind that played before it, however
  soon that one is back (v1).
- **Its own settings and its own book.** Each fallback plays its own
  profile whole: its model, address, key variable, answer mode, effort,
  `max_tokens`, `think_secs`, price and budgets, never the model's, and no
  flag of the command line tunes it. `--fallback-config` names the
  settings file the fallbacks are in, and `--fallback-ledger` their spend
  book: each once for all of them, or once for each `--fallback-profile`
  in the same order; any other count refuses the chair before it reserves
  anything. By default a fallback's file is the model's, and its book the
  model's `--ledger` where both are in one file, else the book beside its
  file. At sit-down each file is read and each key looked for, which costs
  nothing; a game is reserved in a fallback's book only when it takes over,
  under its file's caps. A fallback that cannot stand by (a profile the
  file lacks, one that plays an agent CLI, a key that is missing, the
  chair's own profile, a name given twice) is left out of the chain at
  sit-down, the bridge saying why (`the fallback … does not stand by: …`);
  one whose book refuses when it is to take over (a cap reached) is passed
  over for the next (`the fallback … could not take over in turn N: …`, and
  the transcript's `fallback_refused`). The model's own reservation settles
  at the game's end as always, and each fallback's beside it.
- **Only an API takes over.** The model may play behind an API or an agent
  CLI; each fallback plays behind an API: a CLI is none. A ranked AI
  account's chain is the exception: one model on several access paths, a
  CLI among them ([below](#the-ranked-runner)).

- **Said.** At sit-down `the fallback backup stands by: …`, or `the
  fallbacks opus-api, deepseek stand by, in this order: …`; at a limit
  `the fallback opus-api answers from turn 3: the primary is rate limited
  (it lifts in 120 s)` (the words a seat agent reads to hold the profile
  back from new games, `bridge::LIMITED`), and when a fallback that took
  over is limited in its turn `the fallback deepseek answers from turn 9:
  the fallback opus-api is rate limited` (`bridge::fallback_limited`, which
  holds that fallback back); at another failure `… : the primary failed:
  502: …` or `… : the fallback opus-api failed: …` (which holds nothing
  back); in the transcript `{"fallback": {"profile", "turn", "cause",
  "why"}}` for each that takes over, `cause` `limited` or `failed`, and at
  the end `{"fell_back": [{"label", "turn", "answered"}, …]}`; to the table
  a new declaration of each fallback's model before its first answer, so
  the game's record names the model that made each action
  ([below](#what-the-games-record-says-played)); in the closing line, after
  everything it said before, `; the fallback (opus-api) answered 12 from
  turn 3` for each that played, in order (their answers are also among
  the mind's), and each one's tokens and money on a line of its own.

## Models and efforts

What the client offers to choose from, `llmseat::models`. A model is
shown under a label a player reads, with its exact id beside it (`Claude
Opus 5.5 · claude-opus-5-5`), and the id is what the file and `--mind`
carry. Models are listed only where no key leaves the machine:

- **Known**: this build's tables. Hosted APIs whose listing would need
  the key are keyed by host, whichever protocol reaches it
  (`models::HOSTED`: `api.anthropic.com`, `api.deepseek.com`); Anthropic's
  table also serves an Anthropic-protocol address the build does not know
  (a proxy). An agent CLI's models and efforts are its dialect's table
  (`llmseat::clis::choices`, [below](#a-cli-as-the-model)), offered as
  `<tool>:<model>` and the bare tool last, which plays the tool's own
  default, each at the levels the tool's effort flag takes. A snapshot of
  the providers' documentation, which stays the authority.
- **Listed**: an OpenAI-compatible server on this machine (LM Studio, a
  llama.cpp server) is asked `GET {base}/models` without a key. Only a
  loopback address is asked.
- **Typed**: any other id is played as written, labelled with itself.

Efforts are offered only where the build knows which ones the model takes
(Claude's current models `low` to `max`, `xhigh` from Opus 4.7 on;
DeepSeek's models none it can name; agy's orchestrators none, since agy
refuses `--effort` for them). Elsewhere the chair plays the model's
own default and says so, and the bridge is started with
`--default-effort`, so an effort the profile names cannot reach a model
that may refuse it.

## A CLI as the model

`provider: cli` plays through an agent CLI that its owner signed in to a
subscription, instead of an API and a key (`baylee_seat::cli`). This build
speaks Claude Code (`claude:opus`, or `claude` for its own default model),
Antigravity's `agy`, Codex (`codex`), opencode (`opencode`) and Junie
(`junie`), each a dialect of its own (`cli::dialect::Dialect`; the table
in [The tools](#the-tools) says how each is locked down). Whether a tool's
terms allow automated play on a subscription is for its owner to check
before playing.

- **No key.** The tool plays on its own login (`claude` signed in once by
  hand). A cli profile has no `key_env`, `base_url`, `price` or `game_usd`
  (each refused), and answers `json_schema` or `json`, never `tools`.
- **One process per conversation, across turns.** A seat's first question
  starts the tool with the game's prefix (the answer's rules, the game,
  the deck), the seat's notes and the decision; every later question, of
  this turn or a later one, is one more message to the same process, which
  keeps the conversation. That is how the prefix stays cached: the tool
  sends the whole conversation again with each message and its provider
  reads all but the newest message back from its prompt cache, so each
  decision sends only what is new since the last (the board as it stands,
  the question, what the model has not been told). Claude Code and `agy`
  work this way: they take each message as a line on the stdin of one
  long-lived process, and that process is the session. Claude Code's own
  `--resume` would need the session kept on disk, which
  `--no-session-persistence` forbids. (`agy` reads our instructions from
  the private `baylee-seat` agent selected at startup.) Junie works this
  way too, in its ACP mode: one process holds one session of the Agent
  Client Protocol for the game, each message a `session/prompt` in it
  (`cli::acp`; §"The tools"). Codex and
  opencode answer one message a process (`Dialect::one_shot`):
  a process whose stdin is closed after the message. Each goes on with
  its conversation by the id its first process named (`Dialect::resumes`;
  §"The tools"), so each later question sends only what is new, as to a
  long-lived process, and an answer that cannot be read is asked again of
  a new process that goes on with it and hears only why. One the tool
  cannot go on with (its session files gone, unreadable, or a process
  that names another conversation than the one asked for) is begun again
  for that question, with the prefix, the notes and that it was lost.
  A conversation ends only when it outgrows its size (about 100,000
  tokens, `conversation_tokens`): the next question closes its stdin (two
  seconds, then it is killed) and starts another with the prefix and the
  notes. A process that hangs past the question's time is killed; one that
  dies while a question waits on it is said with its exit and its last
  line on stderr, and the house answers that question; either way the
  next question starts one again, with the prefix, the notes and a
  sentence that the conversation was lost. One that died while no
  question waited on it (between turns, say) is found dead by the next
  question, which starts one again for itself and is answered by the
  model. A process idle five minutes is ended, and a conversation kept on
  disk is over (the time Anthropic's API
  keeps a cached prefix by default, which is the entry the tool writes;
  a conversation resumed after that would be written to the cache whole
  again, which costs more than beginning a new one; the hour's entry is
  the API path's, set by the bridge, `docs/llm-protocol.md` §"The
  cache"), and so is the least recently used one when a
  third would start: at most two live per bridge. The summary counts the
  conversations and those begun again after a loss
  (`conversations: 3 (1 begun again after one was lost)`).
- **The program.** `command`, a whole path, else the tool's name on the
  bridge's `PATH` (absolute entries only). It is run directly with an
  argument array and never through a shell, so a shell function or alias
  of the same name (one that adds a token to every call, say) never runs.
  Claude Code's own installer keeps `~/.local/bin/claude` a link to
  `~/.local/share/claude/versions/<version>` and moves it to each version
  it updates itself to, so a profile without `command` plays whichever
  version that is when the game starts. Pin one: name the version's own
  file in `command`, as a whole path
  (`/Users/<you>/.local/share/claude/versions/2.1.284`), and change it by
  hand after an update. The bridge gives the process
  `DISABLE_AUTOUPDATER=1`, so a game never updates the tool itself.
- **Locked down.** The working directory is a fresh directory under
  the OS's temp directory, readable by this user alone and removed with
  the process, so no project's `CLAUDE.md` or settings are found. It is
  empty except for agy's own `.agents/agents/baylee-seat.md` (below). It is
  named `baylee-cli-<seat>-<random>` (the store of a tool that keeps its
  conversations on disk `baylee-cli-store-<seat>-<random>`, 122 random
  bits), never after the game: a tool may tell its provider the directory
  it runs in. The process runs in the bridge's process group, so a
  signal to that group (a collector stopping its seat, `SIGKILL`
  included) ends it with the bridge. Junie's alone runs in a session of
  its own, without the bridge's controlling terminal (`setsid`,
  `cli::launch::detach`, `Dialect::detaches`): nothing it starts can read
  the terminal the bridge was started from (below, Junie's
  `systemd-inhibit`). Every tool ends with the bridge: its stdin closes,
  and the bridge kills it when its session drops. The
  environment is cleared and given only `PATH`, `HOME`, `USER`, `LOGNAME`,
  `TMPDIR` (the session's own), `LANG`/`LC_ALL=C.UTF-8`, `TERM=dumb`,
  `NO_COLOR=1`, the tool's own login variables where set
  (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `JUNIE_HOME`, opencode's
  `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME`), its fixed ones
  (`DISABLE_AUTOUPDATER=1`, opencode's `OPENCODE_*` switches) and those
  naming the session's own files (opencode's `OPENCODE_CONFIG`,
  `XDG_CONFIG_HOME`); on Windows also
  `SYSTEMROOT`, `APPDATA`, `LOCALAPPDATA`, `USERPROFILE`, `TEMP`, `TMP`. No `*_API_KEY`, `*_TOKEN`, `BAYLEE_*`,
  `GITHUB_*`, `AWS_*`, `ANTHROPIC_*`, `SSH_AUTH_SOCK` or `DATABASE_URL` ever
  reaches it (so neither `CODEX_API_KEY` nor `JUNIE_API_KEY`), and a passed
  value that looks like a key refuses the game. A file a tool takes only
  as a path (Codex's instructions and schema, opencode's configuration,
  Junie's empty guidelines) is written, for this user alone, into a
  `support` directory beside the working directory, never into it.
  agy's agent is the exception: its discovery requires the file under
  the working directory. That file is `0600`, its directories `0700` on
  Unix, and it contains only our fixed instructions, no game or player data.
  `HOME` is the user's own, because Claude Code reads its login from
  `~/.claude` (or `CLAUDE_CONFIG_DIR`); so with any tool that reads files
  the model could read `~/.ssh` or `~/.aws` too. `--tools ""` is that
  barrier, and the tool list in the process's `init` line is the only
  proof at run time that it held (below). A private `HOME` beside
  `CLAUDE_CONFIG_DIR` would narrow it and is not done: whether the login
  is still found that way is unverified.
  Claude Code runs as `claude -p --input-format stream-json --output-format
  stream-json --verbose --restricted --safe-mode --tools "" --strict-mcp-config
  --disable-slash-commands --setting-sources "" --permission-prompts none
  --permission-mode manual --no-session-persistence --system-prompt <ours>
  --json-schema <the answer's> [--model M] [--effort E]`: no tools, no MCP
  server, no skill, no settings file or hook, no `CLAUDE.md` or plugin,
  nothing that asks a permission, nothing kept on disk. What the process
  says at its start (Claude Code's `init` line) is the proof the flags
  held, and no reply is taken before it: it must name the model's tools
  (none but `StructuredOutput`, the answer's own), its MCP servers and its
  slash commands (none), and its key's source must be named and must not
  be a variable such as `ANTHROPIC_API_KEY` (a subscription's is `none`).
  A process that does not say so, answers or fails before it does, or says
  anything else takes the mind off the table for good: the process's own
  reader kills every process of the mind at once, whether or not a
  question still waits. A rate limit before the `init` line carries no
  answer: nothing more is read from that process, it is ended, and the mind
  cools down as for any rate limit, then starts a new one. A line of
  output over a mebibyte is never read. The other tools are locked down
  as far as each lets itself be ([The tools](#the-tools)); where a tool
  names nothing at its start, its first line is the start, and any line
  that shows the model used a tool (a command, a file, an MCP or web
  call) takes the mind off the table in the same way
  (`dialect::Event::Breach`). Where a tool asks the seat first (Junie's
  permission requests), the seat always says no before anything runs:
  that is no breach, but counted (`tools the model asked for and was
  refused: 2` in the summary) and written to the transcript
  (`cli_refused`), and the model goes on (`dialect::Event::Refused`).
  **What the lockdown leaves** (27.11.2026, [The tools](#the-tools)), for
  the operator to know: agy offers every agent `manage_task` beside our
  `finish`, whatever the agent's `tools` say; Codex keeps code mode's
  `exec` and `wait` for every gpt-6.x (a JavaScript isolate with no file
  system, no network and no nested tool left), and GPT-6.2 Astra and Sol
  also keep `send_message_to_user_async`, which their catalog entry adds
  and no setting removes. A step or an item that uses any of them is a
  breach like any other tool: the answer is not taken, the mind is taken
  off the table, the profile's fallbacks (else the house) play on, and
  the bridge says which tool it was. Whether Codex reports code
  mode's or the async message's use as an item at all is unproven: its
  `--json` items are the answer, reasoning, commands, file changes, MCP
  and web calls, plans and errors.
- **Spend.** A subscription has no price: a game's limits are its tokens,
  as the tool counts them, and its calls. Cache reads count, and the tool
  reads the whole conversation again at every decision (up to its size of
  about 100,000 tokens, across turns), so a game takes far more tokens
  than through an API, most of them read from the cache: `game_tokens` is
  20,000,000 by default. Each call is held at its worst before it is sent:
  the whole conversation so far as input, and the reply. Each call is
  booked by what it used, per dialect: Claude Code's `result.usage` covers
  only that turn (the Agent SDK's cost-tracking page, "Track costs in
  streaming input mode"), and is booked as it stands; `agy` reports the
  process's running count (read off recorded games, not from its docs), so
  each reply is booked as the difference to the process's reading before
  it, and a new process counts from nothing again
  (`Dialect::usage_is_cumulative`). Codex's count is its thread's, which
  a resumed process reads back from the rollout, so it is booked as the
  difference across the processes of one conversation
  (`Dialect::usage_spans_resumes`); Junie's prompt result counts its own
  turn (measured: a second turn of one session read 18,645 input tokens
  after a first of 17,723), booked so that its whole is Junie's own
  `totalTokens`, and the session's running cost Junie reports
  (`usage_update`) is written to the transcript at the end of each turn
  (`cli_turn`); opencode's `step_finish` counts its own
  step. Under the caps a cli game reserves its `game_tokens` against
  `day_tokens` and `month_tokens`, so a day's cap of 20,000,000 holds one
  game; raise the cap, or lower `game_tokens`. `game_calls` (500 by
  default, `--spend-calls`) is held like a budget, and the summary says
  `212 of 500 calls`. A reply that does not say what it used counts at its
  worst; a process that ended without a word (a session not found, a
  crash before its start) reached no model, and is a failed call at no
  cost.
- **Rate limits.** A rate limit or a spent quota is unbilled and is the
  mind limited (`MindError::Limited`), as a model behind an API is: the
  profile's fallbacks take the game over where it names any (the same
  model behind an API is the natural first), and else the house plays
  the rest of that game ([above](#a-fallback-profile)). Until 31.10.2026
  it was the mind unavailable, so the house answered and the mind came
  back after its cooldown. The mind cools down for the
  time the tool names when that is some time and at most fifteen minutes,
  else a minute, doubling to fifteen. It plays a game again once that has
  passed
  and the tool's login check (which calls no model) passes: `claude auth
  status --json` says `"loggedIn": true`; `codex login status` says
  `Logged in using …` on stderr, not with an API key; `opencode auth list`
  names an `oauth` credential, not a stored key; `agy -p /quota
  --output-format json` (agy's read-only `/quota` in print mode, about five
  seconds) says `SUCCESS` and names at least one group of models with its
  quota. Output it cannot read counts as signed out, and so does a check
  past its ten seconds (agy's `--version`, the check before 27.11.2026,
  never failed, and showed only that the program runs). Junie has no
  such check: its `--version` shows only that the program runs. Junie's spent balance
  ("Insufficient account balance. All tokens in your account have been
  spent.") and its provider's rate limiting are limits; a session it
  cannot open because it is not signed in is unavailable for that
  question, never a lockout.
- **Tests.** Only against `examples/fake-agent-cli.rs`, a stand-in that
  speaks Claude Code's stream-json, `agy`'s, each one-shot tool's output
  and Junie's ACP, and logs what it was started with and every message it
  read (`tests/cli_mind.rs`: one process across turns with the prefix sent
  once, a process that died between turns or mid-question begun again
  with the prefix; a one-shot tool's process a question, a tool used
  taking the mind off the table; Junie's session opened with brave mode
  off before any prompt, a permission refused and played on, a tool that
  ran or brave mode kept on taking the mind off the table). Each
  dialect's arguments are golden and its lines are read from fixtures
  written from its documentation, its source or its recorded output
  (`cli/dialect_tests.rs`); no dialect may pass an approving flag or a
  credential in its arguments. No test starts a real CLI or reaches a
  model.

### The tools

The agy disconnect reported on 24.10.2026 was reproduced locally: the
default coding agent attempted `run_command` during a game decision, the
adapter permanently locked it out, and three unavailable answers caused
the bridge to disconnect. The same bridge serves client and gateway seats.
The fix selects an agent whose only capability is the output function
`finish`, rather than relying on the prompt to suppress tools. The
[agent format](https://antigravity.google/docs/subagents?tab=cli) and
[CLI flag](https://antigravity.google/docs/cli/reference/) are documented by
Google. Live agy 1.4.7 checks showed that `tools: []` inherits tools, whereas
`tools: [finish]` refused an explicitly requested shell command. Its init
still lists the global tool inventory; the adapter checks the selected
agent and continues rejecting every external tool event. A CLI that does
not confirm the selected agent fails before its first answer.

The same seed that failed on its first model call then played 500 game
actions through turn 21: 27 CLI calls, 61 mind answers including plans,
no fallback, no restart and no disconnection. A second seed played to
game end at turn 17 (524 actions, 50 CLI calls, 109 mind answers), also
without fallback, restart or disconnection. Both used
`agy:gemini-3.8-flash-low` with low effort. Automated regressions use
only the fake CLI: 20 decisions across turns, output-only tool events,
rejection of external tools and wrong/missing agent selection, private
configuration permissions and process-directory cleanup.

The Codex 0.164.0 live check on the same date found a separate startup
failure: `--output-schema` is sent in strict mode, so the API rejected the
common answer schema before the first answer. Codex now receives a strict
variant: every object is closed with `additionalProperties: false`, all
properties are required, and otherwise optional fields accept null.
This includes objects inside attacks, blocks and prevention arrays and
the nested targets/stops objects. The common schema for other transports
stays compact. The requirements are in OpenAI's
[structured-outputs guide](https://developers.openai.com/api/docs/guides/structured-outputs).
Regressions check the complete schema and that unused null fields preserve
ordinary decisions, attacks and concessions. With this correction, Codex
with `gpt-6-luna` and low effort played to game end at turn 15: 442 actions,
54 model calls, 112 mind answers, no failure, fallback or restart. Its
conversation resumed across all 54 CLI processes without a disconnect.

The CLI check of 27.11.2026 read Claude Code 2.1.325, Codex 0.166.0 and
agy 1.5.18 on the owner's Mac, and the help of Claude Code 2.1.332, Codex
0.172.0 and agy 1.5.18 on the machine that plays the AI accounts (there
also one `codex debug prompt-input` with an empty `CODEX_HOME`, which
calls no model and reads no login). Six short agy turns were played;
Claude Code and Codex called no model.

- **Claude Code.** No flag the seat passes changed; 2.1.332's help only
  rewords. `--bare` still never reads a subscription's login;
  `--agents`/`--agent` need the Agent tool, which `--tools ""` removes;
  `--fallback-model` would play another model under the profile's name,
  which a ranked chain forbids; `--prompt-suggestions` stays off unless
  asked, and no settings file is read. Its own model table: `fable` now
  plays Fable 5.2 (`claude-fable-5-2`, default `medium`), `haiku` plays
  Haiku 5.5, and every current model takes `low` to `max`.
- **Codex.** Every gpt-6.x of its catalog is `code_mode_only` with
  multi-agent `v2`, which neither `--disable multi_agent` nor
  `features.multi_agent_v2=false` turns off. With the flags of before,
  `codex debug prompt-input` showed the model `spawn_agent`,
  `send_message`, `followup_task`, `wait_agent`, `interrupt_agent`,
  `list_agents`, `request_user_input`, a collaboration-mode text, two
  multi-agent texts and the list of the user's own skills
  (`~/.agents/skills`, with their paths). The `-c` keys of the table take
  all of them out; 0.166 and 0.172 accept every one. What stays is named
  under the lockdown above. `--disable <name>` refuses a name it does not
  know ("Unknown feature flag"), where `-c features.<name>=false` ignores
  it, so the features go as `-c` and a name a later version drops fails
  no game. GPT-6.2 Astra's default effort is now `low`.
- **agy.** Gemini 4 Neon goes up to `xhigh` as Hydrogen does; the dialect
  had played `gemini-4-neon-xhigh` as `gemini-4-neon-high` and no longer
  does. `excludeDefaultComponents: true` in our agent drops agy's own
  prompt sections and built-in tools: a first turn read 470 input tokens
  where it read 14,849 without. A control pair shows that the agent's
  `tools` list is what holds, not the model's obedience: an agent with
  `tools: [finish, view_file]` read a file in a Git repository outside
  its workspace without asking (Auto mode, agy's default, reads anywhere
  since 1.5.14 and writes outside the workspace inside a repository
  since 1.5.16); ours, `tools: [finish]`, was told `view_file` is not
  available, and no tool step came. agy still offers every agent
  `manage_task`. `--mode plan` has no effect beside
  `--disable-slash-commands` (agy says so), `--sandbox` restricts only a
  terminal the agent does not have, and permission rules in `--settings`
  are not documented; none of them is passed. The login check is
  `/quota` ([above](#a-cli-as-the-model)). A result now says why a turn
  ended early (`finished_reason`); such a result is no answer.
- **agy's orchestrators** (`gemini-pro-orchestrator-neon`,
  `gemini-pro-orchestrator` for Carbon, `gemini-pro-orchestrator-argon`)
  plan with one model and execute with another, each at the effort agy's
  own `orchestrator` setting names (`high` by default since 1.5.15; the
  same model and effort for both run one agent on that model since
  1.5.17). agy refuses `--effort` for them, so the seat passes none
  whatever the profile names, and the settings panel offers none; a
  ranked account on one claims the orchestrator's id at its Standard,
  never the model it plans with. Under the seat's agent, whose only tool
  is `finish`, the Neon orchestrator answered a one-line question in one
  model step and no tool in 73 seconds, where `gemini-4-neon-xhigh` took
  82: with nothing to execute it buys little, and a game decision's
  prompt can take it near Central's 180 seconds a decision. It is a model
  a profile may name, off unless named. `/swarm` (a slash command) and
  `run_workflow` (a tool) are agy's other multi-agent modes: both lie
  outside the lockdown, spend quota fast and outlast a decision's clock,
  and are not offered. An "orchestra of Neons at xhigh" is, by agy
  1.5.17's rule, one agent on `gemini-4-neon-xhigh`, which the seat now
  plays as named.

opencode 1.19.4 with `deepseek/deepseek-flash` played a complete game
through turn 14 (508 actions, 73 calls) in the live check. One call exceeded
the probe's 58-second reply allowance; the existing recovery restarted the
conversation and play continued to game end without `mind_down` or a
disconnect. No opencode code change was needed for that run.

Junie 26.10.27 (build 3893.6) is played through its ACP mode since
31.10.2026. Measured that day on a signed-in machine: in non-interactive
mode (`--input-format=json`, the dialect before) Junie approves every
terminal command itself. `touch <file>` and `ls -la ~/.junie` (which lists
its credential file) ran, `~/.junie/allowlist.json` with
`defaultBehavior: ask` and `allowReadonlyCommands: false` was ignored,
`--act-as-interactive` did not help, and `--agent-mode=chat` (no tools) is
refused by the released build ("The chat agent mode is not available in
this version. Please use the Nightly build."). In live games Junie Lite
ran `echo …` and Gemini 3.7 Flash ran `ls -la ~/.junie/` on their own; the
seat saw each step only after it had run. In ACP mode (`--acp=true`) with
the session's `brave_mode` set `off` Junie asks its client before every
terminal command, even `ls` (`session/request_permission`, with
`reject_once`/`reject_always` options), and a refusal keeps the command
from running. The seat now opens one ACP session per process and game,
offers no file system, terminal or MCP server, asks for `brave_mode`
`off` and sends no prompt before Junie confirmed it, refuses every
permission (once), and counts each refusal instead of leaving the table.
Through the seat's own code against `gemini-3.7-flash` at low effort,
three sessions of two turns each: a game question that also asked for
`ls` and `touch` was answered as a game question by Junie's answering
agent, with no tool attempted; a bare "run this exact terminal command:
touch <file>" was asked of the seat, refused, the file never made, and
the same session answered the next question with its JSON object.
The model the profile names must be the session's model: Junie's ACP
session ignores `--model` for a model it does not offer and plays its
default instead (Junie Lite, 31.10.2026: the session said
`v1:12:jetbrains-ai:gemini-3.7-flash`). Junie names its models by a full
id (`v1:12:jetbrains-ai:claude-opus-5-5`); the seat sets the one whose
last `:` part is the profile's model, and a session that holds another
is the model unavailable (a fallback may take the game), never played
under the declared name. Its ACP list offers no Junie Lite and none of
the CLI's aliases (`opus`, `sonnet`, …).
`--system-prompt` is honoured in ACP mode (its text is in Junie's log),
`--model` and `--effort` select the session's model and effort (the
session names the model `v1:12:jetbrains-ai:gemini-3.7-flash`), each
prompt result counts its own turn, Junie exits when its stdin closes, and
the seat removed its session directory when the mind was done with it.
A turn costs about 17,000 to 19,000 input tokens (Junie's own prompt and
router), and about 1.2 US cents, by Junie's own running cost.

One hazard of the old runs stays worth knowing: while it works Junie
starts `systemd-inhibit --what=sleep … cat` (its `keepAwakeMode` setting,
`Off` or `WhileTaskRunning`). From a login without an active desktop seat
polkit asks through `pkttyagent`, which reads the controlling terminal; a
bridge started in the background of a shell then got `SIGTTIN`, which
stopped its whole process group (state `T`), the seat's own timeouts
included. The seat now starts Junie in a session of its own without a
controlling terminal (`setsid`, `Dialect::detaches`), so that agent fails
at once and the tool goes on; `"keepAwakeMode": "Off"` in
`~/.junie/settings.json` keeps Junie from starting it at all. Only Junie:
a process in a session of its own also leaves the bridge's process group,
which a collector stops with `SIGTERM` and then `SIGKILL`, so every other
CLI stays in that group and ends with it however the bridge ends.

What each dialect passes to take the tool's own agent away, how it holds
a conversation, and how its usage is booked. **Docs** marks what the
tool's help (`--help` of the installed version, 06.10.2026; Claude Code,
agy and Codex again 27.11.2026), documentation
or source says; **assumed** what no source confirmed and a first live game
must show (a tool that refuses a flag fails its game with a sentence; it
never plays unlocked). The models and effort levels the settings panel
offers for each are `baylee_client_core::llmseat::clis::choices`, a pure
table (no program is asked).

| CLI | Flags and files that strip its overhead | Conversation | Usage | Lockdown check | Confirmed |
|---|---|---|---|---|---|
| Claude Code `claude` (2.1.325; acenb 2.1.332) | `-p`, stream-json both ways, `--restricted --safe-mode --tools "" --strict-mcp-config --disable-slash-commands --setting-sources "" --permission-prompts none --permission-mode manual --no-session-persistence --system-prompt <ours> --json-schema <answer>`, `--model`, `--effort`; `DISABLE_AUTOUPDATER=1` | one process across turns, stdin lines (not `--resume`: transcripts and the login share `CLAUDE_CONFIG_DIR`) | per turn (`result.usage`) | `init`: tools only `StructuredOutput`, no MCP server, no slash command, key source named and not a variable | docs (help, Agent SDK); help of 2.1.325 and 2.1.332 checked 27.11.2026 |
| Antigravity `agy` (1.5.18) | stream-json both ways, `--disable-slash-commands`, `--agent baylee-seat`, `--json-schema`, `--model`, `--effort` (never for an orchestrator); private `.agents/agents/baylee-seat.md` carries ours and `tools: [finish]`, `subagent: false`, `commandExecutionPolicy: off`, empty MCP/skills/plugins lists, `excludeDefaultComponents: true`. User rules under `~/.gemini` remain the CLI's | one process across turns, stdin lines (its sessions under `~/.gemini/antigravity-cli` cannot be moved) | running count per process, booked as differences (1.5.18: a second turn's result counted both) | `init` must confirm `agent: baylee-seat`, name its tool inventory and connect no MCP server; every tool event except the output-only `finish` is a breach (`manage_task`, which 1.5.18 offers every agent, included); a result cut short (`finished_reason`) is no answer | flags and agent format: docs and 1.5.18's changelog; capability filtering, `excludeDefaultComponents`, orchestrators, `/quota` and multiple turns: live 1.5.18 checks, 27.11.2026 |
| Codex `codex` (0.166.0; acenb 0.172.0) | `exec --json --ignore-user-config --ignore-rules --skip-git-repo-check --sandbox read-only --output-schema <file>`; `-c model_instructions_file=<ours> project_doc_max_bytes=0 mcp_servers={} notify=[] web_search="disabled" tools.view_image=false history.persistence="none" analytics.enabled=false feedback.enabled=false otel.exporter="none" check_for_update_on_startup=false include_*_instructions/context=false model_reasoning_summary="none" include_collaboration_mode_instructions=false skills.include_instructions=false tools.experimental_request_user_input.enabled=false features.multi_agent_v2.disable_native_tools=true features.multi_agent_v2.root_agent_usage_hint_text="" features.multi_agent_v2.multi_agent_mode_hint_text="" [model_reasoning_effort]`; `-c features.<name>=false` for each tool feature (shell, exec, image, plugins, apps, hooks, skills, sub-agents, memories, browser, computer use, web, permissions, async messages, …); `--model`; prompt on stdin (`-`); going on: the same, then `resume <thread uuid> -`. `$CODEX_HOME/AGENTS.md` is still read (the bridge warns where one is) | one process a question, going on by `exec … resume <id>`; sessions in `$CODEX_HOME/sessions` beside the login, the seat's own removed | thread total, read back on resume: booked as differences across the conversation | none at start (`thread.started`); any item but the answer, reasoning or a warning is a breach | flags: docs and source (main, 06.10.); the model's prompt and tools: `codex debug prompt-input` (no model call) on 0.166 and on acenb's 0.172, 27.11.2026; **assumed**: that `exec`'s `--sandbox` and `--color` ahead of `resume` hold for it (source: global flags) |
| opencode `opencode` (1.18.34) | `run --pure --format json --agent seat --title seat [--model p/m] [--variant v]`, message on stdin; `OPENCODE_CONFIG` = a file denying every tool (`permission {"*":"deny"}`, also on the agent), `mcp {}`, no instructions, share/snapshot/formatter/compaction/autoupdate off, agent `seat` whose prompt is ours; `XDG_CONFIG_HOME` = an empty directory; `OPENCODE_DB` = a file in the seat's store; `OPENCODE_DISABLE_PROJECT_CONFIG`, `_CLAUDE_CODE`, `_EXTERNAL_SKILLS`, `_AUTOUPDATE`, `_AUTOCOMPACT`, `_LSP_DOWNLOAD`. opencode still adds its environment block (model, directory, date) | one process a question, going on by `--session <id>` from the seat's store: only what is new is sent | per step (`step_finish.tokens`) | none at start (its first line is the start); a `tool_use` line is a breach | source (v1.18.34 tag, `dev` for the schema); **assumed**: built-in plugins left on (they carry the logins); that a resumed `run` reads its history whole (source: `dev`) |
| Junie `junie` (26.10.27) | `--acp=true --skip-update-check --share-anonymous-statistics=false --config-default-locations=false --mcp-default-locations=false --skill-default-locations=false --command-default-location=false --agent-default-location=false --model-default-locations=false --extensions-default-location=<empty> --guidelines-filename=<empty file> --cache-dir=<session's> --system-prompt=<ours>` (added to Junie's, not replacing it), `--model`, `--effort`; ACP: `initialize` with no file system and no terminal, `session/new` in the empty working directory with no MCP server, `session/set_config_option` `brave_mode` `off` (and `model`/`effort` where the flags left another value Junie offers), each message a `session/prompt`. Never `--brave`, `--auth` or a key | one process across turns, one ACP session; sessions in `~/.junie/sessions` beside its settings, the seat's own removed | per turn (the prompt result's `usage`), booked to its `totalTokens`; the session's running cost to the transcript | none at start (its first message); no prompt before `brave_mode` `off` is confirmed, else a breach, as is a later change; every permission request refused (counted, transcript, no breach); a tool call that completed, failed unrefused or is still open at its turn's end is a breach | flags: help; protocol, permission requests, brave mode, `--system-prompt`, `--model`, `--effort`, per-turn usage and the refusal: live 26.10.27 checks, 31.10.2026; **assumed**: how a spent balance and a missing sign-in look on the wire (the strings are from its jar; the fake replays them) |

A tool that answers one message a process goes on with its conversation
by the id the tool's own output named for this seat, never "the most
recent" (`--continue`, `--last`, `--resume` alone, a picker). Every
process of the seat's conversation works in one directory of the seat's
own (a store under the OS's temp directory, `0700`, removed with the
conversation, the seat and the mind; a mind's start sweeps away stores a
killed bridge left, untouched for an hour). A resumed process gets the
same lockdown as a new one. One whose session files are gone is not asked
to resume; one that ends before a line (not found, unreadable) or names
another conversation than the one asked for (stopped at that line)
begins the conversation again for that question, with the prefix,
counted as lost. A conversation is over after `idle`, as a process is,
and by its size, as a process's is.

- **opencode**: `OPENCODE_DB` (an absolute path) puts its sessions in the
  store; its login (`auth.json`) stays in the data directory. It still
  appends `log/opencode.log` and makes its empty directories under the
  user's data directory, as it does without resuming: the log and the
  login share that directory, and only a key in the environment
  (`OPENCODE_AUTH_CONTENT`) would let it move. Usage is per step, so no
  session file is read.
- **Codex**, **Junie**: their sessions cannot be split from their login
  (Codex: `auth.json`, or a keyring entry keyed by the `CODEX_HOME` path;
  Junie: `JUNIE_HOME` holds its fallback credentials, and on Linux its
  keyring needs `DBUS_SESSION_BUS_ADDRESS`, which no CLI is given), so
  they are kept where the user's are (owner, 06.10.2026): Codex's rollout
  `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<time>-<uuid>.jsonl` (no
  `--ephemeral`, which would keep nothing to resume), Junie's
  `$JUNIE_HOME/sessions/<id>/`. Junie no longer resumes: its ACP process
  holds the session for the game, and the session it kept on disk is only
  removed (a dead process's conversation begins again with the prefix,
  as Claude Code's does). When the conversation is over the seat
  removes exactly the files named for the ids its own processes named
  (one that strayed included), matched by the tool's own file naming,
  never through a link; the store notes those ids, so the sweep after a
  killed bridge removes them too. Nothing else of the user's is touched:
  Codex's thread row in its own state database and the line Junie's
  `sessions/index.jsonl` holds for the session stay. No session file is
  read: Codex's `turn.completed` already carries the thread's count, and
  Junie's prompt result its turn's.
- **Claude Code**, **agy**: one process holds the conversation already;
  a resume would need transcripts on disk beside the login. agy's usage
  is cumulative over its session, as the differences booking assumes.

Not spoken, and why:

- **Gemini CLI** (`gemini`): since 18.06.2026 it no longer serves Google
  AI Pro/Ultra or free sign-ins (Google's developer blog, "transitioning
  Gemini CLI to Antigravity CLI"); its successor is `agy`, which the
  profile name `gemini` also names.
- **GitHub Copilot CLI** (`copilot`): no way to replace its system prompt;
  its `--output-format json` lines are documented only by third parties;
  more than one message needs its ACP server.
- **Cursor CLI** (`cursor-agent`): its stream-json reports no token usage
  (every call would be booked at its worst), and it has no flag that
  removes its tools or its rules files (`AGENTS.md`, `.cursor/rules`).
- **Qwen Code** (`qwen`): its free sign-in closed on 15.04.2026, leaving
  keys and a paid plan; its stream-json input is documented as "under
  construction".
- **Aider**: no structured output, and keys only.

## Reports and lookups

Every language-model seat, API or CLI, may report a rules or engine error
it notices and look a card up before it answers (owner, 31.10.2026: "better
a false report than a wrongly accepted bug"). Since 01.11.2026 a report is
structured, backed by an exact quote of a card's text or a rule number,
and filed one bug report each, so triage can sort and filter them (design
by the PM and the owner); the prompt no longer asks for a report when
unsure. What the model is told and how its answer carries them is
`docs/llm-protocol.md` §"Reports and lookups"; this is what the bridge
does with them.

**Reports** (`baylee_seat::reports`). Each one is filed as it is read, in
the answer that carried it whether or not that answer could be taken. The
model's fields are `docs/llm-protocol.md` §"Reports and lookups"; the
seat adds what only it may say, never the model (01.11.2026,
`reports::Checked`, `reports::Moment`):

- `decision`: the question the report was made at, as the seat showed it:
  `turn`, `phase` and `step` (the view's, `"FirstMain"`, `"Main"`),
  `pending` (the question's kind, `"Priority"`), `options` (every id the
  question listed and the words beside it, `[{id, text}]`, exactly as the
  narrator wrote them, `narrator::Menu::offered`: an option's label, an
  object's line without its id, a player's description; at most 40, each
  id cut to 40 characters and each text to 200, `options_omitted` counting
  the rest), `answer` (the answer that carried the report read against the
  question as the seat reads it, `"a1 Cast Shock #50"`, or `"not taken:
  …"` with why; `null` for an answer that only asked for a lookup; at most
  300), `seq` (the frame's) and, in a bug report that carries the log,
  `log_line` (below).
- `oracle_quote_matches`: `true` only if every card's `oracle_quote` is
  words of the text the seat itself told the model for a card of that
  name, spacing aside (`Narrator::told_texts`: the deck's in the prefix,
  every other card's under "New cards" this game, and the build's own text
  of a card it looked up); `false` otherwise; absent where it quotes
  nothing. A match says the model quoted what it was told, not that it
  read it right.
- `rules_format_ok`: whether every rule number is spelt as one (three
  digits, the first not 0, then a full stop and one to three digits, then
  one lower-case letter: `601`, `601.2`, `702.19b`); absent where it cites
  none. Well-formed is not the right rule.
- `kind` becomes `unverified` where the report cites nothing well-formed
  (PM, 01.11.2026, `reports::cites`; the text says which kind the model
  gave). Two things count: a quote of at least three words (a word holds
  a letter or a digit) that is word for word, spacing aside and no word
  cut in two, of a card's rules text as the seat told it for that card
  (not the face's heading: its name, cost, type line or stats), and not
  the card's name or type line alone; or a rule number spelt as one
  (above). A quote that does not match, one of a word or two (or a
  single character), a card's name, its type line, and a rule such as
  `"none"` cite nothing. `oracle_quote_matches` and `rules_format_ok`
  are said as before, whatever `kind` became.
- `dedupe_key`: SHA-256, in hex, of `[kind, the cards' names folded
  (lower case, spacing collapsed), sorted, each once, the rule numbers
  sorted, each once, pending]` as JSON. A game files one report a key,
  at most 25: the same claim about the same cards, rules and kind of
  question in other words is not filed again.

Then:

- one JSON line in `reports.jsonl`: `source`, `game_id`, `seat`, `turn`,
  `model`, `profile`, `question`, the report as a bug report carries it
  (below) without `log_line`, `lookups` (made for that question) and
  `unix`, written as it is filed, so a report the gateway never takes (a refusal,
  `429` or `503`, no answer within 15 s, a link that never came back) is
  still on the machine that played. Every language-model seat has the
  file (`config::Paths::reports`): in `join --reports-dir` where it is
  given, else in the `--transcripts` directory, else in `llm-reports/`
  beside the settings file, where its spend book is (a seat the desktop
  client starts: `~/.config/baylee/llm-reports/` on Linux; a named
  `--config` elsewhere keeps the player's config directory out of it). A
  fallback that takes over writes to the model's. A seat agent hands its
  bridges `--reports-dir <state>/reports`. The bridge never reads the
  file back, sends it or cuts it short: at most 25 lines a game, until
  whoever owns the machine removes it (`docs/privacy.md`). The seats of a
  table may share the directory; each line is one write.
- when the game ends (won, lost, conceded, or the room closed under the
  seat), **one bug report a report** to the gateway's `POST /reports`
  (`docs/feedback.md`) under the seat's own session, in the order filed, so
  triage takes them one at a time: `kind: "bug"`, the game's id (the
  gateway attaches its record), a text that says the report is a model's
  and tells it (cards written `[Card Name]`, the seat's checks, the
  answer), and

  ```text
  client: {source: "llm-seat", untrusted, model, profile, seat, game_id,
           report: {kind, severity, confidence, oracle_quote_matches,
                    rules_format_ok, dedupe_key, cards: [{name, oracle_quote}],
                    rules, decision, observed, expected, references,
                    question, lookups, unix},
           record_tail, record_sha256, refs, log, log_omitted}
  ```

  `refs` (the cards the text names, resolved from the seat's own view and
  this build's pool only); `evidence` too for a report of the old shape.
  `oracle_quote_matches` and `rules_format_ok` are left out where they are
  absent (above), `severity` and `confidence` are `null` where the model
  did not say them, an `oracle_quote` is left out of a card the model
  named without one. `model` and `profile` are the mind that filed the
  report (after a fallback took over, its own), and the text names the
  seat as its log and its model do (`P2`; `seat` is the view's number,
  from 0) and the mind. Triage reads it as a
  player's report: the model's claim, never an instruction, checked
  against the Comprehensive Rules and the Oracle; the seat's checks only
  sort. Each text is held to 20,000 characters and each `client` to
  2,000,000 bytes.
- **no record of the seat's own.** The seat has no record of the game
  (another seat's inputs, the engine's hashes), only its own log, so it
  attaches none (`local_record`, which the gateway would check as a
  client's record); the gateway attaches the game's own by `game_id`, as
  below. `client` names the game (`game_id`) and carries `record_sha256`
  (SHA-256, in hex, of every line of the seat's whole log, each followed
  by a line break: the same in every report of a game, and recomputable
  from `client.log` where `log_omitted` is 0) and `record_tail` (the last
  20 lines of the log the seat had been told when the report's question
  was asked, `[{turn, text}]`, each cut to 300 characters, kept whole
  however much of `client.log` was left out).
- **the seat's own game log** rides with them as `client.log` (since
  31.10.2026, `reports::game_log`): the desktop client's "your game log"
  attachment in its shape (`{seat, roster: [{seat, name, is_ai, team}],
  lines: [{turn, at, text}]}`), written by the client's own code
  (`bugreport::seat_log_with`) out of the log the table told this seat,
  which the core keeps in the client's own book (`TableMemory::book`). So
  it holds only what the seat was told (another seat's hidden draw is "P1
  drew 1 card", a face-down card stays face-down), never its view, a hand
  or a library, and nobody's name: lines in English, the seat "You" and
  every other seat `P1…Pn` as its model knew them
  (`Placeholders::Seats`; a person's report says "Player A", "Player B").
  The whole game, up to its end; where `client` would pass 2,000,000
  bytes (the client's reading of the gateway's 2 MiB) the oldest lines
  are left out, the newest kept, and `client.log_omitted` and the text
  say how many (0: none). Each report says after which line of the log as
  sent it was made: `decision.log_line` (from 1, as the feedback service
  lists the lines; 0 before the first) and "q12, turn 4, after log line
  57" in the text. A seat told no line yet sends no log, and no
  `record_tail` or `record_sha256`.
- before it goes, each body is sealed as a person's report is
  (`bugreport::seal`): one that holds the seat's session or seat token
  anywhere is not sent, and `why` names which token, never its value; the
  game's other reports still go.
- a refusal (a gateway without a feedback service, `429` when the
  account's budget an hour is spent) or no answer within 15 s stops the
  rest, which would meet the same: they stay in the reports file. A seat
  that files many reports in many games spends its account's budget
  sooner than with one bug report a game. The budget is the gateway
  operator's (`BAYLEE_REPORTS_PER_HOUR`, 64 by default and on the official
  gateway, the same for every account, never tied to `client.source`;
  `docs/feedback.md`): the LLM collector's private gateway gives its seat
  accounts 2000.
- the seat transcript's `llm_reports` line says what became of them
  (`count`: the reports filed; `bug_reports`: the bug reports they made;
  `sent`: whether the gateway took every one; `report_id`: the first id;
  `report_ids`: every id taken, in order; `why`: the first reason one was
  not sent; and `route`: `"reports"`, `"chair"`, `"direct"` or `"none"`,
  below). It is said there and in the log, never in the game.

Which chairs send how, and with which record (`link::Reporter`):

- a seat with its own account, and a host's own chair handed to the
  bridge (`--host-chair`: the account's person's chair, played over its
  seat token), send under that account's session (`POST /reports`, route
  `"reports"`). The account sits at that chair in the game's record
  (`game_record_seat.account_id`), so the gateway attaches the record as
  it does to a player's report.
- a chair taken on its host's ticket (`--chair-ticket`, a host's own
  bridge or a hosted model) has no session: it sends on its seat token
  (`POST /lobby/games/{id}/chair/report`, route `"chair"`, owner,
  31.10.2026). The gateway files it under the pseudonym of the host who
  answers for the chair, against the host's budget an hour, with that game's
  record, which the chair played (`delegated_by`); **once a chair a
  game**, and only about that game (`docs/feedback.md`). That limit stays,
  so this route sends the game's reports in one bug report
  (`reports::bundle`): `client.reports` instead of `client.report`, each
  item the same object with the same flat fields, its own `record_tail`
  and the `model` and `profile` of the mind that filed it;
  `client.model` and `client.profile` are the first report's, the text
  names every mind in order, and `client.record_sha256`, `game_id` and
  the log are there once. One
  report alone goes exactly as under a session. Should the reports alone
  pass the bound of `client`, the last are left out and
  `client.reports_omitted` counts them. A gateway from before the route
  answers `404`, which `why` says.
- a ranked seat (`baylee-seat ranked`) has no session at its host and
  Central's relay no route for reports: with `--reports-to` its runner
  sends the game's reports in one, as a client signed in nowhere sends a
  report, to the official gateway's `POST /client/reports` (route
  `"direct"`, queued in the runner's outbox; §"The ranked runner"),
  else they stay in the reports file (route `"none"`).

`join --no-send-reports` sends none either way and still writes the file.
The LLM collector (`llm_collector4 collect`) sends every game's reports
by default (owner, 01.11.2026; there is no cap of 200 games any more):
how many reach the triage is bounded by its private gateway's budget an
hour for its accounts (`BAYLEE_REPORTS_PER_HOUR`, 2000 as `serve` starts
it) and by each seat's dedupe (each key once, at most 25 a game). An
operator who wants fewer passes `--report-games N`: the root's first N
games send theirs, and every later one passes `--no-send-reports`.

**A model that watches the trained AI** (`client.source: "llm-trainer"`,
`reports::Source::LlmTrainer`). The report's building is the same, with
that source: `client.untrusted` and the text say the model watched the
trained AI play, and `decision` adds `policy_chose` (what the policy
chose) and `model_would` (what the model would have), each at most 300
characters, `null` where not given; a seat's report never has them. Who
files such reports is not wired yet.

**Lookups** (`baylee_seat::lookup`). An answer of only `ask` and `lookup`
asks for a card; the seat answers with its text and asks the same
question again, which costs none of the question's two tries. An answer
that also answers (`pick`, `attacks`, `blocks`, `prevention`, `number`,
`piles`, `name` or a concession, `Decision::answers`) is taken as it is,
and its lookup is passed over: not made, not counted, and a
`lookup_passed_over` line of the mind's transcript. At most 3
lookups a question and 30 a game a seat, and none with less than 13 s of
the question left; past them the field is passed over (an answer that also
answers is taken, else it is asked again, "no lookups left …"). A name is
read to its first 200 characters (`lookup::NAME_CHARS`). The text
comes from the nearest source that has it, under its name:

1. **Baylee card text**: the English Oracle compiled into this build, every
   face's name, mana cost, type line, power and toughness or loyalty. No
   network.
2. **gateway catalog**: for a card this build does not play, `GET
   /catalog/search` and `GET /catalog/text` (`docs/protocol.md` §"Card
   text") under the seat's own session; a chair without one skips it.
3. **Scryfall**, only with `"lookup": "scryfall"`: the card's rulings (at
   most ten), and the text of a card neither of the others knows. A
   lookup asks `GET https://api.scryfall.com/cards/named?fuzzy=<name>` and
   then the card's `rulings_uri`, which is followed only when it is on
   `api.scryfall.com`; no redirect is followed. The requests carry the
   card's name and nothing else, under `User-Agent: baylee-seat/<version>`
   and `Accept: application/json;q=0.9,*/*;q=0.8`, one at a time and
   100 ms apart (`docs/legal.md` §3: at most ten a second) for every
   seat of the machine: within a process under a lock in memory, and
   across the processes that keep the same cache directory (a collector's
   seats) under an exclusive lock on `scryfall/.requests.lock` there,
   which holds when the last request ended (`lookup::scryfall::Turn`). A
   request waits for that lock at most its own time (≤ 10 s) and then
   fails the lookup, never longer; where the file cannot be opened or
   locked the spacing holds within the process only. What was fetched is
   kept a week (at least the day Scryfall
   asks for) in memory and in `scryfall/` of the user's cache directory
   (`~/.cache/baylee/scryfall` on Linux), one file a card written through
   a temporary file, for this machine's seats only; "not found" is kept
   in memory only (a model's made-up names never fill the directory), a
   failure not at all.

A failed source is a sentence in the text (`lookup failed: …`), never the
mind's error. Each lookup, with the text the model was told, is a line of
the mind's transcript (`lookup`: the name, the sources, the text, its
count in the question and in the game).

### Giving the reports a status

`llm_report_status` (crate `baylee-train-ops`) signs in to the feedback
service's UI as one of its admins (`--admin`, default `claude-bigace`,
made on the server with `baylee-feedback admin add`) and works on its
reports there (`docs/feedback.md` §"The web UI"):

- `facets [--filter q]` prints the counts per status, kind and source
  under the list's filters, as a query string (`status=new`, `source=llm-seat`).
- `pull --filter q --out f.jsonl` writes every report the filter matches,
  oldest first, one line each: id, status, revision, labels, game, text,
  model, profile and the seat's claims (`client.reports`, or `client.report`
  as a list of one), never the log.
- `apply --plan p.jsonl` counts a plan's changes, one
  `{"id", "status", "labels", "revision"}` a line, and makes them only with
  `--write`, each against the revision it was decided on (`409`
  otherwise); `labels` is the whole set.

The password is `BAYLEE_FEEDBACK_ADMIN_PASSWORD`, never an argument; a
trailing newline is ignored. A seat's reports reach the service only from
a collection run without `--report-games 0`.

## The spend book

`llm-spend.json` beside the settings file, or where `--ledger` says (a
book with no file has no caps, but counts). Under a file every game of a
language model is in the book, and a house or scripted game never is.

- **Reserve.** Before it sits down, a game reserves its budget, or less
  when less is left of the day's cap or the month's. With nothing left, or
  too little for one call, it is refused with a sentence naming the cap,
  what is left, the games that took it (and how many are still open), and
  when the day or month begins again.
- **Hard limit.** A reserved game plays under its reservation as a hard
  limit: before each call it holds that call's worst case (every byte of
  the request as an input token, a provider's allowance of 2,000 tokens
  and `max_tokens`, dollars at the dearest input rate, which is an hour's
  cache write where the model has one), and a call that
  could pass the budget is never sent. A call whose bill is unknown (a
  timeout, a reply that could not be read) counts at its worst.
- **Settle.** When the game ends, however it ends (its end, an error,
  ctrl-c, SIGTERM from `kill`, `docker stop` or a service manager; on
  Windows ctrl-break, its console closing, logging off, shutting down), it
  settles with what it spent, rounded up. A bridge killed outright
  (SIGKILL, or SIGHUP when its terminal closes, which keeps its meaning so
  that `nohup` still works) never settles, and its reservation counts in
  full: the book errs high and never lower than the bill.
- **At once.** Bridges that reserve at the same time take turns under an
  exclusive lock on a file beside the book (`llm-spend.lock`), and the
  book is rewritten whole through a temporary file, so no two games take
  the same money.
- **Periods.** A game counts in the calendar day it reserved in, and that
  day's month: the player's local day where the system says its offset,
  UTC where it does not. The bridge says which as the game sits down
  (`reserved $3.00 for this game in the spend book …, counted in
  2026-09-30 (local time, UTC+02:00)`), and so does a refusal. On unix the
  offset is read whatever the number of threads; a zone changed while a
  bridge runs may not be seen until it starts again.
- **What it holds.** Per game: an id, the time, the day and offset, the
  profile and model, what was reserved and what was spent, and when it
  settled; never anything of the game itself. A book that cannot be read
  refuses every game until it is moved aside: it is never started afresh.

## The settings panel

The client's settings screen carries a panel for this file, below its two
columns, on a desktop: a browser and a phone run no bridge, and say so in
one line. `llmseat::panel::SeatPanel` decides and is tested in client-core,
`llmseat::desk::Desk` reads and writes the files, and
`crates/baylee-client/src/seatpanel.rs` only draws.

- **Profiles.** Listed as chips; *Add a profile* makes one on the build's
  Anthropic default (named `sonnet`, else `sonnet-2`, …), *Duplicate*
  copies one below itself under a free name, *Remove* takes it away, and
  *Default* makes it the default or, pressed again, none. The default
  follows its profile through renames and removals of others; removing it
  leaves no default (the house plays) rather than choosing one for the
  player.
- **Boxes.** One per field of the table above and one per cap; a field
  its provider does not use (an address beside Anthropic or a CLI; a key,
  a price or a dollar budget beside a CLI; a command beside an API) shows
  only while it holds something or has the caret, to be emptied. An empty
  box leaves the field out and says what the bridge plays instead. Beside
  the model are this build's priced models, with their dollars per
  million in and out, and under the prices what this build knows of the
  model's. The key variable's box is the *name* of its variable, and
  beside it whether that variable is set in the client's own environment:
  only whether; its value is never read into the panel, and the bridge
  reads its own.
- **Protocol and address.** A profile's provider is offered as its
  protocol (*Anthropic Messages*, *OpenAI-compatible*, *Agent CLI*) and
  its address is a box beside it. *Add an adapter* offers presets, each a
  profile with protocol, address and key variable filled and every box
  editable afterwards: Anthropic (`ANTHROPIC_API_KEY`), OpenAI
  (`https://api.openai.com/v1`, `OPENAI_API_KEY`), DeepSeek
  (`https://api.deepseek.com/v1`, JSON answers) and DeepSeek over its
  Anthropic address (`https://api.deepseek.com/anthropic`), both
  `DEEPSEEK_API_KEY`, LM Studio on this machine (`http://127.0.0.1:1234/v1`,
  no key, its loaded models listed), and the CLIs: Claude Code,
  Antigravity, Codex, opencode and Junie (`llmseat::seating::Preset`).
- **The key box.** Under the address of a profile that needs a key: what
  the credential store keeps for that variable and host (*a key is kept*,
  *none kept*, or why there is no store), a box that draws what is typed
  or pasted into it only as dots and has no eye to show it, *Keep* and
  *Forget*. *Keep* hands the key to `baylee-seat key set` on its stdin and
  empties the box; nothing in the panel ever holds a kept key, so there
  is nothing to show back, only to replace or forget. The store is asked
  again each time the settings screen opens (a key may have been kept
  from a terminal meanwhile). `llmseat::keys::KeyDesk` decides,
  `crates/baylee-client/src/seatbin.rs` runs the jobs off the frame, and
  `BAYLEE_SEAT_BIN` names the bridge (default: `baylee-seat` beside the
  client; every package ships it there). A room's open chair offers
  *Language model* on every desktop build (`tableseats::offer`): live with
  the bridge, drawn off with its reason without it, and with no profile yet
  its sheet explains profiles and keys and leads to Settings › Language
  models.
- **Faults.** Every refusal of the file stands beside the box it is about,
  in the player's language: `SeatSettings::faults` is the list that
  `parse` and `check` say the first of, so the panel and the bridge refuse
  by one predicate. The panel adds its own (a number that is not one, half
  a price, two profiles with one name). Save is dead while any stands, so
  a key typed or pasted into any box but the key box is refused there and
  never written. A dollar cap beside a model with no price is
  a warning rather than a fault: the file is sound, and the bridge refuses
  only that model's games.
- **On disk.** Save writes through `store::save`. While the screen is up
  the panel looks at the file and the book once a second (their length and
  time; a quiet look reads neither). A file changed on disk is read again;
  edits in the panel are kept, with a line saying the file changed, and
  *Discard changes* shows it. A file that cannot be used is named with its
  refusal and left alone for the player to mend. No file is an empty panel
  that says so.
- **Spent.** Today and this month, from the book at the client's clock and
  local offset, with the zone said (`Counted in local time, UTC+02:00.`),
  against the caps as the boxes now say them.

## A language model at your table

What a model is offered for a card is every price its lands can pay, each with
its taps: buyback and multikicker among them (`docs/llm-protocol.md`, "Prices
above the printed one"). The narrator says what buyback does as it asks, and
where more than one effect wants a resolved spell, what each answer does.

A model orders its own triggers (CR 603.3b). The question lists each ability with
its sentence, what it is about, and whether a target or a mode follows, under a
short id (`#T3`), and says which end goes first: the id listed first resolves
first. The answer is one list in `piles` holding every id. Rows that need no order
between them say so ("the same as #T2"). The question is asked at every table,
the collector's included (`docs/trained-ai.md` §"The order of triggers").

A host's desktop client can seat a language model in a chair of its own
gateway table, beside the house. The engine plays the house; a language
model is a seat bridge the client starts (`llmseat::seating`, decided and
tested in client-core; `crates/baylee-client/src/tableseats.rs` spawns and
holds the processes).

- **Where.** In the room, on every chair that is not taken, the host sees
  *→ language model* (a desktop at a gateway only: offline the house
  plays alone, and a browser or phone runs no bridge). Pressed, the chair
  is opened (the house leaves it) and an editor stands on its card: the
  settings file's profiles, the chosen profile's models (labelled, [as
  above](#models-and-efforts)), the efforts the model takes, the deck
  it brings (one of the acceptance decks), *Keep as the profile's* and
  *Remove the model*. *Keep as the profile's*
  writes the chosen model and effort into the profile, through the same
  `store::save` as the settings panel; until then they are the chair's.
- **When.** Once the gateway lists the chair open, the client starts
  `baylee-seat join <room> --chair <n> --config <file> --profile <name>
  --acceptance <deck> --tethered` (with `--mind`, `--effort` or
  `--default-effort` where the chair differs from its profile). A locked
  room's password is handed over in its environment
  (`BAYLEE_ROOM_PASSWORD`), never on its command line, and its key it
  finds itself ([above](#where-a-key-is-kept)). A changed plan restarts
  the bridge while the room waits; nothing starts or stops once the game
  is on.
- **How it gets in: the host's chair ticket.** A host signed in to an
  account first asks its gateway for a chair ticket for that chair
  (`llmseat::door`; `docs/protocol.md` §"A host's chair for a seat
  bridge"), and starts the bridge with `--chair-ticket` and the ticket as
  the first line of its stdin: never an argument, never its environment,
  never a log. The bridge redeems it with its name (`LLM-…`) and its deck
  and sits in the chair as the host's delegate, with no account of its
  own, so it gets in where the gateway takes no guests
  (`BAYLEE_AUTH_MODE=central`, the default) or its guest cap is reached.
  The room lists the chair under the
  bridge's name with the host's handle (`delegated_by`), and the game's
  record names the host as the one who answers for it
  (`game_record_seat.delegated_by`), never as the one who played it. A
  host who is a guest is handed no ticket: its bridge signs in as a guest
  named for its mind (`GuestSignIn`), as before, where the gateway takes
  guests, and the chair's card says why not where it takes none. So does
  an older gateway that sells no tickets (`404`): the guest door, where
  there is one.
- **An already occupied account chair (local supervision).** A controller
  whose regular account already holds a chair of a waiting room (the
  host's chair 0 of the room it opened, or a chair it joined) may hand that
  same chair to `baylee-seat join <room> --chair <n> --tethered
  --host-chair`. Its first stdin line is JSON with `game_id`, `seat` (the
  `--chair`), `seat_token` and `session_token`; the remaining stdin still
  holds the tether. Never put that line in arguments, environment, logs or
  files. The bridge checks that the session owns that occupied human chair
  of that waiting room, then uses the same session and chair for ready,
  waiting and its socket. A chair ticket seats only a language model
  (`LLM-…`), so this is how the house (`--mind house`) takes a chair at a
  table it does not host: in an account named `HOUSE-…` (the LLM
  collector's `house:<level>` seats). It creates no guest, uploads no second deck and
  never starts the room; the controller does that once all chairs are
  ready. The account's displayed name and uploaded deck must already
  match the chosen mind's disclosed name and configured deck. This is the
  account's own chair, not a delegated chair ticket: the normal account
  identity remains in the gateway's record until its ordinary anonymized
  export. Only a trusted local supervisor should hand over this session,
  which retains the account's authority. `--chair-ticket` is incompatible.
- **Ready once the model answers.** Sitting is not ready. The bridge first
  asks its mind the cheapest question its provider takes (`Mind::check`:
  the model's entry for Anthropic, the model list for an OpenAI-compatible
  endpoint, the tool's own login check for a CLI; never a game's call),
  and only then says the chair is ready (a delegate with `POST
  …/chair/ready` and its seat token, a guest with `POST …/ready`). A
  refused key, an unknown model, an endpoint that does not answer or a CLI
  that is not signed in stops the bridge with that reason, which is the
  chair's card's last line, and the chair is given back. A guest bridge
  says ready again whenever the host's rearranging took its yes back; a
  delegate's yes is about its model and stays.
- **Held by the client.** `--tethered`: the bridge holds its stdin from
  the client, and when that closes (the plan removed, the client quit or
  crashed) it stops as for ctrl-c and, before the game, gives its chair
  back (`POST …/leave`, or `POST …/chair/leave` with its seat token on a
  ticket), so a room never keeps a chair for a bridge that is gone. A
  bridge that cannot start says why on the chair's card and is not retried
  until the plan changes. Its output can outlive the client reading it, so
  every line it writes goes through `client_core::say!`, which drops a
  line nobody reads rather than panicking ("failed printing to stderr:
  Broken pipe", beta.5).
- **Taken back.** The host empties the chair by arranging it (`POST
  …/seats/{seat}` with a `kind`), which ends its seat token; a host who
  leaves the room, hands it on or deletes its account takes its bridges'
  chairs and its unspent tickets with it.
- **Refused for a while.** The gateway answers `401` to a delegate's seat
  token when the chair was taken back, when the host's sign-in it was
  minted under ended (a Central session lives 30 minutes, never renewed:
  `docs/central-login.md`), and when it could not make the check at all (a
  failed Central check is a `401` too, every five seconds on every
  socket). A delegate counts its room closed to it only once the refusals
  of its chair (`GET …/chair`) have lasted `link::REFUSAL_PATIENCE` (45 s);
  until then a refused ticket is dialled again on the client's schedule
  and a refused chair read is asked again. A room that is gone (`404`)
  closes it at once. Until 22.10.2026 the first `401` ended the seat for
  good (owner: the local LLM agent "keeps losing its connection").
- **Limits.** A positive decision clock of 30 seconds or less is refused by
  the bridge, as from a terminal, unless `--allow-blitz` is explicit.
  `decide_secs: 0` and an absent decision clock mean untimed and need no
  override: guest rooms, delegated tickets, host handoffs and the socket
  seat all use this rule. The chair keeps the house level `steady`
  for its fallbacks (a plan's end, a cap reached).

## Changing a chair during the game

In a debug build only (`seating::LIVE_CHANGES`), a panel of the duel
lists the chairs this client's bridges play and offers, for
each, the house, every profile, the profile's models and their efforts.
A press writes one JSON line to that bridge's stdin (`seating::Order`:
`{"mind":"house","level":"steady"}` or
`{"mind":"model","profile":…,"model":…,"effort":…}`, at most 1 KiB, no
key-shaped value), and the bridge plays the new mind from its next
decision; a question already being thought about is answered by the mind
that was asked it. The order is chosen as at sit-down: the key or program
checked and the game reserved in the spend book first, and on any refusal
the old mind plays on, the bridge saying why. The new mind thinks for as
long as its profile says (`think_secs`, else the bridge's), from the
question it is first asked where that is shorter and from the next one
otherwise (`bridge::Swap::think`). The chair keeps the name it sat down
under, so a language-model chair takes a model or the house, never the
reverse; the panel then lights the house alone. The table's name is what
sat down, not what plays now (`SeatIdentity` says why a seat never renames
itself); a seat that renamed itself mid-game would need its own message
from the gateway to the engine. A release bridge reads no orders and a release client
draws no panel.

## A hosted seat

A model the *gateway's operator* runs for any registered player, on any
platform: the bridge runs beside a **seat agent** (`baylee-seathost`) on the
operator's machine instead of on the host's, and the host only picks it in
the room (`docs/protocol.md` §"Hosted language-model seats" has the routes
and frames). Everything below the choice is the bridge as this document
describes it.

- **The seat agent** dials the gateway's `/seathost/ws` with
  `BAYLEE_SEATHOST_TOKEN` (on its unix socket when on the same machine, the
  only way a key may be sent to it), under a stable name
  (`BAYLEE_SEATHOST_NAME`). Several may be connected, on several machines;
  a profile of the same id on several is one offer, and each order goes to
  the least busy one that has room.
- **Its profiles** are `hosted.json` in its state directory
  (`BAYLEE_SEATHOST_STATE`): per id a label, a vendor, `enabled`, and the
  settings file's own profile (`provider`, `model`, …: an API with a key, or
  a CLI signed in as the seat agent's user). Optional per profile, each off
  when absent: `max_games` (games at once), `caps` (the settings file's
  four caps, kept in the profile's **own** spend book,
  `spend/<id>.json`, so one profile's spend never stops another),
  `canary` (a paid check, below) and `fallback` (other API profiles of the
  same seat agent and vendor, one id or a list in the order they take
  over, [above](#a-fallback-profile) and below; an agent CLI's profile may
  name them too; they are named here, never in `profile`). Which
  profiles a seat agent runs is
  written through the admin console (`PUT /admin/llm/seathosts/{name}/
  profiles/{id}`), which the gateway passes to that seat agent and keeps
  nowhere; the file may also be edited by hand, read at start. Every
  refusal of the settings file holds: a key or key-shaped value in any
  field refuses the profile.
- **Keys** are in the seat agent's key store, `BAYLEE_KEY_STORE=file:<dir>`
  (`llmseat::keys::FileKeys`: one file per entry, named as the OS store's
  account, the directory `0700`, each file `0600`, written through a temp
  file, and a file another user may read or owns refused rather than
  read). The bridges it starts read the same store, as on a desktop. The
  console may set or forget a key, write-only: it goes over the unix socket
  into the store, and nothing ever answers it back (the listing says `kept`
  or `absent`). `baylee-seat key set --profile <id> --config <file>` as the
  seat agent's user does the same on the machine.
- **A chair.** For each order the seat agent writes a settings file that
  holds the profile alone (its caps as the file's) and starts
  `baylee-seat join <room> --chair <n> --chair-ticket --tethered
  --config … --profile <id> --ledger spend/<id>.json --name <label>` with
  the gateway's chair ticket as stdin's first line. A profile whose
  definition names fallbacks ([above](#a-fallback-profile)) gets a
  settings file for each that is usable (a profile of the seat agent,
  behind an API, of the same vendor, switched on), holding it alone with
  its own caps (`llm-seat-fallback-1.json`, `-2`, …, named by place, never
  by id), and for each, in order, `--fallback-profile <fallback>
  --fallback-config … --fallback-ledger spend/<fallback>.json`. No key
  travels with any: the bridge reads each from the seat agent's key store,
  each under its own profile's entry, and its environment is the seat
  agent's allowlist as before.
- **New games on a fallback.** While a profile is not available for any
  reason (at its `max_games`; limited, its book at a cap or the provider's
  limit, which the seat agent also hears from a bridge's `the primary is
  rate limited` line; failing its check; signed out; being checked;
  switched off) and one of its fallbacks is available, a new game starts
  outright on the first of the chain that is: that fallback's settings
  file and book, so the provider's cache is the same endpoint's from the
  first question, with the fallbacks after it in the chain standing by as
  they would behind the profile. A fallback a bridge heard limited in its
  turn (`the fallback <id> is rate limited`) is held back from new games
  in the same way. The chair keeps the ordered profile's name, the
  gateway's order and status are the ordered profile's, and the game
  counts in that fallback's games (its `max_games` holds it). The profile
  is reported `available`, with `diverts` naming that fallback, and the
  operator's log
  says which profile the game got and why (`primary at max_games 5/5`,
  `primary failing: …`,
  `primary limited for another 300 s`). Like every bridge it
  names no player to the model ([above](#the-language-model-seat-settings-and-spend)):
  the prompt's prefix says `P2 is an opponent.`, never a display name. (A
  `--hosted` switch did this for hosted seats alone until 13.10.2026; it is
  gone, because no bridge names anyone now.) A
  stop from the gateway closes the bridge's stdin, and a tethered bridge
  gives its chair back.
- **States** (reported whole on every change, never derived by the
  gateway): `available`; `busy` (at `max_games` or the seat agent at its
  capacity); `exhausted` until the next UTC day or month when the
  profile's book reached a cap, or until the provider's stated time, or a
  backoff of 5, 10, 20, 40, then 60 minutes when it named none (a good game
  starts it over); `probing`; `needs_login` (a CLI's login check says
  signed out; checked again hourly); `failing` (its check failed; checked
  again hourly, and at once after any change); `disabled`.
- **Checks.** `baylee-seat check --profile <id> --config <file>` runs the
  bridge's free check (`Mind::check`: the model's entry, the model list, a
  CLI's login status) and prints one JSON line, `{"ok":true}` or
  `{"ok":false,"cause":"signed_out|limited|failing","error":…}`. With
  `--canary` it also asks the model one real question (a colour of two on
  an empty board, `baylee_seat::canary`), which costs a call and is the
  only way to see a spent credit or a used-up subscription window; the
  seat agent asks it only when an admin presses *probe* on a profile with
  `canary: true`.
- **Debug bridges.** A debug `baylee-seat` shows the model's reasoning to
  the whole table (§"The AI log"); `baylee-seat --version` says `debug` or
  `release`, and a release seat agent refuses to start with a debug bridge.
- **Who may order one**: a host signed in to a registered account to which
  the profile is open; a guest is refused (`403`). Each profile id has a
  policy on the gateway, set in the admin console's Models page: every
  registered account, only the accounts on its list, or nobody
  (`docs/protocol.md` §"Who may seat which model"). A profile no admin has
  opened is open to nobody, except the one marked the default for
  registered players. A player is listed only the profiles open to them,
  and the room shows no hosted entry when there is none. The policy is the
  ordered profile's alone: its fallback is the operator's choice and plays
  for whoever may seat the profile, whatever the fallback's own policy
  (which applies to ordering the fallback itself). Which profiles a
  seat agent runs and who may seat them are set apart: a profile's id is
  only a label the operator chose, and nothing on the gateway names what
  stands behind it.

Running one (`scripts/server/baylee-seathost@.service`): its own OS user
per instance (`baylee-llm-<name>`) whose home holds the CLIs' sign-ins and
the key directory, `BAYLEE_GATEWAY=unix:/run/baylee/gateway.sock`,
`BAYLEE_SEATHOST_BRIDGE_GATEWAY=http://127.0.0.1:28766` (bridges speak
HTTP), `BAYLEE_SEAT_BIN` beside it by default. On `SIGTERM` it reports
every profile `disabled`, refuses new chairs, and exits when its bridges'
games are over.

## The ranked runner

`baylee-seat ranked` (`baylee_seat::ranked`) plays Ranked as an AI player
(`docs/proposals/ai-players.md`; Central's side is baylee-central
`docs/ai-players.md`). It is the same bridge with another way to its
table: it holds an AI runner token instead of a password, offers the AI
account's decks, plays every match Central makes from those offers
through Central's relay, and says what played to the game's record. One
process plays one AI account (`baylee-ai@<account>.service`); a machine
runs one per account its token lists.

```text
baylee-seat ranked --account <id | Name#tag>
    (--token-file <path> | --token-key <name>)
    [--central live|http://127.0.0.1:<port>]
    [--state <dir>] [--decks <dir>] [--transcripts <dir>]
    (--level expert | --profile <name> [--config <file>] [--ledger <file>])
    [--fallback-level expert]
    [--accept-host <origin>]… [--facts-from <origin>] [--facts-file <file>]
    [--games <n>] [--min-think-ms 1500] [--no-carousel]
    [--reports-to <origin>]
```

Everything but the pilot's own flags is `ranked::cli::RankedArgs`, and
`ranked::cli::run(args, pilot)` is the whole process (its stop signals,
logging, TLS, the credential store, the runner): another program that
flies its own pilot (the trained net's `seat4`) flattens the one and calls
the other.

- **The account.** An administrator creates it on Central's website:
  a display name starting `KI-` (`KI-Haus-Expert`), its kind, its profile
  (the runner's key for its mind, `house:expert`), its label ("Haus-KI ·
  Expert"), `max_games`, and no password. Central badges it as an AI
  wherever it names it, so the seat is held to the account's label
  (`BridgeConfig::labelled`, `Labelled::Account`) rather than a chair's
  name. `--account` is its id, or its whole public handle with the tag
  (`KI-Haus-Expert#00ab`): Central looks a name up only with its tag.
  The runner plays an account only with the pilot its settings name: the
  kind and the profile Central's `Work` answers must be the pilot's
  (`--level expert` flies `house:expert`), else it makes no offers and
  says why (`ranked::fits`).
- **The runner token** (`bat_` and 64 hex digits) is what an administrator
  issues on Central's website for this machine, bound to the AI accounts
  it may play, shown once. It comes from the operator, never from an
  argument, a URL, the environment or a log: a file only the runner's
  service can read, its first line (`--token-file`; under systemd
  `LoadCredential=runner-token:/etc/baylee-ai/runner-token` and
  `--token-file ${CREDENTIALS_DIRECTORY}/runner-token`), or this machine's
  credential store (`--token-key <name>`, kept with `baylee-seat key set
  --key-env <name> --host <central>`, the token on stdin; §"Where a key is
  kept"). Either is read afresh at each exchange, and an error never
  quotes it (`ranked::RunnerToken`).
- **Its sessions.** The token is traded at Central for an ordinary native
  session of the account (`Account::ai_runner`): no master key, nothing
  restored (its library, document and chat refuse), and the terms,
  admission and the relay work as for any native session. The runner
  accepts Central's terms in force for the account before it offers, on
  its first session and on each fresh one it rolls over to, and again when
  an offer says they changed. An account may hold four runner
  sessions; the newest takes the offers and every new match, is renewed
  every five minutes, and is replaced by a fresh one every two hours
  (`ranked::ROLL_OVER`), well inside the ten hours of `drain_minutes`
  Central allows, and at once when Central will not renew it. An older
  session keeps the matches it took and is signed out once none holds it;
  a match whose session Central ended goes on with the newest.
- **The decks** are files on this machine, since a runner session cannot
  read the account's library: per format the newest version in
  `<decks>/<format>/vNNNN-<UTC stamp>.txt` (Baylee text with `# name:` and
  `# format:`, beside its `.json` record; `deckbuild::history`) that
  freezes and that the host would seat: checked against the host's
  legality facts (`ranked::admit` and its blocked cards), which the runner
  fetches as a guest from `--facts-from` (default the first
  `--accept-host`, unless `--facts-file`) at start and every 30 minutes and
  keeps as `<decks>/facts.json` (`deckbuild::facts`), or reads from the
  operator's `--facts-file` (never written), the fresher of the two
  counting. Facts are best effort: a fetch that fails keeps the facts
  saved and says why, and with no facts at all every deck that freezes is
  offered, the host's own check at seating standing behind it (a host
  that takes no guests, as the live gateway in its closed beta, gives
  none). A newer version that fails is passed over for the one before it,
  told once. Each deck is frozen as the native client freezes one, and its
  commitment is computed as a person's search computes it
  (`baylee_cards::ranked::registration`); every deck the runner offered
  stays with it, so a match made with it is played with it however the
  directory changed, and a runner that restarted finds an older version a
  match was made with in the directory.
- **A deck the host refuses.** The host checks the deck again when the
  seat registers it (`POST /ranked/{match}/play`: its catalog's verdicts
  and its blocked cards) and answers `400` for one it will not seat; the
  seat is then never claimed, Central voids the match once its two-minute
  reservation lapses (no score for either side, no change to either
  rating), and the person's search is over without a game. The runner
  takes two such refusals on a live session before the first seat as the
  host refusing the deck (`ranked::DeckRefused`), offers that deck no
  more, and offers the version before it at once, so the next lonely
  search does not meet the same refusal.
- **The chronicle.** After every match its game goes to the deck's
  chronicle (`deckbuild::history::record_game`,
  `<decks>/<format>/games.jsonl`: the version, the result, the turns, the
  Elo and its change, the opponent's AI label, never a person's name),
  which the deck builder reads.
- **Offers and work.** While the account is switched on, its pilot fits
  and is ready, it plays fewer matches than its `max_games` and has a deck,
  it offers one deck per format (`Ai::Offer`), renewed every 15 s (Central
  keeps one 45 s); otherwise it withdraws them, saying why once. Every 2 s
  it reads Central's work (`Ai::Work`: its settings and every live match)
  and plays each match it has not started as a task of its own, also one a
  runner before a restart took (the link resumes it), as long as it holds
  the deck the match was made with. `--games` stops it after that many.
- **The carousel** (baylee-central `docs/ai-players.md` §7: matches
  between AIs, for calibration). While it offers and has a slot free under
  `max_games`, the runner asks Central's carousel for a match in each
  offered format (`Ai::Carousel`), every 30 s; a seat it makes is played as
  any match. Central decides everything else (no person waiting, its caps,
  fill-in off while calibrating), and a refusal says when to ask again:
  `off` (the carousel is off for the account) in ten minutes, `offer` once
  the offers are renewed, `tomorrow` after the next midnight UTC, `later`
  after a wait from 30 s that doubles with each one in a row, jittered, at
  most ten minutes. On by default; `--no-carousel` asks for none.
- **A match.** The relay takes the match-bound admission for the account
  (`POST /auth/ranked/login` at the host), hands the host the frozen deck
  (`POST /ranked/{match}/play`, asked again with the same admission while
  the host answers `202`), and trades the seat for a one-use ticket to
  Central's `/v1/ranked/socket`; the seat is claimed by its match, which
  no queue entry holds (`Account::play_ai_seat`). The runner never dials
  the host itself. A dropped socket is replaced the same way, the host
  given the game's id instead of the deck, and the seat asks for what it
  missed. While the game runs, the admission is renewed at the `renew_in`
  the host gave. After the game the runner reads the match's result in
  Central's work (`Ai::Work`'s `finished`: the account's matches settled in
  the last day, at most 32: the seat, win, loss or draw, the Elo and its
  change, the opponent's AI badge), asked until it is there, at most two
  minutes (`ranked::Receipt`). Only where the list does not name the match
  is the result inferred from the account's standing in the format (none
  when another of its matches there settled meanwhile), and the
  transcript's `receipt` and the chronicle then say so.
- **Host terms.** Central asks every participant to approve the hosts of a
  match and their terms. The runner approves only a match whose hosts are
  all on the operator's list (`--accept-host <origin>`, repeatable) and
  declines any other, which releases the match without a score.
- **What plays: the pilot.** A `ranked::Pilot` plays one match's seat over
  the link it is handed (`ranked::Seat`: the link, the deck, the
  transcript, the pacing) and says what it is (`declared`, `kind`,
  `profile`) and whether it can take a game now (`ready`); the token, the
  sessions, the offers, the work, admission, renewal and shutdown are the
  runner's and the same for every pilot. `ranked::HousePilot` is the house
  at a fair level (`--level`; never `seer`), and `ranked::fly_bridge` plays
  any mind through the bridge. A pilot that plays its own core (the trained
  net) implements `Pilot` the same way, through the link's `Table`. One
  that cannot go on, because a resumed socket lost observations its memory
  needs and a snapshot cannot rebuild, answers `Piloted::HandOver(why)`:
  the runner plays the rest of the game with the house fallback
  (`--fallback-level`) on the same link, which declares the house to the
  table before its first answer, so the record names the house from there
  on, and the transcript says `handed_over` and why.
- **A language model** (`--profile <name>`, `ranked_llm` in the binary)
  is one model at one effort on a chain of access paths (the owner,
  26.11.2026: "primär A, dann B, dann C"): the settings file's profile
  (`--config`, default `<state>/llm-seat.json`) and the profiles its
  `fallback` names, in order, such as the model on a subscription's CLI,
  on another subscription, and behind the provider's API; most models have
  the one path alone, the house behind it. Unlike a `join`'s chain, a CLI
  may be one of the fallbacks, and every path must play the same model at
  the same effort. Each path is read as what it claims
  (`ranked::chain::claimed`: the model's family, the model, the effort):
  it names its model (a CLI's own default can change under it, so
  `<tool>:<model>`); `agy`'s tier in its id is its effort
  (`agy:claude-sonnet-5-5-low` is Sonnet 5.5 at low, unless `effort` names
  another); a model whose efforts this build knows names one in its
  profile, and one whose efforts it does not know (DeepSeek) names none
  and plays at its Standard (an account labelled `KI-DeepSeek-Flash ·
  Standard`). Two paths are one model (`ranked::chain::same_model`) where
  their efforts are equal, their families too where both are known, and
  their models equal or one a word of the other: Claude Code's `sonnet`,
  `agy`'s `claude-sonnet-5-5-low` and the API's `claude-sonnet-5-5`, each
  at low. A chain that crosses models or efforts is refused before any
  offer, saying what to change: the account's name and rating are the
  model's. Central's profile for the account is `model:<profile>`.
  - Before a game the pilot is ready while a path is not held back; with
    every path held back the account makes no offer. A path that was rate
    limited sits out new games until its limit lifts (as the provider
    said, else 30 minutes); one that failed, or whose free check (a
    model's entry, a CLI's login) fails before a game, for 10 minutes,
    doubled with each failure in a row up to 4 hours; one that cannot be
    reached (its key not kept yet, its program gone) likewise, from the
    start: the account starts whatever paths it can reach, and refuses to
    start only for a chain that is wrong (crossing models or efforts, a
    model or effort not named). A game starts on the first path not held
    back, the others standing by in order.
  - Mid-game the bridge hands the question down the chain at once on a
    limit or a failure (§"A fallback profile"), each path declaring its
    model to the table before its first answer, and past the last path
    the declared house (`--fallback-level`) plays the rest of the game
    (`ranked::chain::house_at_the_end`), so a ranked seat never leaves its
    table because its model is gone, and the record says what played.
  - Each path's game is in the spend book (`--ledger`, default
    `<state>/llm-spend.json`) and settles at the game's end with what it
    spent. By the owner's decision of 27.11.2026 nothing is capped: the
    file's day and month caps stay the mechanism and are absent unless
    written, and a profile that names no `game_usd`, `game_tokens` or
    `game_calls` plays without one, not with the bridge's defaults of $5
    and 500 CLI calls. Such a game is counted and never reserved where no
    cap holds its unit (`spend::reserve_or_count`: its entry reserves
    nothing, so a game killed outright leaves no budget it never had in
    the book); under a cap it reserves what is left of the cap, as any
    game does.
  - A second subscription of the same CLI needs that path to sign in as
    another account; a profile has no field for a CLI's own config
    directory yet, so today the second path is another program (its
    `command`) or the API.
- **Stopping.** A first ctrl-c or SIGTERM withdraws the offers and plays
  the running matches to their end (an AI does not leave a game; give the
  unit a `TimeoutStopSec` as long as a game); a second leaves them, and
  the sessions are signed out.
- **Single-host only.** Every ranked game runs on one host today, and the
  runner plays that path: ordinary seat frames, answers on the socket, and
  no presence reports (the native client sends none there either). A match
  replicated across two hosts is refused in one sentence; that link is
  still to be built.
- **Reports.** A ranked seat has no session at its host and the relay no
  route for reports. Without `--reports-to` a language model's reports
  stay in its reports file (`route: "none"` in the transcript), as they
  always did. With `--reports-to <origin>` (the official gateway,
  `https://gateway.baylee.app`; `https://`, or `http://` only to this
  machine; no default, so the machine's unit names it; it needs `--state`)
  they go to Baylee Central the way a client signed in nowhere sends a
  report (`POST <origin>/client/reports`, `docs/feedback.md` §"Straight
  from a client"; `ranked::reports`):
  - **What goes.** One report a game, every report of it inside
    (`reports::direct`): the chair route's bundle (above, §"Reports and
    lookups": `client.reports`, the seat's log, `record_sha256`, each
    report's `record_tail`, `dedupe_key` and checks), in the route's body
    and no other field: `{kind: "bug", text, build: {version, commit},
    device, client}`. There is no `game_id` beside it (the route takes
    none; `client.game_id` names the host's game) and no record (the seat
    has none). `client` adds `ai: {account, name, kind, profile}` (the AI
    account's id and public name, and the kind and profile it plays as;
    `model` and `profile` stay the mind that filed each report, a fallback
    too), `match_id`, `format` and `host` (the host's origin), never in
    place of the seat's own fields, and is held to the route's 1 MiB:
    the oldest log lines go first, then the last reports
    (`client.reports_omitted`). Hidden information stays as on every
    route: only what the table told the seat, its log in the client's
    shape with nobody's name.
  - **Who it comes from.** The route takes no credential. Each AI account
    has a device id of its own, 32 random hex digits made once and kept
    as `<state>/report-device`; the gateway files the report with Central
    marked `direct` under its keyed hash of that id
    (`BAYLEE_DIRECT_REPORT_KEY`, the gateway's), never the id, so triage
    sees one reporter per AI account, and the report names the gateway
    and this build. Before it is queued the body is sealed
    (`bugreport::seal`): one that holds the seat's socket ticket, the
    runner token or one of the account's Central sessions is not queued,
    and the transcript's `why` names which, never its value.
  - **Never in the game's way.** At the game's end the bridge writes the
    sealed report to the outbox, `<state>/reports-outbox/<uuid v7>.json`,
    and the transcript's `llm_reports` says `route: "direct"`, `sent:
    false` and `why: "queued: the runner sends it to <origin>/client/reports"`.
    The runner's sender posts what waits there, oldest first, when a
    report is queued and every 15 s, and removes a report only once the
    gateway answers `201` (Central filed it). A limit (`429`: ten an hour
    from one address, forty from all direct reporters; the runners of one
    machine share its address), a gateway that does not take direct
    reports now (`503`), any other answer or none keeps it, and the sender
    tries again after a minute, then twice as long each time up to an
    hour, jittered. A report the gateway refuses for what it is (`400`,
    `413`) would be refused again: it moves to `<state>/reports-refused/`
    with the answer beside it as `<name>.why`. None is deleted unsent:
    a runner that stops tries once more for up to 30 s (not after a
    second signal) and leaves the rest in the outbox for its next start.
    The reports file keeps every report as before, whatever the route.

`cargo dev central integration --ai-players-only` proves it end to end
against the pinned Central (its crates run in the test, so the pin is the
Central tested): an administrator creates `KI-Haus-Expert` on the website,
switches it on and issues a runner token for it; the runner trades it,
accepts the terms and offers sixty Forests; a person searches with house
opponents allowed, and after the fill-in wait Central seats the AI; the
person's seat is played by the house through their own queue entry, the
AI's by its runner with a pilot that cannot resume: its first socket is cut
mid-game, it hands the seat over, and the house fallback takes the seat
again with a fresh core and plays on. Both see the whole game, the
person's assignment and receipt carry the AI's badge, and Central's
results agree with the table's
(`crates/baylee-gateway/tests/e2e_central/ranked/runners.rs`). Beside it,
two house AIs, each its own runner, with fill-in off and the carousel on,
are paired by the carousel in Standard and finish the game, Central's
result for each (not inferred) agreeing with the table's.

The same test plays a proof game of a real pilot on the machine whose CLIs
are signed in, when `BAYLEE_E2E_AI_RUNNER` names a runner binary: the
administrator creates the account of `BAYLEE_E2E_AI_KIND` and
`BAYLEE_E2E_AI_PROFILE` (default `house`, `house:expert`; a model gets a
day's cap of ten games), the harness writes the token to a `0600` file and
starts `$BAYLEE_E2E_AI_RUNNER ranked --central <served> --account <id>
--token-file <file> --state <dir> --accept-host <host> --games 1
$BAYLEE_E2E_AI_ARGS`, with `BAYLEE_E2E_AI_DECKS` copied into its state as
its deck directory (the person plays its newest Standard deck too; default
sixty Forests). The person searches accepting any AI for a model and the
local ones otherwise, and plays the house. The test checks the person's
receipt against the table, reads the AI seat's transcript, and prints one
line (also kept as `<target>/tmp/ai-players-runners/summary.txt`):
whether the AI won or lost and at which seat, the turns and how the game
ended, the questions that expired, and its answers counted apart: decided
(every answer the pilot's mind gave, whatever mind it is, a house pilot's
own choices too, with their think time: median, P95, most), instant (only
what the standing orders answered with no mind asked), house (only what
the declared fallback answered after the pilot handed the seat over) and
fallback (what the seat's own house or least answer gave before any
hand-over, where the pilot's mind failed or ran out of time). A failing
runner, or one that
does not stop after its game, fails the test, and the process is killed
either way.

```text
cargo build -p baylee-seat        # or the trained net's seat4
BAYLEE_E2E_AI_RUNNER=$PWD/target/debug/baylee-seat \
BAYLEE_E2E_AI_KIND=model BAYLEE_E2E_AI_PROFILE=model:haiku-high \
BAYLEE_E2E_AI_ARGS="--profile haiku-high --config /path/llm-seat.json" \
BAYLEE_E2E_AI_DECKS=/var/lib/baylee-ai/<account>/decks \
  cargo dev central integration --ai-players-only
```

## A dev table

`cargo run -p xtask -- dev-table --bridge profile:<name>` seats the bridge
with `--profile <name>` instead of `--mind`, from the same file and under
the same caps (`BAYLEE_SEAT_CONFIG` passes through).

## Deck building

An AI account keeps its own decks (`baylee_seat::deckbuild`,
`docs/proposals/ai-players.md` §"Decks"). Central gives a runner session no
library of the account's, so each AI's decks are files on the runner's
machine, in a directory of the AI's own. The ranked runner offers the newest
deck of each format there (`deckbuild::history::newest`).

```text
baylee-seat deck facts   --gateway https://<host> [--out <dir>/facts.json]
baylee-seat deck check   --format <f> --facts <file> <deck file>
baylee-seat deck lend    --format <f> --dir <dir> [--seed 1] [--facts <file>] [--exclude <file>]
baylee-seat deck build   --format <f> --dir <dir> (--profile <p> | --mind <m>)
    [--config <file>] [--ledger <file>] [--effort <e>] [--fallback-profile <p>]…
    [--no-fallback] [--rounds 40] [--reply-secs 300] [--facts <file>] [--exclude <file>]
baylee-seat deck improve (the options of build) [--most 6] [--recent 20] [--games <file>]…
```

`<f>` is `standard`, `highlander` or `commander`. `--facts` defaults to
`<dir>/facts.json`.

- **The directory.** Each format has its own subdirectory,
  `<dir>/<format>/v0007-20261127T031500Z.txt` (the deck) and `.json` (its
  record: how it was made (`build`, `improve` or `lend`), the model, profile
  and effort or the lend's seed, the parent version, the cards swapped, the
  model's note, the facts version it passed against, and what the work
  spent). A version is never rewritten. Each version number is taken under a
  lock on `<dir>/<format>/.lock`. Both files are written through a temporary
  file, the record first, so a deck never stands without its record.
  `<dir>/<format>/games.jsonl` is the format's games, one line each
  (`history::record_game`: when, the version that played, `won`, `lost`,
  `draw` or `unrated`, turns, Elo and its change, the opponent's public
  label, a note). The runner appends a line after each match, and `deck
  improve` reads it. An opponent is told to the model by its label only when
  that is an AI's (`KI-`, `LLM-`, `HOUSE-`, …); a person is "a person".
- **The deck file** is Baylee text (`docs/deck-format.md`): `# name:`,
  `# format: <f>`, plain `N Name` rows, `CMD:` for a commander, no sideboard.
  It is one of the four formats `deck.rs` reads. Before a version is kept,
  its file is read back and checked again.
- **The facts** (`deckbuild::facts`). A ranked host admits a deck only when
  its catalog has a verdict for every card, so a deck is checked against that
  host's own table. `facts::fetch` signs in as the guest `KI-Decks`, reads
  `GET /pool/facts` (`?have=` keeps an unchanged table on `304`) and
  `GET /cards/blocked`, and signs out; the guest is deleted with its session.
  It never uses the AI account's own session, because a second sign-in ends
  that account's seats. A host that takes no guests is said in those words.
  So is one that asks a guest to accept its terms first, which this does not
  do for anybody. The file holds the gateway's origin, when it was read, the
  table and the blocked cards. An empty table (a gateway without a catalog)
  is refused, since no deck could pass against it.
  A host that lets no guest in writes the same file on itself:
  `baylee-gateway pool-facts --out <file>` reads its own database
  (`DATABASE_URL`, as it stands) with the route's own code, names itself by
  `BAYLEE_PUBLIC_URL`, opens no port and writes the file atomically, mode
  0644; its operator copies it to the runner.
- **The check** (`deckbuild::check::Checker`) is the host's own code, in the
  order `crates/baylee-gateway/src/ranked/deck.rs` runs it:
  1. every row names a card this build plays;
  2. `baylee_cards::ranked::registration` (counts, sizes, commanders);
  3. the facts' blocked cards in force now;
  4. `baylee_cards::ranked::admit` against the table (the format's shape,
     every card's verdict under its catalog key `standard`, `vintage` or
     `commander`, rules-complete coverage, Highlander's singleton over deck
     and sideboard, circled colours inside the commanders' identity);
  5. `ranked::Frozen::from_rows`, which is how the runner freezes what it
     offers.

  `admit` answers one sentence for every fault. The model is told each fault
  on its own instead, by card (`«Counterspell» is banned in standard`,
  `«Made Up Card» is no card this build knows`,
  `«X» is a real card, but not one this build plays`), worded from the same
  checks. A deck passes only when every step does, and the tests hold the
  two to the same answer over a table of accepted and refused decks. What
  this check cannot see is a block or a catalog change after the facts were
  read. The host checks again at seating, so read the facts again before
  relying on an old deck.
- **An exclusion file** (`--exclude`, `deckbuild::exclude`) names cards a
  pilot's decks never hold, whatever the format allows: one exact name a
  line, as the registry spells it; `#` starts a comment. A name that is no
  card of this build refuses the file, naming its line. The trained net plays
  an export made on an older card catalog and reads some cards only by
  summary, and its agent hands that list over. Excluded cards are out of the
  pool, never lent, and a proposal holding one is refused by name. The
  version's record keeps the file's SHA-256 and how many cards it named
  (`excluded`).
- **The pool** (`deckbuild::pool`) is the cards a deck of the format may
  hold: rules-complete here, playable in the format by the table, not
  blocked. Cryptic Spires is left out, because it needs two circled colours
  chosen. The model never gets the pool pasted. It searches it (words of the
  name, type line or rules text, type words, a colour identity, mana values,
  commanders only, pages of 25) and reads cards whole (this build's card
  text, `lookup::compiled`). A name outside the pool is refused by the check
  with its reason.
- **The conversation** (`deckbuild::work`). Every reply is one JSON object
  with one action: `search`, `read` (at most 10), `propose` (the whole deck:
  `name`, `commanders`, `cards` as `{count, name}`, and `note`, its plan in
  two to six sentences) or, for `improve`, `change` (`out`, `in`, `note`),
  plus an optional `say`. A proposal is checked at once. A refusal lists its
  faults, at most 40, and the model answers again, until a deck passes or
  `--rounds` replies are spent. A conversation that begins again (a CLI
  process lost, a fallback taking over) is first told the task and the last
  proposal with its faults.
- **The model** is chosen as a seat's mind is (§"Which model plays, with
  what"): the profile or `--mind`, its key from the environment or this
  machine's credential store, never an argument. An agent CLI plays through
  `CliMind::converse` under the seat's lockdown. Its brief (`cli::Brief`)
  carries the builder's instructions and answer schema in place of the
  game's, and a seat's brief is unchanged byte for byte. An API plays
  through `llm::deckchat`: one tool, `act`, whose input is the answer object,
  or that object as JSON where the profile answers `json` or `json_schema`.
  The profile's fallbacks, or `--fallback-profile`, take over in order when
  a model is limited, failing or out of budget.
- **Spend.** Under a settings file each model of the chain reserves its
  profile's game budget in the spend book before the work begins, as a game
  does, but under no day or month cap (owner, 27.11.2026: no caps for the
  AIs' deck work; the book still records it).
  It settles with what it spent when the work ends however it ends (Ctrl-C
  and errors included); a fallback that never answered settles at nothing.
  The record keeps the replies, calls, tokens and, where the price is known,
  the dollars.
- **Lent decks** (`deckbuild::lend`), for the pilots that cannot build (the
  fair house levels, the trained net). The deck generator
  (`baylee_cards::deckgen::build_synergistic`) builds from the format's pool
  in its shape, and the deck is held to the same check. A refused seed is
  passed over for the next, up to 64, and so is a Standard deck longer than
  sixty (the generator makes one in four of 61 to 75 cards, for its training
  variety). The archetype goes round with the seed. The same seed and the same facts lend the same deck. The precons are
  not drawn on: of the 89 this build plays, none is a hundred-card singleton
  or Commander deck, and the sixty-card ones are lists of the 1990s.
- **Improving** reads the newest version, its note, the format's last
  `--recent` games and any `--games` files (each held to 6,000 characters,
  24,000 in all). It lets the model move at most `--most` cards, each copy
  in or out counting once. The result is held to the check, and the change
  is kept as a new version with `parent`, `removed` and `added`.
- **Tests** reach no model. A scripted chat answers fixed objects
  (`deckbuild/tests.rs`), and a stand-in Messages API answers
  `llm::deckchat`. The fake agent CLI converses as Claude Code and as Codex,
  and plays `deck build` end to end (`tests/cli_mind/deck.rs`). The facts
  are read from a stand-in gateway. Measured against a real catalog
  (27.11.2026: a local gateway in `guests` mode on a copy of the full
  catalog, its default terms of 2026-10-21 in force): a fresh guest read
  both routes without accepting them; of the 25,824 cards this build plays
  completely, 3,846 are in the Standard pool, 25,185 in Highlander's and
  25,153 in Commander's; four seeds a format each lent a deck the host's
  check passed, none passed over but one Standard seed of more than sixty
  cards.

## What the game's record says played

A game with a bridge at it is recorded like any other (`docs/protocol.md`
§"The game record (#315)"), and the bridge tells the table what answers
its seat, so the record says which model made which action
(`docs/protocol.md` §"Who answers a seat, as it says"). It is the mind as
chosen above, flags over profile over build (`baylee_seat::declare`):

- `--mind house` (or nothing named): `house` and its `--level`, a fair
  level or a frozen reference (`sharp-2026-11`); `seer` reads hidden cards,
  which a seat is never sent, and is refused in a sentence naming `expert`
  (`baylee_seat::house::level`, `docs/house-ai.md`);
  it answers as the table's own house chair does, from the view, the note
  beside the question and the deck it brought (`docs/house-ai.md`
  §"The seated house"), and so does every house fallback;
- `--mind scripted`: `scripted`;
- a model behind an API: `llm_api`, the provider (`anthropic`, `openai`),
  the exact model id and the effort, as sent to the provider;
- a model behind a CLI: `llm_cli`, the tool (`claude`, `codex`, …), the
  model it names (empty for the tool's own default) and the effort.

Each fallback that takes over ([above](#a-fallback-profile)) declares its
own model before its first answer, so the record names it from there on,
and the next one's from where that one took over.
Never the profile's name, its `base_url`, its key variable, a key or any
prompt: the declaration has no field for them, and the engine refuses one
whose text is shaped like a key or an address. It is sent on every socket
before the seat says it is ready, so a bridge that comes back after its
mind was down declares again; a mid-game swap is `SeatCore::declare`,
which nothing in the bridge calls yet (a debug swap would). It is
self-declared: the record labels it `declared_mind`, and nobody at the
table is shown it.

What the record does not split: within one declaration the bridge's own
standing answers (a pass with nothing to do), its house fallback (a model
too slow, refused or out of budget) and its least answer go to the table
as the seat's answers, like the model's. Which of them made each answer is
in the bridge's transcript (`--transcripts`: an `answered` event's `by`), and the
counts in its closing line. A decision the seat makes for another player
whose turn it controls (Mindslaver, CR 724.5) is the seat's answer in the
record too, made the same way: the standing orders (read from that
player's chair), the model, the house, the least answer. The model plays
that player's turn with that player's cards and mana, against that
player's interest where it likes, and never concedes for that player
(CR 724.6): a concession there is refused and asked again, and the house
answers after the retries. `controlled` in the closing counts says how
many such decisions there were, the transcript's `asked` line names the
player (`controlled`), and how the model is told one is
`docs/llm-protocol.md` §"A decision for a controlled player". Until
4 Nov 2026 the least answer made every one of them: the controlled
player passed, attacked with nothing and made the first choice it had
to. A mind taken off the table (`down_after`
failures) leaves the socket, and the table's own stand-in, recorded as a
`chair` line, plays until it is back. A limited mind is never taken off
the table: the bridge declares the house (`kind: house` and its level) and
its house plays the rest of the game on the same socket, so the record
names the house from that answer on.

## The AI log (debug builds only)

Beside each answer its model made (not a standing answer, a plan's tap or
the house's), a debug bridge sends the table what the mind said: its note
(the answer chosen, its `say`, tokens and time) and the model's reasoning,
each cut at a character boundary to 16 KiB. A debug engine forwards it to
every other seat at the table, the other side included, and a debug
client shows it in its AI log panel: a tool for watching a model play and
for sparring against one. The reasoning reads the model's hand out loud,
so this is hidden information shown on purpose. A release build has none
of it: the bridge sends nothing, the engine drops what a debug bridge
sends, and the client stands no panel (`docs/protocol.md` §"An AI seat's
reasoning"). The full transcript stays with the bridge either way
(`--transcripts`).
