# The language-model seat's protocol: fewer tokens, fewer calls

How the seat bridge (`crates/baylee-seat`) tells a language model a
decision and reads its answer, and why each part has the shape it has.
`docs/llm-seat.md` is the settings and the spend; this page is the
messages. Owner-approved design of 2026-10-06; this page is normative,
and the code cites its sections.

Five rules carry most of it:

1. The state is told as prose, one object a line; the answer is JSON
   held by a schema or a tool (§"The format").
2. The deck is told as the provider reads it: in full where a prompt cache
   serves it back, by name where every token is read again (§"The deck"),
   and the constant head of a conversation is cached for an hour on
   Anthropic's API (§"The cache").
3. A message says nothing a schema or the system prompt already says
   (§"Boilerplate").
4. A later message of a turn tells the board as what changed since the
   turn's last whole board (§"Delta wakes").
5. An answer may carry a plan for the questions after it, a pass `until` a
   point of the game, and how to `react` to an opponent's spell meanwhile;
   the seat runs them without a call and asks the model at anything they
   do not answer exactly (§"Plans").

## Measuring

The numbers on this page come from the seat transcripts of 17 recorded
Commander games (861 model calls; `baylee-play/target/seat-transcripts`,
read-only) and were measured on 2026-10-06 by the scripts kept beside the
design (`measure.py`, `calibrate.py`, `formats.py`, `delta.py`,
`requests.py`, `sim.py`, `sim_impl.py`). The minds in the corpus are
DeepSeek through the OpenAI-compatible path (tools, provider cache),
OpenAI-compatible endpoints without a cache (LM Studio), and `agy`
(Gemini). No Anthropic game is in it: what is said of the Anthropic path
is a projection from the same message sizes at that API's prices.

Tokens are counted with tiktoken `o200k_base`, offline. DeepSeek's own
count is 1.14× o200k on these messages (median of 106 calls, p10–p90
1.09–1.25); the crate's `narrator::estimate_tokens` (characters / 3) is
1.09× o200k. The token budgets the tests hold are in `estimate_tokens`.

Before this design a running wake cost 801 tokens (median; p90 1,219) and
the first of a turn 6,726, because the deck's full text (5.0–6.3k) went
with it: the deck was 34 % of every token sent. Between two decisions of
one turn 91 % of the state lines were unchanged (median). And 60 % of the
750 calls of the ten games with at least 20 calls answered something a
plan or a standing pass could have: the calls were the cost.

## The format

The state stays prose. On 602 captured boards, re-encoding the same state
(header, log and question unchanged) costs, per whole wake:

| encoding of the state | whole wake, median | against prose |
|---|---|---|
| prose, `·`-separated, one object a line (what is sent) | 851 | — |
| abbreviated prose (`[Leg PW-Elspeth] L3`, `#208T`) | 786 | −8 % |
| a line table (`#id \| name \| type \| P/T`) | 815 | −4 % |
| JSON, short keys, no defaults | 1,121 | +32 % |
| YAML, flow style | 1,182 | +39 % |
| YAML, block style | 1,263 | +48 % |

JSON and YAML pay for keys, quotes and braces on every object, and invite
an answer in the same shape with invented keys. The answer stays JSON: it
is held by a schema (`--json-schema`, `StructuredOutput`, the `decide`
tool), it is 35–60 output tokens against thousands of thinking, and its
`ask` refuses a stale answer (`Menu::resolve`). Ids are the handle: 765 of
814 recorded answers used `pick`, and none of the six games that wrote a
summary had an answer refused; a name alone is ambiguous (two Treasures).

## The deck

The deck is told once, in the conversation's constant prefix, in one of
two ways, chosen once a game from how the mind is reached
(`narrator::DeckText`) so the prefix never changes bytes within a game:

- **Full**, where a prompt cache serves the prefix back at a tenth of the
  price: Anthropic, a remote OpenAI-compatible endpoint, an agent CLI.
  Every card's text, once.
