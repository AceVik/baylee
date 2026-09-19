# Dictionary

Five sessions and one owner work on this repository at the same time. This file
exists so that a word means one thing to all of them.

It is **not** a glossary of Magic, and not a summary of `CLAUDE.md`. An entry
earns its place by being **ambiguous**: a word this repo uses for two different
things, or a word whose everyday meaning is not the meaning here. If a term is
obvious from the code, it does not belong here.

**Who maintains it:** the PM session curates and commits. Any session may
propose an entry by message — say the word, the two things it could mean, and
which one is meant where. An entry that turned out to be wrong is corrected, not
appended to.

---

## The words this repo uses for two different things

These are the ones that have actually caused a misunderstanding, or are one
sentence away from causing one.

### index

Three of them, and none is a row number.

- **`CardIndex`** — the append-only ledger id that names *every card there is*
  (33 694 rows), not this pool. What saved decks and replays store. Spelled as a
  constant: `index::MOX_OPAL`.
- **`SubtypeId`** — a running index into one sorted range partitioned by kind.
  It **renumbers** when a subtype is added, so nothing durable may store it.
- **ability index** — the `index` in `AbilityRef { card, index }`. Reserved ones
  (`SPELL`, `ENTERS`, `choice::GRANTED_ABILITY`, …) count down from `u32::MAX`.

Say which one. "The index" alone is a question, not a fact.

### projection

