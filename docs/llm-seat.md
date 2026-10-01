# The language-model seat: settings and spend

The seat bridge (`baylee-seat join`, `crates/baylee-seat`) sits at a table
as an ordinary socket player and hands its questions to a mind: the house,
a script, or a language model. This page is about the language model: the
settings file that says which model plays and what it may spend, and the
spend book that holds a player's caps across games. The types are
`baylee_client_core::llmseat` (pure, every target), the files are
`llmseat::store` and `llmseat::ledger::Book` (native only), and the bridge
applies them in `baylee_seat::config` and `baylee_seat::spend`.

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
| `provider` | `anthropic` or `openai` (any OpenAI-compatible endpoint). Required. |
| `model` | The provider's model id. Required. |
| `effort` | A word such as `low`, `medium`, `high`. |
| `answer` | `tools`, `json` or `json_schema` (the last two for OpenAI-compatible endpoints only). `json` asks the endpoint for a JSON object (`response_format` `json_object`, which `DeepSeek` takes); `json_schema` for one held to the answer's schema (`json_schema`, for an endpoint that refuses a bare object, such as LM Studio). Either way the model is told the answer's fields in its instructions, and when an endpoint turns the one mode down, the error says to try the other. |
| `max_tokens` | The most one reply may take (default 16000 Anthropic, 8000 OpenAI-compatible). |
| `price` | `{"input": …, "output": …}`, US dollars per million tokens: the price of a model this build has none for, or a better one. It applies to the profile's own model only. |
| `game_usd` | The most one game may spend in dollars (default $5); only for a model with a price. |
| `game_tokens` | The most one game may spend in tokens, in and out (default 5,000,000); the limit of a model without a price. |
| `think_secs` | The longest one answer may take (default 60). |
| `key_env` | The environment variable the key is read from (default `ANTHROPIC_API_KEY`, or `BAYLEE_LLM_API_KEY` for `openai`). |
| `base_url` | Where the API is: `https://`, or `http://` on loopback only. |

A model without a price in this build plays only with a `price` or a
`game_tokens`, as `--spend-tokens` on the command line (`join --help`);
where the caps count dollars a day or a month, it also needs the token cap
of that period, or its spend would count nowhere.

## Keys

No key is ever in the file. A profile names the variable its key is read
from, and the bridge reads it from the environment. A field named like a
key (`api_key`, `key`, `token`, `secret`, `password`, `authorization`, …),
a profile's name included, or a name or value shaped like one (`sk-…`)
refuses the file with a sentence saying
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
- `--mind anthropic[:<model>]` or `openai:<model>` puts its model in the
  profile's place and keeps the profile's other settings; a named profile
  of the other provider is refused, the default one is left out with a
  note, and the model plays with the build's defaults.
- Nothing named and no default: the house plays.
- `--effort`, `--answer`, `--max-tokens`, `--price-in`/`--price-out`,
  `--spend-usd`, `--spend-tokens` and `--think-secs` beat the profile's
  field of the same meaning.
- No file: everything is as it was before the file existed. No caps and
  no spend book; a game's budget is $5 (or `--spend-usd`), checked before
  each call.

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
  and `max_tokens`, dollars at the dearest input rate), and a call that
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
- **Boxes.** One per field of the table above and one per cap. An empty
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
