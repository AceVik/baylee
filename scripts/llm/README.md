# Native model lanes and independent card tests

> **Retired for cards (owner, 29.09.2026):** the cheap lanes made too many
> errors. Cards are written by Opus, their tests by another model, and Fable
> reviews (`CLAUDE.md` §Cards). These commands retain the historical workflow;
> their existence does not change that authoring policy.

The drivers live in `crates/baylee-tools/src/llm/`. They execute in Rust,
use in-process HTTPS for DeepSeek, and invoke `agy` for the agent lane.
`prompts/` still holds the four authoring contracts. No Python runtime is
used. Build and test commands belong to the coordinator.

| Previous script | Native command | Optional second argument |
| --- | --- | --- |
| `cards_deepseek.py` | `baylee-tools llm cards-deepseek BATCH.json` | workers, default 5 |
| `tests_deepseek.py` | `baylee-tools llm tests-deepseek WORKLIST.txt` | workers, default 5 |
| `cards_gemini.py` | `baylee-tools llm cards-gemini BATCH.json` | minutes, default 60 |
| `tests_gemini.py` | `baylee-tools llm tests-gemini WORKLIST.txt` | minutes, default 45 |
| `lane.py` | Shared native policy, packing, transports and telemetry | see `llm lane --help` |

Use `cargo run --profile tools -p baylee-tools -- llm ...` when the binary is not on PATH.
Underscore aliases, such as `cards_deepseek`, also work.

## The cross rule

**Whoever wrote the card does not write its test.** DeepSeek's cards go to
`tests-gemini`; Gemini's go to `tests-deepseek`. A model can repeat its own
misreading in a test and produce an internally consistent, incorrect pair.
The coordinator assembles the resulting fragments, reviews them, formats,
compiles and runs the tests before treating them as evidence.

## Inputs and outputs

A card batch is a JSON array of `{"name": "...", "path": "..."}` objects.
A test worklist has one repository-relative card path per line; blank lines
are ignored. Only existing `.rs` card files under
`crates/baylee-cards/src/cards/` are admitted. Absolute paths, traversal,
symlinks, duplicate inputs and colliding test filenames are rejected.
A batch contains 1–1000 cards.

Put the worklist **outside the repository**. Outputs land beside it:

- Card reports: `<stem>-report.md`.
- Agent card prompts and answers: `<stem>-sent.md`, `<stem>-answer.md`.
- Test reports: `<stem>-tests-report.md`.
- Agent test prompts and answers: `<stem>-tests-sent.md`,
  `<stem>-tests-answer.md`.
- DeepSeek test fragments: `<stem>-tests/<door>/<slug>.rs`.
- Agent test fragments: `<stem>-tests/<slug>.rs`.

A successful card result updates its listed repository file. A test result
only writes a fragment in scratch. Files are replaced atomically, read back,
and compared with the intended content. Generated card documentation,
identity and face-field lines must survive; tests must parse as a card handle
and one `#[test]` function. Multiple or truncated Rust fences are refused.
A `STUB:` or `SKIP:` response leaves the card unchanged. Provider, policy or
verification failures make the command fail after the per-item report is
written. Agent items with no verified output also make the command fail.
A refusal is not proof that a card or test is implemented.

```bash
mkdir -p /tmp/card-round
cargo run --profile tools -p baylee-tools -- llm cards-deepseek /tmp/card-round/deepseek.json 5
cargo run --profile tools -p baylee-tools -- llm cards-gemini /tmp/card-round/gemini.json 50
cargo run --profile tools -p baylee-tools -- llm tests-gemini /tmp/card-round/for-deepseek.txt 30
cargo run --profile tools -p baylee-tools -- llm tests-deepseek /tmp/card-round/for-gemini.txt 3
```

## Credentials, isolation and limits

