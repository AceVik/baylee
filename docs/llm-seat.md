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
temporary file, mode `0600` on unix).

Unknown fields are refused, and so is every fault, in one sentence naming
the profile: a file is played as written or not at all.

| Field | Meaning |
|---|---|
| `default` | The profile played when `--profile` names none. |
| `caps.day_usd`, `caps.month_usd` | The most the games of models with a price may spend together per calendar day and per month, in US dollars. |
| `caps.day_tokens`, `caps.month_tokens` | The same in tokens, for the games of models without a price. |
| `profiles.<name>` | Up to 32 letters, digits, `-` and `_`, starting with a letter or digit. |
| `provider` | `anthropic`, `openai` (any OpenAI-compatible endpoint) or `cli` (an agent CLI, [below](#a-cli-as-the-model)). Required. |
| `model` | The provider's model id; for `cli` the tool, then its own model if any: `claude`, `claude:opus`. Required. |
| `effort` | A word such as `low`, `medium`, `high` (default medium on Anthropic, the endpoint's or the CLI's own elsewhere). |
| `answer` | `tools`, `json` or `json_schema` (the last two for OpenAI-compatible endpoints and CLIs; a CLI answers only these, `json_schema` by default). `json` asks the endpoint for a JSON object (`response_format` `json_object`, which `DeepSeek` takes); `json_schema` for one held to the answer's schema (`json_schema`, for an endpoint that refuses a bare object, such as LM Studio). Either way the model is told the answer's fields in its instructions, and when an endpoint turns the one mode down, the error says to try the other. |
| `max_tokens` | The most one reply may take (default 16000 Anthropic, 8000 OpenAI-compatible). A CLI takes no such limit: for one it is only what a call is held at for its reply (default 16000). |
| `price` | `{"input": …, "output": …}`, US dollars per million tokens: the price of a model this build has none for, or a better one. It applies to the profile's own model only. A cache write is billed at 1.25× the input price for five minutes' entry and 2× for an hour's (the API path caches the game's constant head for an hour, `docs/llm-protocol.md` §"The cache"), a cache read at the input price unless the build knows better. |
| `game_usd` | The most one game may spend in dollars (default $5); only for a model with a price. |
| `game_tokens` | The most one game may spend in tokens, in and out (default 5,000,000; 20,000,000 for `cli`); the limit of a model without a price. |
| `game_calls` | The most calls one game may make; past it the house finishes the game (default 500 for `cli`, no limit for an API). |
| `think_secs` | The longest one answer may take (default 60). |
| `key_env` | The environment variable the key is read from (default `ANTHROPIC_API_KEY`, or `BAYLEE_LLM_API_KEY` for `openai`). Not for `cli`. |
| `base_url` | Where the API is: `https://`, or `http://` on loopback only. Not for `cli`. |
| `command` | `cli` only: the tool's program, a whole path; default: the tool's name on the bridge's `PATH`. Name a version's own file to play that version (`/Users/<you>/.local/share/claude/versions/<version>`, [below](#a-cli-as-the-model)). |

A model without a price in this build plays only with a `price` or a
`game_tokens`, as `--spend-tokens` on the command line (`join --help`);
where the caps count dollars a day or a month, it also needs the token cap
of that period, or its spend would count nowhere.

## Keys

No key is ever in the file. A profile names the variable its key is read
from, and the bridge reads it from the environment. A field named like a
key (`api_key`, `key`, `token`, `secret`, `password`, `authorization`, …),
a profile's name included, or a name or value shaped like one refuses the
file (`llmseat::shaped_like_a_key`: the generic `sk-`, `Bearer ` and
`x-api-key` where a word starts, so `risk-free` is none; the long
provider markers `sk-ant-`, `sk-proj-`, `ghp_`/`gho_`/`ghu_`/`ghs_`/`ghr_`
and `github_pat_` with twenty key characters after them wherever they
stand, so a key glued to a model id is one) with a sentence saying
where keys go, and so does a command line that carries the key of any
variable the bridge would read. A profile's key goes only to the address
written beside it: a profile's `base_url` beats `ANTHROPIC_BASE_URL` and
`BAYLEE_LLM_BASE_URL`, and without one the environment's address, then the
provider's, is used.

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
  a process whose stdin is closed after the message. opencode goes on with
  its conversation by its id (`--session <id>`, `Dialect::resumes`) from a
  store of the seat's own, so each later question sends only what is new,
  as to a long-lived process; one it cannot resume (not found, or not
  read) is begun again for that question, with the prefix, the notes and
  that it was lost. With Codex and Junie, whose sessions cannot be kept
  apart from their login (§"The tools"), **each question is a
  conversation of its own**: the prefix and the seat's notes every time
  (the provider's cache reads the unchanged prefix back), not counted as a
  loss. An answer that cannot be read is asked again of a new process:
  opencode's goes on with the conversation and hears only why, the others'
  hear the whole question and why.
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
  (`Dialect::usage_is_cumulative`). Codex's and Junie's counts are their
  run's whole (one message a process, so each is booked whole), and
  opencode's `step_finish` counts its own step. Under the caps a cli game reserves its `game_tokens` against
  `day_tokens` and `month_tokens`, so a day's cap of 20,000,000 holds one
  game; raise the cap, or lower `game_tokens`. `game_calls` (500 by
  default, `--spend-calls`) is held like a budget, and the summary says
  `212 of 500 calls`. A reply that does not say what it used counts at its
  worst.
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
| Codex `codex` (0.160.0) | `exec --json --ephemeral --ignore-user-config --ignore-rules --skip-git-repo-check --sandbox read-only --output-schema <file>`; `-c model_instructions_file=<ours> project_doc_max_bytes=0 mcp_servers={} notify=[] web_search="disabled" tools.view_image=false history.persistence="none" analytics.enabled=false feedback.enabled=false otel.exporter="none" check_for_update_on_startup=false include_*_instructions/context=false model_reasoning_summary="none" [model_reasoning_effort]`; `--disable` each tool feature (shell, exec, image, plugins, apps, hooks, skills, sub-agents, memories, browser, computer use, …); `--model`; prompt on stdin (`-`). `$CODEX_HOME/AGENTS.md` is still read: give the seat a `CODEX_HOME` of its own | one process a question, each a new conversation (`exec resume <id>` is not used: sessions and the login share `CODEX_HOME`) | thread total, one turn a process: booked whole | none at start (`thread.started`); any item but the answer, reasoning or a warning is a breach | flags: docs and source (main, 06.10.); features list and AGENTS.md: source, not the 0.160 tag; **assumed**: `-c` keys 0.160 does not know are ignored |
| opencode `opencode` (1.18.34) | `run --pure --format json --agent seat --title seat [--model p/m] [--variant v]`, message on stdin; `OPENCODE_CONFIG` = a file denying every tool (`permission {"*":"deny"}`, also on the agent), `mcp {}`, no instructions, share/snapshot/formatter/compaction/autoupdate off, agent `seat` whose prompt is ours; `XDG_CONFIG_HOME` = an empty directory; `OPENCODE_DB` = a file in the seat's store; `OPENCODE_DISABLE_PROJECT_CONFIG`, `_CLAUDE_CODE`, `_EXTERNAL_SKILLS`, `_AUTOUPDATE`, `_AUTOCOMPACT`, `_LSP_DOWNLOAD`. opencode still adds its environment block (model, directory, date) | one process a question, going on by `--session <id>` from the seat's store: only what is new is sent | per step (`step_finish.tokens`) | none at start (its first line is the start); a `tool_use` line is a breach | source (v1.18.34 tag, `dev` for the schema); **assumed**: built-in plugins left on (they carry the logins); that a resumed `run` reads its history whole (source: `dev`) |
| Junie `junie` (26.9.22) | `--input-format=json --output-format=json-stream --skip-update-check --share-anonymous-statistics=false --config-default-locations=false --mcp-default-locations=false --skill-default-locations=false --command-default-location=false --agent-default-location=false --model-default-locations=false --agent-mode=chat --extensions-default-location=<empty> --guidelines-filename=<empty file> --cache-dir=<session's> --system-prompt=<ours>` (added to Junie's, not replacing it), `--model`, `--effort`; task as `{"task": …}` on stdin. Sessions are still kept under `~/.junie/sessions` | one process a question, each a new conversation (`--session-id --resume` is not used: see below) | the task's per-model records, summed, booked whole | none at start (`session`); any step but `TASK RESULT` is a breach | flags: help; output shape: a JetBrains fixture and action; **assumed**: that a signed-in account plays headless without `--auth`, what chat mode does, that no tool step appears in a plain answer |

A tool that answers one message a process may go on with a conversation
by its id only if its sessions can be kept in a store of the seat's own
(under the OS's temp directory, `0700`, removed with the conversation,
the seat and the mind; a mind's start sweeps away stores a killed bridge
left, untouched for an hour) while its login stays where the user has
it; never "the most recent" (`--continue`, `--last`). A resumed process
gets the same lockdown as a new one, and a session the tool cannot find
or read begins the conversation again for that question, with the
prefix, counted as lost. A conversation is over after `idle`, as a
process is.

- **opencode**: resumed. `OPENCODE_DB` (an absolute path) puts its
  sessions in the store; its login (`auth.json`) stays in the data
  directory. It still appends `log/opencode.log` and makes its empty
  directories under the user's data directory, as it does without
  resuming: the log and the login share that directory, and only a key
  in the environment (`OPENCODE_AUTH_CONTENT`) would let it move. Usage is
  per step, so no session file is read.
- **Codex**: not resumed. Sessions (`sessions/`), its SQLite files and
  the login (`auth.json`, or a keyring entry keyed by the `CODEX_HOME`
  path) all live under `CODEX_HOME`; no setting moves the sessions alone.
  A seat home would need the login copied or linked (a link would write
  refreshed credentials through it) or `CODEX_API_KEY`, a key: so each
  question stays a conversation of its own, with `--ephemeral`.
- **Junie**: not resumed. `JUNIE_HOME` moves sessions, logs and settings
  together with the fallback credential file; on Linux its keyring needs
  `DBUS_SESSION_BUS_ADDRESS`, which no CLI is given, so a seat home
  signs it out. And a session it does not find may start a new one
  silently, so a lost conversation could not be told from a kept one.
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
  model's. The key's box is the *name* of its variable, and beside it
  whether that variable is set in the client's own environment: only
  whether; its value is never read into the panel, and the bridge reads
  its own.
- **Faults.** Every refusal of the file stands beside the box it is about,
  in the player's language: `SeatSettings::faults` is the list that
  `parse` and `check` say the first of, so the panel and the bridge refuse
  by one predicate. The panel adds its own (a number that is not one, half
  a price, two profiles with one name). Save is dead while any stands, so
  a key typed or pasted into any box is refused there and never written.
  There is no box for a key. A dollar cap beside a model with no price is
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

## A dev table

`cargo run -p xtask -- dev-table --bridge profile:<name>` seats the bridge
with `--profile <name>` instead of `--mind`, from the same file and under
the same caps (`BAYLEE_SEAT_CONFIG` passes through).

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