- **view projection** — the *characteristics* `baylee-view` carries after the
  layer system has run (P/T after anthems, a clone's name). A client cannot run
  layers, so the host projects for it.
- **catalog projection** — the `card_search` table, one row per oracle face,
  rebuilt wholesale by `Catalog::project()`. Nothing to do with the layer
  system.

### ledger

- **the card ledger** — `crates/baylee-cards-index`, one `Row` per card,
  assigns `CardIndex`. Append-only.
- **the token ledger** — `tokenledger::assign`, which gives a token its id in
  `generated_tokens::ALL` and refuses a name two definitions claim.

### lock

- **the cargo lock** — `/Users/viktor/.baylee-locks/with-cargo-lock.sh`, a
  *directory* lock held across all five trees so two heavy cargo runs do not
  fight over the CPU. This is the one that matters to a session.
- **`Cargo.lock`** — the dependency file.
- **cargo's own lock** — what a second `cargo` in the *same* `target/` blocks
  on. Separate trees already avoid this one; they do not avoid the first.

### agent

- **`baylee-agent`** — the process that starts one engine per game and talks to
  the gateway over `/agent/ws`.
- **an agent session** — one of us. A Claude session, or Astra (Codex).

Never write "the agent" in a commit message about a session.

### AI

- **the house AI** — `baylee-ai`, `HeuristicAgent`, what plays a seat.
- **us** — the sessions. Say "session", never "AI", in anything committed.

### gate

- **the rules gate** — `./scripts/gate-rules.sh`: fmt + `cargo lint-rules` +
  `cargo test-rules`, which is the workspace **minus `baylee-client`**. A filter
  for working, never a replacement.
- **the full gate** — the same over the whole workspace, client included. The
  one that runs before a push, because the rules gate cannot see the client.

### tree

Always a **git worktree**, never a data structure. There are five; see
*Territories* below.

---

## Terms of art you are expected to know

### the three card sets

- **the pool** — the 1365 cards this repo compiles (`crates/baylee-cards`).
- **the corpus** — the 33 694 cards the ledger numbers, scanned by
  `baylee-catalog corpus`. A superset of the pool.
- **the reference** — the external, GPL-licensed card-script corpus. Read as an
  automated lookup, **never copied into this repo**. Not vendored, found three
  ways (`--scripts`, `BAYLEE_CARD_SCRIPTS`, a `cardsfolder` near the parent).

A fourth, separate thing: **the catalog** — PostgreSQL card *text* for the
client, optional, unrelated to all three.

### the three ownership states of a card file

- **stub** — `stubgen::STUB_MARKER`. Nobody has finished it. Rewritten on every
  codegen run.
- **machine-owned** — `stubgen::OWNED_MARKER`. A reader wrote it in full.
  Rewritten on every codegen run. **Never patch one**; fix the reader.
- **hand-written** — neither marker. Never touched by codegen.

`xtask adopt --name "<card>"` is the one door from machine-owned to
hand-written, and it is one-way.

### the honest-stub rule

One unread clause and the card is refused. A generated `Coverage::Implemented`
therefore means exactly what a hand-written one means, because the deckbuilder
offers `Implemented` cards as playable. Getting this backwards is worse than
generating nothing.

### first refusal

`transcode-report` lists a script under the **first** thing that refused it. So
closing one cause moves every card that had it to whatever refuses it next. The
cause going to zero measures the rule; the cards finished measures the residue.
They are different numbers and a commit says which one it was aiming at.

### `--stubs`

`transcode-report` ranks the whole reference by default, which measures the
**DSL**. `transcode-report --stubs` ranks only this pool's unfinished cards,
which measures **cards shipped**. The two orders disagree sharply. Rank
corpus-wide to grow the DSL; rank `--stubs` to finish cards.

### two-phase

`codegen` writes some tables from the *compiled* pool, so a card added by one
run gets its row from the next. Run it twice. Only `codegen --check` sees the
gap.

### seat vs. chair

A chair and whoever answers for it are two questions. `SeatKind` is
`Human · Ai · Driven · StandIn`; `answers_over_socket()` is the question almost
everything actually wants. A held chair is **not** relabelled an AI —
`SeatIdentity.away` says so instead.

### Revier (territory)

The crates a session owns. Overlap is **announced, not discovered**. In someone
else's Revier the owner of it is the expert, and a measurement beats seniority
in both directions.

---

## Territories

| Session | Tree | Branch | Owns |
|---|---|---|---|
| PM | `/Users/viktor/Projects/baylee` | `main` | board, integration, this file, `docs/` |
| engine | `…/baylee-engine` | `engine` | `baylee-engine`, `baylee-core`, `baylee-cards`, `-cards-dsl`, `-cards-codegen`, `-cards-index`, `baylee-gamehost`, `baylee-engine-server`, `xtask` |
| client | `…/baylee-client` | `client` | `baylee-client`, `baylee-client-core`, `baylee-client-android`, `scripts/mobile` |
| gateway | `…/baylee-gateway` | `gateway` | `baylee-gateway`, `baylee-agent`, `baylee-protocol`, `baylee-db`, `baylee-catalog` |
| ai | `…/baylee-ai` | `ai` | `baylee-ai`, house-AI scouting, `docs/house-ai.md` |

Two crates belong to nobody and are **announced before they are changed**:

- **`baylee-view`** — `VIEW_VERSION` is asserted in gamehost *and* client tests,
  so a breaking change reaches engine, client and gateway at once.
- **`baylee-build`** — the compile-time build stamp every binary reads.

`scripts/llm/` is card volume and belongs to the engine session, which plans it
in cards (DeepSeek, billed per token) and in minutes (Gemini, a time quota).
**No lane runs cargo**, and no `xtask codegen` runs while a batch is in flight.

---

## German ↔ English

The owner writes German; the repo, the board and every message between sessions
are English. These are the pairs that are not a dictionary lookup.

| Deutsch | English | Note |
|---|---|---|
| Baum | worktree | never "tree" as in data structure |
| Revier | territory | which crates a session owns |
| Gate | gate | untranslated; say *rules* or *full* |
| Karte | card | |
| Kartenmasse | card volume | the cheap LLM lanes, not a session |
| Spur | lane | `scripts/llm/`, one script per (model, job) |
| Briefkasten | mailbox | `/tmp/agentbus`, the Codex bridge |
| Sitzung | session | |
| Zug | turn | one agent turn, not a Magic turn |
| Stapel | stack | the Magic zone |
| Zone | zone | |
| Hand ablegen / Tray | tray | the client's put-down dialog |
| Schranke | bound | as in "a reader that reports a population" |
| Naht | seam | an architectural boundary that drops a capability |
| Messung | measurement | the thing that beats an argument |
| Befund | finding | what a report produces; not necessarily a defect |
| Rückfahrt | return trip | the second half of a timing rule |
| Gegenprobe | counter-test | the run that must fail against the old code |
| Vorspulen | fast-forward | `merge --ff-only` |
| ungetrackt | untracked | never staged: `.junie/`, `gateway-store.json.imported` |

---

## House phrases

Short sentences this team uses as shorthand. Each one is a rule.

- **"Fix the reader, never the card."** One rule wrote hundreds of files.
- **"A measurement beats the lead."** In both directions.
- **"One unread clause and the card is refused."** The honest-stub rule.
- **"Rank the pool, not the corpus."** When the goal is a finished card.
- **"Never restate a default."** Why the card macros exist.
- **"A blocker entry is a sentence, not a subsystem."** Read what the DSL cannot
  *say* before assuming the mechanism is absent.
- **"Overlap is announced, not discovered."** The Revier rule.
- **"Waiting is free."** A blocking lock costs no tokens; a broken gate does.