DeepSeek reads `DEEPSEEK_API_KEY`, or the `deepseek.key` field in
`~/.local/share/opencode/auth.json`. The key is only placed in an in-process
HTTPS header. Redirects are disabled. API error bodies, auth-file content
and subprocess stderr are suppressed in diagnostics. Known credentials and
recognizable token strings are removed from saved agent answers.

Gemini requires **macOS `sandbox-exec`**, `agy`, `sqlite3` and an existing
`~/.gemini/antigravity-cli/conversations` directory. It copies tracked Rust,
Markdown and TOML source into a disposable scratch tree. No corpus, Git
metadata, untracked files, build outputs or binary assets are copied.
The subprocess can write only its scratch tree, scratch runtime directory,
and agy's conversation directory. The original checkout is inaccessible.
Every changed, deleted or added scratch file is checked against the worklist
before import. Unrelated changes reject the import. Invalid card identity
lines are restored from the pre-run card only if the repaired card verifies.

The agent lane fails closed when isolation or attributable telemetry is
unavailable. Additional CLI cache writes may require configuring `agy` to
use its scratch `TMPDIR`; the driver never grants broad home-directory
writes to work around them. Linux and Windows agent execution is currently
unsupported. The HTTPS lanes and import parsers have no macOS dependency.

`BAYLEE_LLM_AGY_MODEL` selects the agent model (default
`gemini-3.8-flash-high`). `BAYLEE_LLM_AGY_PERMISSIONS=ask` omits the historical
`--dangerously-skip-permissions` switch; the default, or `skip`, includes it
inside the filesystem sandbox. Both modes require sandbox isolation.

| Setting | Default | Allowed range |
| --- | --- | --- |
| workers positional argument | 5 | 1–32 |
| agent minutes positional argument | 60 cards / 45 tests | 1–240 |
| `DS_MAX_TOKENS` per answer | 64000 | 1–131072 |
| `BAYLEE_LLM_REQUEST_SECONDS` | 900 | 1–3600 |
| `BAYLEE_LLM_BATCH_SECONDS` | 3600 | 1–14400 |
| `BAYLEE_LLM_BATCH_MAX_TOKENS` HTTP admission budget | 32000000 | 1–1000000000 |
| `BAYLEE_LLM_MAX_INPUT_TOKENS` | 1000000 HTTP / 250000000 agent | 1–1000000000 |

The HTTP lane uses prompt bytes as a conservative token admission bound and
checks reported input/output token usage. Each HTTP item reserves its prompt-byte\nbound plus its maximum output tokens from the shared batch budget before\nsending; failed requests keep their reservation. Agent input tokens are estimated
from the transcript prefix sum divided by four, as in the historical driver;
this is an estimate, not a billing guarantee. Telemetry is checked during
execution and on completion. Quota refusals, missing transcripts and multiple
new sessions are errors. Deadline termination kills the subprocess group.
Inputs are bounded to 4 MiB, subprocess/API output to 16 MiB, scratch source
trees to 512 MiB, and packed requests to 4 MiB.

A repository lock `.baylee-llm.lock` prevents overlapping native lanes; a
scratch lock `.<stem>-llm.lock` protects each output batch. The lock records
the owner PID. A stale lock is never removed automatically. Other editors
and codegen do not participate in these locks, so the coordinator must still
keep codegen and unrelated card writes out of a running batch. Card content
is compared with its pre-run snapshot before each import.

Offline inspection commands:

```bash
cargo run --profile tools -p baylee-tools -- llm lane door crates/baylee-cards/src/cards/lands/basic/forest.rs
cargo run --profile tools -p baylee-tools -- llm lane check-header /tmp/stub.rs /tmp/proposed.rs
cargo run --profile tools -p baylee-tools -- llm lane transcript /path/to/session.db
```

## Coordinator checks

Never run codegen while a card batch is in flight. Assemble test handles in
`card_tests/mod.rs` and tests in the module matching their card's door.
Run formatting before compiling, then review and execute the tests.
Every new contract failure earns a prompt rule and a note in
`docs/llm-learnings.md`. No card batch is run by the migration itself.