- **Names**, where every token of every call is read again: an
  OpenAI-compatible server on loopback (LM Studio, llama.cpp). The prefix
  lists the cards by name (620–664 tokens for a 100-card deck against
  5–6k in full) and a card's text is told under "New cards" the first time
  the card is seen; 21–55 of a deck's cards appear in a recorded game.

Reminder text a model knows by heart is stripped from the prefix and from
"New cards" (`narrator::strip_reminder`): a whole-line parenthesis (a basic
land's mana), one after a bare keyword the narrator names, or after a
sentence ending in one. A keyword's reminder after a cost or a number
(`Suspend 4—{U} (…)`) stays. Measured on the 100-card fixture of three
precons' cards (estimated tokens, 2026-10-06): full prefix 4,685 → 4,508
(−3.8 %; the corpus decks' parentheses were 9–12 % of their prefix, most
of them after a cost or a number, which stay); the names prefix 677.

## The cache

On Anthropic's API the system block and the prefix block (the game, the
seats, the deck) carry `cache_control: {"type": "ephemeral", "ttl":
"1h"}`, and the top-level automatic breakpoint on the growing tail stays
at the default five minutes, after them, as the API requires
(`llm::anthropic::long_cache`). Between two of a seat's turns the other
players act, often past five minutes in a four-player game (the recorded
`agy` games show the same miss: 30.8k and 32.7k uncached after a quiet
stretch), and an expired entry is written again whole.

An hour's write costs 2× the input price, a five minutes' 1.25×. The usage
tells them apart (`cache_creation.ephemeral_1h_input_tokens`, read by
`Usage::of_anthropic` for the API and Claude Code alike), `Price` carries
`cache_write_hour`, and a call's worst case counts the dearer write as its
input rate, so a reservation stays at or above the bill.

Everything before the last breakpoint is byte-stable within a game, in
render order: the tools (`decide`, `concede`) or `--json-schema`,
`SYSTEM` (and `JSON_MODE`, `GAME_DATA` for a CLI), the prefix. The
per-message header (the clock, the stops) comes after it. A `SYSTEM`
edit is a cache miss for every seat: they ship together, in one release.
A CLI's first message starts with the prefix's bytes and nothing variable
ahead of them; its process keeps the conversation, so the prefix goes once
per process, and a process idle five minutes is ended (`docs/llm-seat.md`
§"A CLI as the model"): the tool sets its own cache entries, and the hour's
TTL is the API path's, set by the bridge.

## Boilerplate

- The header is the question, the turn and whose, the step by the name
  stops use, and the seconds left: `DECISION q12 · turn 7, yours · main1 ·
  25 s`. The stops the model set follow it: they are state it chose.
- A question that picks one option does not end with how to answer it
  under a schema or a tool (the schema says it); under plain JSON it does.
  The shapes a question itself carries (piles, blocks, attacks, how many
  ids) are said to every model.
- An empty mana pool is not said.
- Log lines that read the same once written are folded into one with a
  count (`(4 times)`).

The crowded fixture's wake (twelve creatures a side) is 1,043 estimated
tokens, held at 1,150 by `a_wake_holds_its_token_budget`.

## Delta wakes

The first wake of a turn tells the whole board. A later wake of the same
turn and conversation tells it as what changed since the turn's last whole
board (`narrator::delta`):

```text
Board as at q12, except:
You (P1): 17 life · library 45 · hand 3 · graveyard 2
Your battlefield, changed lines (the rest as before):
  lands: Forest #21 (tapped), Forest #22, Mountain #23
  no longer here: Llanowar Elves #30
Stack (top first; the top resolves next):
  #50 Lightning Bolt · your instant spell
Your hand, changed lines (the rest as before):
  no longer here: Lightning Bolt #50
Your graveyard (2): Shock #60, Llanowar Elves #30
```

- The difference is always from one whole board, never from another
  difference: a model that reconciles a chain of differences attacks with
  creatures that died.
