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
| `max_tokens` | The most one reply may take (default 16000 Anthropic, 8000 OpenAI-compatible). A CLI takes no such limit: for one it is only what a call is held at for its reply (default 16000). |
| `price` | `{"input": …, "output": …}`, US dollars per million tokens: the price of a model this build has none for, or a better one. It applies to the profile's own model only. A cache write is billed at 1.25× the input price for five minutes' entry and 2× for an hour's (the API path caches the game's constant head for an hour, `docs/llm-protocol.md` §"The cache"), a cache read at the input price unless the build knows better. |
| `game_usd` | The most one game may spend in dollars (default $5); only for a model with a price. |
| `game_tokens` | The most one game may spend in tokens, in and out (default 5,000,000; 20,000,000 for `cli`); the limit of a model without a price. |
| `game_calls` | The most calls one game may make; past it the house finishes the game (default 500 for `cli`, no limit for an API). |
| `think_secs` | The longest one answer may take (default 60). |
| `key_env` | The environment variable the key is read from (default `ANTHROPIC_API_KEY`, or `BAYLEE_LLM_API_KEY` for `openai`), and, with the address's host, the name a key is kept under in this machine's credential store ([below](#where-a-key-is-kept)). Not for `cli`. |
| `base_url` | Where the API is: `https://`, or `http://` on loopback only. Not for `cli`. |
| `command` | `cli` only: the tool's program, a whole path; default: the tool's name on the bridge's `PATH`. Name a version's own file to play that version (`/Users/<you>/.local/share/claude/versions/<version>`, [below](#a-cli-as-the-model)). |

A model without a price in this build plays only with a `price` or a
`game_tokens`, as `--spend-tokens` on the command line (`join --help`);
where the caps count dollars a day or a month, it also needs the token cap
of that period, or its spend would count nowhere.

## Where a key is kept

In the environment or in this machine's credential store, never in the
file. A profile names the variable its key is read from (`key_env`); the
bridge reads that variable first, and where it is unset, the key kept in
the operating system's credential store under the same name for the
profile's address: the macOS Keychain, the Windows Credential Manager, the
Secret Service on Linux (the `keyring` crate, Apache-2.0 or MIT; only
`baylee-seat` links it).

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
DeepSeek's models none it can name). Elsewhere the chair plays the model's
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
  `--no-session-persistence` forbids. (`agy` takes no system prompt as a
  flag, so ours rides ahead of the prefix in the first message.) Codex,
  opencode and Junie answer one message a process (`Dialect::one_shot`):
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
- **Locked down.** The working directory is a fresh, empty directory under
  the OS's temp directory, readable by this user alone and removed with
  the process, so no project's `CLAUDE.md` or settings are found. The
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
  (`dialect::Event::Breach`).
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
  (`Dialect::usage_spans_resumes`); Junie's result counts its task (one a
  process, booked whole), and opencode's `step_finish` counts its own
  step. Under the caps a cli game reserves its `game_tokens` against
  `day_tokens` and `month_tokens`, so a day's cap of 20,000,000 holds one
  game; raise the cap, or lower `game_tokens`. `game_calls` (500 by
  default, `--spend-calls`) is held like a budget, and the summary says
  `212 of 500 calls`. A reply that does not say what it used counts at its
  worst; a process that ended without a word (a session not found, a
  crash before its start) reached no model, and is a failed call at no
  cost.
- **Rate limits.** A rate limit or a spent quota is unavailable, unbilled:
  the house answers, and the mind cools down for the time the tool names
  when that is some time and at most fifteen minutes, else a minute,
  doubling to fifteen. It plays again once that has passed
  and the tool's login check (which calls no model) passes: `claude auth
  status --json` says `"loggedIn": true`; `codex login status` says
  `Logged in using …` on stderr, not with an API key; `opencode auth list`
  names an `oauth` credential, not a stored key. Output it cannot read
  counts as signed out. `agy` and Junie have no such check: their
  `--version` shows only that the program runs.
- **Tests.** Only against `examples/fake-agent-cli.rs`, a stand-in that
  speaks Claude Code's stream-json, `agy`'s, and each one-shot tool's
  output, and logs what it was started with and every message it read
  (`tests/cli_mind.rs`: one process across turns with the prefix sent
  once, a process that died between turns or mid-question begun again
  with the prefix; a one-shot tool's process a question, a tool used
  taking the mind off the table). Each dialect's arguments are golden and
  its lines are read from fixtures written from its documentation or
  source (`cli/dialect_tests.rs`); no dialect may pass an approving flag
  or a credential in its arguments. No test starts a real CLI or reaches a
  model.

### The tools

What each dialect passes to take the tool's own agent away, how it holds
a conversation, and how its usage is booked. **Docs** marks what the
tool's help (`--help` of the installed version, 06.10.2026), documentation
or source says; **assumed** what no source confirmed and a first live game
must show (a tool that refuses a flag fails its game with a sentence; it
never plays unlocked). The models and effort levels the settings panel
offers for each are `baylee_client_core::llmseat::clis::choices`, a pure
table (no program is asked).

| CLI | Flags and files that strip its overhead | Conversation | Usage | Lockdown check | Confirmed |
|---|---|---|---|---|---|
| Claude Code `claude` (2.1.290) | `-p`, stream-json both ways, `--restricted --safe-mode --tools "" --strict-mcp-config --disable-slash-commands --setting-sources "" --permission-prompts none --permission-mode manual --no-session-persistence --system-prompt <ours> --json-schema <answer>`, `--model`, `--effort`; `DISABLE_AUTOUPDATER=1` | one process across turns, stdin lines (not `--resume`: transcripts and the login share `CLAUDE_CONFIG_DIR`) | per turn (`result.usage`) | `init`: tools only `StructuredOutput`, no MCP server, no slash command, key source named and not a variable | docs (help, Agent SDK) |
| Antigravity `agy` (1.2.17) | stream-json both ways, `--disable-slash-commands`, `--json-schema`, `--model`, `--effort`; ours ahead of the first message (no system-prompt flag). It has **no** flag that removes tools, MCP servers or its user rules (`~/.gemini`); print mode soft-denies what asks approval | one process across turns, stdin lines (its sessions under `~/.gemini/antigravity-cli` cannot be moved) | running count per process, booked as differences | `init` must name its tools and connect no MCP server; a `tool` step is a breach | flags: docs; cumulative usage: recorded games |
| Codex `codex` (0.160.0) | `exec --json --ignore-user-config --ignore-rules --skip-git-repo-check --sandbox read-only --output-schema <file>`; `-c model_instructions_file=<ours> project_doc_max_bytes=0 mcp_servers={} notify=[] web_search="disabled" tools.view_image=false history.persistence="none" analytics.enabled=false feedback.enabled=false otel.exporter="none" check_for_update_on_startup=false include_*_instructions/context=false model_reasoning_summary="none" [model_reasoning_effort]`; `--disable` each tool feature (shell, exec, image, plugins, apps, hooks, skills, sub-agents, memories, browser, computer use, …); `--model`; prompt on stdin (`-`); going on: the same, then `resume <thread uuid> -`. `$CODEX_HOME/AGENTS.md` is still read (the bridge warns where one is) | one process a question, going on by `exec … resume <id>`; sessions in `$CODEX_HOME/sessions` beside the login, the seat's own removed | thread total, read back on resume: booked as differences across the conversation | none at start (`thread.started`); any item but the answer, reasoning or a warning is a breach | flags: docs and source (main, 06.10.); features list and AGENTS.md: source, not the 0.160 tag; **assumed**: `-c` keys 0.160 does not know are ignored; that `exec`'s `--sandbox` and `--color` ahead of `resume` hold for it (source: global flags) |
| opencode `opencode` (1.18.34) | `run --pure --format json --agent seat --title seat [--model p/m] [--variant v]`, message on stdin; `OPENCODE_CONFIG` = a file denying every tool (`permission {"*":"deny"}`, also on the agent), `mcp {}`, no instructions, share/snapshot/formatter/compaction/autoupdate off, agent `seat` whose prompt is ours; `XDG_CONFIG_HOME` = an empty directory; `OPENCODE_DB` = a file in the seat's store; `OPENCODE_DISABLE_PROJECT_CONFIG`, `_CLAUDE_CODE`, `_EXTERNAL_SKILLS`, `_AUTOUPDATE`, `_AUTOCOMPACT`, `_LSP_DOWNLOAD`. opencode still adds its environment block (model, directory, date) | one process a question, going on by `--session <id>` from the seat's store: only what is new is sent | per step (`step_finish.tokens`) | none at start (its first line is the start); a `tool_use` line is a breach | source (v1.18.34 tag, `dev` for the schema); **assumed**: built-in plugins left on (they carry the logins); that a resumed `run` reads its history whole (source: `dev`) |
| Junie `junie` (26.9.22) | `--input-format=json --output-format=json-stream --skip-update-check --share-anonymous-statistics=false --config-default-locations=false --mcp-default-locations=false --skill-default-locations=false --command-default-location=false --agent-default-location=false --model-default-locations=false --agent-mode=chat --extensions-default-location=<empty> --guidelines-filename=<empty file> --cache-dir=<session's> --system-prompt=<ours>` (added to Junie's, not replacing it), `--model`, `--effort`; task as `{"task": …}` on stdin; going on: the same and `--session-id=<id>` (never `--resume` alone) | one process a question, following up by `--session-id`; sessions in `~/.junie/sessions` beside its settings, the seat's own removed | the task's per-model records, summed, booked whole (a follow-up is a new task) | none at start (`session`); any step but `TASK RESULT` is a breach | flags: help; output shape: a JetBrains fixture and action; **assumed**: that a signed-in account plays headless without `--auth`, what chat mode does, that no tool step appears in a plain answer, that `--session-id` alone follows a session up (help: "the previously executed session to follow up") |

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
  `$JUNIE_HOME/sessions/<id>/`. When the conversation is over the seat
  removes exactly the files named for the ids its own processes named
  (one that strayed included), matched by the tool's own file naming,
  never through a link; the store notes those ids, so the sweep after a
  killed bridge removes them too. Nothing else of the user's is touched:
  Codex's thread row in its own state database and the line Junie's
  `sessions/index.jsonl` holds for the session stay. No session file is
  read: Codex's `turn.completed` already carries the thread's count, and
  Junie's result its task's. Junie may begin a new session silently for
  one it does not find, which its `session` line then names: that is a
  process that strayed. One it begins anew under the very id asked for
  could not be told from a kept one without reading its files.
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
  client).
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
  (`BAYLEE_GUESTS=off`), its guest cap is reached, or it is a closed beta
  (`BAYLEE_REGISTRATION=invite`). The room lists the chair under the
  bridge's name with the host's handle (`delegated_by`), and the game's
  record names the host as the one who answers for it
  (`game_record_seat.delegated_by`), never as the one who played it. A
  host who is a guest is handed no ticket: its bridge signs in as a guest
  named for its mind (`GuestSignIn`), as before, where the gateway takes
  guests, and the chair's card says why not where it takes none. So does
  an older gateway that sells no tickets (`404`): the guest door, where
  there is one.
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
- **Limits.** A blitz table (30 seconds or less a decision) is refused by
  the bridge, as from a terminal. The chair keeps the house level `steady`
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

## A dev table

`cargo run -p xtask -- dev-table --bridge profile:<name>` seats the bridge
with `--profile <name>` instead of `--mind`, from the same file and under
the same caps (`BAYLEE_SEAT_CONFIG` passes through).

## What the game's record says played

A game with a bridge at it is recorded like any other (`docs/protocol.md`
§"The game record (#315)"), and the bridge tells the table what answers
its seat, so the record says which model made which action
(`docs/protocol.md` §"Who answers a seat, as it says"). It is the mind as
chosen above, flags over profile over build (`baylee_seat::declare`):

- `--mind house` (or nothing named): `house` and its `--level`;
- `--mind scripted`: `scripted`;
- a model behind an API: `llm_api`, the provider (`anthropic`, `openai`),
  the exact model id and the effort, as sent to the provider;
- a model behind a CLI: `llm_cli`, the tool (`claude`, `codex`, …), the
  model it names (empty for the tool's own default) and the effort.

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
counts in its closing line. A mind taken off the table (`down_after`
failures) leaves the socket, and the table's own stand-in, recorded as a
`chair` line, plays until it is back.

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
