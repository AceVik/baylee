# The two card lanes, and why neither writes its own tests

A *lane* is one cheap model doing one job. Two models, two jobs, four
scripts:

|  | writes cards | writes the engine tests |
|---|---|---|
| **DeepSeek V4.1 Flash** (`deepseek-flash[1m]`, HTTP API) | `cards_deepseek.py` | `tests_deepseek.py` |
| **Gemini 3.8 Flash high** (`agy` CLI, agentic) | `cards_gemini.py` | `tests_gemini.py` |

`lane.py` is the half they share; `prompts/` holds the four contracts, one
per script, and those files are the actual product of this directory — the
Python around them only moves bytes.

## The cross rule

**Whoever wrote the card does not write its test.** DeepSeek's cards go to
`tests_gemini.py` and Gemini's go to `tests_deepseek.py`.

That is not tidiness. A card and its test written by the same model share
that model's misreading, and it has been measured: Mikaeus was written with
two keyword bits no engine rule reads, and the same model's test asserted
that they worked. The pair was internally consistent and wrong, and a green
test said so. Cross-written, a red test is evidence about one of the two
readings instead of about neither.

The first cross round paid for itself on a refusal rather than on a test.
Asked for Teferi's Protection's test, Gemini returned `SKIP` and named the
reason: `finalize_spell` put the resolving instant into the graveyard after
`Effect::ExileSource` had already exiled it, so the card's one expressible
clause was undone one step later. That was true, and it had been sitting
under Spirit Water Revival — a `Coverage::Implemented` card with a test
module of its own — the whole time.

## What each lane costs, which is not the same question twice

**DeepSeek** browses nothing. Everything it would have read is packed into
one cacheable prefix (about 240 KB: the authoring contract, the whole DSL
vocabulary, the pool's shared filters and tokens, four finished cards), so a
batch of thirty pays for it once — on the order of 0.1–0.4M input tokens for
the batch. It is billed per token, and the bill is small. It is also
parallel: `workers` is the second argument.

**Gemini** runs as one agentic session that reads the repository for itself
and is handed the growing transcript on every step, which is a prefix sum:
roughly 110–190M input tokens for the same thirty cards. Under the owner's
subscription **that costs no money** — what it spends is a **time quota that
refreshes**. So the planning unit on that lane is *minutes*, and the token
figure is worth reading only as the reason a session is slow. Both numbers
are measured rather than estimated (`lane.transcript` reads the session's own
sqlite transcript), because an earlier unmeasured fan-out of 412 sessions
spent an estimated 2.2 billion input tokens before anybody noticed.

**Batch size is the only Gemini knob that matters.** Five tests in one
session cost 46 steps each; the same work as a single-card session cost 88.
Almost all of that is orientation the session pays for once, which is why
this lane is one session per batch and why a fan-out of one session per card
is the thing never to build again.

The standing division of labour is in `docs/llm-learnings.md`: DeepSeek is
the volume lane, Gemini the reading lane — its refusals are precise engine
tickets and are worth reading in full, while spot-checks thin out on
DeepSeek's volume first.

## Running one

```bash
W=/tmp/round4                       # anywhere outside the repo
mkdir -p $W

# Cards. The JSON is a list of {"name": …, "path": …}, path repo-relative.
python3 scripts/llm/cards_deepseek.py $W/deepseek.json 5
python3 scripts/llm/cards_gemini.py   $W/gemini.json   50

# Tests, crossed over. The worklist is one repo-relative card path per line.
python3 scripts/llm/tests_gemini.py   $W/for-deepseek.txt 30
python3 scripts/llm/tests_deepseek.py $W/for-gemini.txt    3
```

Everything a run produces lands **beside its worklist**, named after it:
`<stem>-report.md`, `<stem>-sent.md`, `<stem>-answer.md`,
`<stem>-tests/<door>/<slug>.rs`. Put the worklist somewhere scratch and the
whole run goes there with it. The one exception is `cards_gemini.py`, which
cannot write elsewhere: `agy` edits the tree itself.

The DeepSeek key comes from `DEEPSEEK_API_KEY`, or from
`~/.local/share/opencode/auth.json` under `deepseek.key`. It is read into a
variable and never printed.

## What these scripts are allowed to do

Worth reading once, because two of the four are not ordinary scripts.

**`cards_gemini.py` and `tests_gemini.py` run an agent with its permission
prompts turned off**, in the repository root, unattended
(`--dangerously-skip-permissions`). That is deliberate — a session whose job
is thirty card files over ten minutes cannot stop at the first write — and
it means nothing inside the run bounds what the agent touches. What bounds
it is the caller: start from a clean tree, and read the `N von M Dateien
angefasst` line the lane prints, which asks *git* rather than the model.
`BAYLEE_LLM_AGY_PERMISSIONS=ask` drops the flag when you want to watch a new
contract on two cards.

The prompt is assembled here, from a contract in `prompts/` and a worklist
of repo-relative paths — so write the worklist by hand. The session then
reads card files, whose oracle text came from Scryfall; that is ordinary,
but it is the reason a batch is not a place to point at arbitrary content.

**The DeepSeek lanes send one HTTPS request per card and write files.** The
key goes in a header through `urllib`, never into a process argument, which
is what a `curl` call would have done — arguments are world-readable on this
machine for as long as the request lasts.

## Three rules the coordinator keeps

1. **No lane runs cargo.** The build belongs to the coordinator; a model
   that cannot run it must not be the last reader of what it wrote. Two
   lanes fighting over the `target/` lock also costs whole card slots.
2. **No `xtask codegen` while a card batch is in flight.** Codegen rewrites
   every file carrying the stub marker and would clobber the session's work.
3. **The tests are assembled by hand.** Each answer is one card handle plus
   one `#[test]`; the handle goes into `card_tests/mod.rs` (a sibling module
   reaches nothing, so every non-test item lives there) and the test into the
   door its card sits behind. Then it compiles, then it runs, and only then
   is it evidence.

## Feeding a prompt back

Every batch that goes wrong in a new way earns a numbered rule in the
contract that prevents it, and a line in `docs/llm-learnings.md` saying
which error class it came from. The contracts are German because the models
answer these two in German better than in English; the cards and tests they
write are not.