- The board is told whole: at the first wake of a turn; in a new
  conversation (a fresh API conversation, a CLI process begun again);
  when the model asked for it with `board: "full"` (for the next message);
  when something of the other side's is on the stack (a decision to read in
  full, not to reconcile); and when more than four lines in ten changed
  (the larger of the lines new and the lines lost, against the board's
  lines now).
- A part of the board (a seat's line, a battlefield, the stack, a hand, a
  zone line, the combat) is told by its heading. Unchanged, it is left out.
  Changed, it is told as its new lines and `no longer here:` with the
  objects it lost, when every line it lost names an object; otherwise, and
  whenever its heading changed (`Stack: empty` to a stack), it is told
  whole. A part that is gone is `<part>: none now.`
- The question's own lines name every object an answer may use, so no id
  has to be recovered from an older board.

Simulated on the recorded games (`sim_impl.py`, 2026-10-06): 450 of 796
later wakes become differences; the median wake is 478 tokens against 762
before (−37 %), and on a no-cache provider the context read per call is
4,603 against 9,617 (−52 %, the names deck included). The design's looser
rule (a difference from the previous message, `sim.py`) predicted 412 and
4,512; the gap is the price of never chaining differences. On the narrator
fixture an unchanged board's second wake costs under 400 estimated tokens,
and a turn with a land tapped, a creature dead and a spell cast 260
against 351 told whole. On a cached API path a wake is in the uncached
tail: about 300 tokens saved at Sonnet 5.5's $2/M is $0.0006 a call. The
win there is §"Plans".

## Plans

An answer may carry three optional fields beside `pick` (`decide`'s
schema, `SYSTEM`'s "PLANS" paragraph with a worked example):

```json
{"ask": "q38", "pick": ["a2"],
 "plan": ["activate #209", "choose Swamp", "cast #141 -> P2", "pass"],
 "until": "my_turn", "react": "targets_me",
 "say": "Fetch a Swamp, then the Shaman."}
```

### `plan`

The steps that answer the questions after this one, in order, one string
each (`narrator::GRAMMAR`, parsed by `narrator::Step::parse`):

| step | answers | read against |
|---|---|---|
| `play #id` | priority | the lands offered |
| `cast #id [-> targets]` | priority, then the targets the cast asks | the spells castable now, or an option whose taps the seat plans; the targets as `then.targets` |
| `activate #id[:n] [-> targets]` | priority | the abilities offered; `:n` the ability's number as the menu prints it, needed when the object offers several |
| `choose <name or #id>[, …]` | cards, piles, a legend, targets, cards to the bottom | the options; a name only when every option it names shares one card (a search shows fresh ids), and each use of a name takes the next one |
| `yes`, `no` | a yes or a no | |
| `color W/U/B/R/G` | a colour | |
| `mode m<n>` | how to cast it | the modes offered |
| `n <number>` | a number | its range |
| `attack none`, `attack #id@P<n> …` | attackers | every pair offered |
| `pass` | priority | |

A step that does not parse refuses the whole answer, with the reason,
before anything is sent, as a bad `pick` does. `then` and the older
`hold: "until_my_turn"` are still read.

The seat (`llm::seatstate::Seat`, shared by the API and the CLI minds)
runs the plan one step a question, without a call, each step read against
that question's own offer exactly as an answer of the model's is
(`Menu::plan_decision`). While a plan has steps, every question of its turn
goes to the mind (`wake::Verdict::Planned`), so no standing order passes a
window the plan wanted. The seat passes priority for a plan only toward
what its next step answers: while nothing but the seat's own spells and
abilities is on the stack (they resolve, and ask what the step answers),
and on its own turn before combat when the next step is the attack;
never in a cleanup step. A plan is bound to the turn it was written in.

It stops, and the question is the model's, at anything it does not answer
exactly; nothing of a plan is ever guessed (`narrator::Stop`):

- the question is not the kind the next step answers;
- the step's id or name is not in the offer, or a name means two cards;
- something of another player's is on the stack;
- a payment is owed;
- the turn it was written in ended;
- the table refused what a step sent (the step is not counted as done);
- the taps of a cast were interrupted by another question;
- the targets named ahead are not legal now;
- the attack step passed without the attack the plan names.

The design also proposed stopping when the seat's life fell; that is left
out, because a fetchland's own life payment would trip it in the very plan
it was meant for, and an opponent's spell that costs life is already a
stop while it is on the stack.

The next message carries the report under its header, in the notes:

```text
Plan q12 ran: activate #209 · passed 1 window · choose Swamp · cast #50 -> P2 · pass.
Plan q12 ran: pass. Stopped at step 2 (pass): P2's Shock #90 is on the stack; the rest of the plan is dropped, and this question is yours.
```

### `until` and `react`

`until`: `my_turn`, `my_main2`, `my_end`, `end_of_turn` (this turn's), or
`none`. While it holds, the seat passes the priority windows the rail
would wake the model in (`wake::WakeFilter`), and declares nothing where
there is nothing to declare, as it always does; it never answers a
question, a declaration with something to declare, a payment owed or a
cleanup window. It ends at its point (`my_main2` written in the second main
phase means the next one), at any wake, and at the model's next answer of
its own, which sets it again or not at all.

`react`, while `until` holds, for something of the other side's on the
stack: `all` (the model is woken; the default), `targets_me` (woken only
when it targets this seat, a permanent it controls or its commander, as
the view's `PublicObject::targets` state; otherwise passed), `none`
(passed). An allowlist of facts the view states, never a list of card
types.

When an `until` passed a window or a wake ended it, the next message says
so once:

```text
until my_turn held through 2 windows; woke because P2's Shock #90 is on the stack.
```

### What the recorded games say

`requests.py` joined each game's bridge transcript with its mind
transcript: 750 calls in the ten games with at least 20.

| what the call answered | calls | share | answered now by |
|---|---|---|---|
| an own-turn pass after the model had acted | 181 | 24.1 % | `until` |
| a pass on their turn over an opponent's spell | 88 | 11.7 % | `react` |
| a pass on their turn at a rail stop | 56 | 7.5 % | `until` |
| a follow-up of the model's own action (a colour, a search, yes or no, targets, a mode, a number) | 120 | 16.0 % | `plan` |
| an empty declaration | 7 | 0.9 % | `until` |
| a real decision | 298 | 39.7 % | the model |

Calls per turn with a call: 5.2 (mean), 2.1 at the ceiling, where every
model plans and opts into `react`. Models used `then.targets` in 33 of 814
answers and `hold` in 44, so 40–50 % of the ceiling (about 2.5 calls a
turn) is what to expect at first; whether a model plans is measured per
model after real games, and the worked example in `SYSTEM` is the first
lever if it is low. The fetchland main phase the owner described (01a10d40
turn 5: play Verdant Catacombs, crack it, find a Swamp, cast Deathrite
Shaman) took four calls; with a plan it takes one, which
`llm::tests::plan::a_fetchland_turn_is_one_call` and
`tests/cli_mind.rs` hold.

## What it saves

Projected per path, the parts multiplied (design, 2026-10-06):

| path | before | after |
|---|---|---|
| no-cache endpoint, tokens per turn | 5.2 calls × 9.6k context = 50k | 2.5 × 4.6k ≈ 12k (−75 %) |
| CLI, one conversation | the conversation grows ≈ 850 tokens + the reply a call | ≈ 480 + the reply, half the calls: about a quarter of the rate |
| cached API, the bill | ≈ 84 % output, 8 % uncached in, 8 % cache | half the calls (output follows calls), −37 % uncached in, no re-writes after a quiet stretch: −45–50 % |
| a fetchland main phase | 4 calls | 1 call |

## Accounting

`agy` reports a process's running totals in each result's `usage`, not the
call's (in every recorded `agy` transcript the numbers grow within a
process and reset with a new one). The seat books the difference from the
process's last reading (`Dialect::usage_is_cumulative`), and a new
process's first reading whole. Claude Code's `usage` is read as the
call's own.
